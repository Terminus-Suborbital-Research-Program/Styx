#![cfg(all(target_arch = "arm", target_os = "none"))]

use aether_core::{
    coordinate::Cartesian,
    math::{Matrix, Vector},
    real::Real,
    reference_frame::Body,
};
use aether_gnc::{
    control::{ErrorMap, LinearModel, MPC},
    MomentumWheel, MomentumWheelInput, MomentumWheelMixer3, MomentumWheelState3,
    navigation::ExtendedKalmanFilter,
    sensors::{Accel, Gyro, IMU, NavigatorSample},
};
use aether_time::time::{GPS, Time};

use elara::{GncTelemetry, TelemetryFrame};

use core::f32::consts::{PI};

const GNC_STATE_LEN: usize = 6;
const GNC_INPUT_LEN: usize = 3;
const GNC_HORIZON: usize = 120;
const ATTITUDE_TRACKING_WEIGHT: f32 = 4.0;
const RATE_TRACKING_WEIGHT: f32 = 2.0;
const ATTITUDE_TERMINAL_WEIGHT: f32 = 1.0;
const RATE_TERMINAL_WEIGHT: f32 = 90.0;
const CONTROL_EFFORT_WEIGHT: f32 = 0.02;
const MOMENTUM_WHEEL_INERTIA_KG_M2: f32 = 0.0001225;
const MOMENTUM_WHEEL_MAX_TORQUE_COMMAND_NM: f32 = 0.25;
const MOMENTUM_WHEEL_MAX_RATE_COMMAND_RAD_S: f32 = 2618.0;
const MOTOR_TRIM_MAX_PULSE_OFFSET_US: f32 = 0.0;
const MANEUVER_ATTITUDE_SETTLE_TOLERANCE_RAD: f32 = 2.0 * PI / 180.0;
const MANEUVER_RATE_SETTLE_TOLERANCE_RAD_S: f32 = 0.25 * PI / 180.0;

include!(concat!(env!("OUT_DIR"), "/spacecraft_config.rs"));

#[derive(Clone, Copy)]
struct SatelliteImu;

impl Accel<f32> for SatelliteImu {}
impl Gyro<f32> for SatelliteImu {}

#[derive(Clone, Copy)]
struct BodyRateModel {
    dt: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct SatelliteTrackingWeights;

impl ErrorMap<f32, GNC_STATE_LEN, GNC_STATE_LEN, Body<f32>> for SatelliteTrackingWeights {
    fn c_body(&self, _frame: &Body<f32>) -> Matrix<f32, GNC_STATE_LEN, GNC_STATE_LEN> {
        Matrix::identity()
    }

    fn w(&self, _frame: &Body<f32>) -> Matrix<f32, GNC_STATE_LEN, GNC_STATE_LEN> {
        Matrix::diag(&[
            ATTITUDE_TRACKING_WEIGHT,
            ATTITUDE_TRACKING_WEIGHT,
            ATTITUDE_TRACKING_WEIGHT,
            RATE_TRACKING_WEIGHT,
            RATE_TRACKING_WEIGHT,
            RATE_TRACKING_WEIGHT,
        ])
    }

    fn w_terminal(&self, _frame: &Body<f32>) -> Matrix<f32, GNC_STATE_LEN, GNC_STATE_LEN> {
        Matrix::diag(&[
            ATTITUDE_TERMINAL_WEIGHT,
            ATTITUDE_TERMINAL_WEIGHT,
            ATTITUDE_TERMINAL_WEIGHT,
            RATE_TERMINAL_WEIGHT,
            RATE_TERMINAL_WEIGHT,
            RATE_TERMINAL_WEIGHT,
        ])
    }
}

impl LinearModel<f32, GNC_STATE_LEN, GNC_INPUT_LEN, Body<f32>> for BodyRateModel {
    fn a(&self, _frame: &Body<f32>) -> Matrix<f32, GNC_STATE_LEN, GNC_STATE_LEN> {
        Matrix::new([
            [1.0, 0.0, 0.0, self.dt, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0, self.dt, 0.0],
            [0.0, 0.0, 1.0, 0.0, 0.0, self.dt],
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn b(&self, _frame: &Body<f32>) -> Matrix<f32, GNC_STATE_LEN, GNC_INPUT_LEN> {
        Matrix::new([
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [self.dt / SPACECRAFT_INERTIA_DIAGONAL_KG_M2[0], 0.0, 0.0],
            [0.0, self.dt / SPACECRAFT_INERTIA_DIAGONAL_KG_M2[1], 0.0],
            [0.0, 0.0, self.dt / SPACECRAFT_INERTIA_DIAGONAL_KG_M2[2]],
        ])
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct GncOutput {
    pub(super) telemetry: GncTelemetry,
    pub(super) motor_trim_commands: [i16; 3],
}

pub(super) struct GncController {
    imu: SatelliteImu,
    ekf: ExtendedKalmanFilter<f32, GNC_STATE_LEN>,
    mpc: MPC<f32, GNC_STATE_LEN, GNC_INPUT_LEN, GNC_HORIZON, GNC_STATE_LEN, Body<f32>, SatelliteTrackingWeights>,
    model: BodyRateModel,
    wheel_mixer: MomentumWheelMixer3<f32>,
    wheel_state: MomentumWheelState3<f32>,
    last_control: Vector<f32, GNC_INPUT_LEN>,
    maneuver_stage: usize,
    maneuver_settle_started_ms: Option<u32>,
    reference_attitude: Option<[f32; 3]>,
}

impl GncController {
    pub(super) fn new(dt: f32) -> Self {
        let frame = Body::<f32>::default();
        let mut mpc = MPC::new(
            Matrix::<f32, GNC_INPUT_LEN, GNC_INPUT_LEN>::diag(&[
                CONTROL_EFFORT_WEIGHT,
                CONTROL_EFFORT_WEIGHT,
                CONTROL_EFFORT_WEIGHT,
            ]),
            frame,
            SatelliteTrackingWeights,
        );
        mpc.set_reference(Vector::new([0.0; GNC_STATE_LEN]));

        Self {
            imu: SatelliteImu,
            ekf: ExtendedKalmanFilter::new(
                Vector::new([0.0; GNC_STATE_LEN]),
                Matrix::diag(&[0.2, 0.2, 0.2, 0.3, 0.3, 0.3]),
            ),
            mpc,
            model: BodyRateModel { dt },
            wheel_mixer: build_momentum_wheel_mixer(),
            wheel_state: MomentumWheelState3::default(),
            last_control: Vector::new([0.0; GNC_INPUT_LEN]),
            maneuver_stage: 0,
            maneuver_settle_started_ms: None,
            reference_attitude: None,
        }
    }

    pub(super) fn update(&mut self, frame: &TelemetryFrame) -> GncOutput {
        rtt_target::rprintln!("GNC Update");
        let timestamp = Time::<GPS>::new(frame.snapshot.seq as i64, 0);
        let accel_sample = self.imu.accelerometer_sample(
            frame.snapshot.accel_x_g as f32,
            frame.snapshot.accel_y_g as f32,
            frame.snapshot.accel_z_g as f32,
        );
        let gyro_sample = IMU::body_rates(
            &self.imu,
            frame.snapshot.gyro_x_dps.to_radians() as f32,
            frame.snapshot.gyro_y_dps.to_radians() as f32,
            frame.snapshot.gyro_z_dps.to_radians() as f32,
        );

        let _accel_nav: NavigatorSample<GPS, _, Body<f32>, Body<f32>> =
            NavigatorSample::new(timestamp, accel_sample);
        let _gyro_nav: NavigatorSample<GPS, _, Body<f32>, Body<f32>> =
            NavigatorSample::new(timestamp, gyro_sample);

        let (roll_rad, pitch_rad) = self.attitude_from_imu(accel_sample);
        let yaw_rad = self.yaw_from_gyro(frame.snapshot.seq, gyro_sample.z());
        let measurement = Vector::new([
            roll_rad,
            pitch_rad,
            yaw_rad,
            gyro_sample.x(),
            gyro_sample.y(),
            gyro_sample.z(),
        ]);

        let model = self.model;
        let last_control = self.last_control;

        self.ekf.predict_with_control(
            |state, control| process_model(model, state, control),
            |_, _| model.a(&Body::<f32>::default()),
            &last_control,
            Matrix::diag(&[1.0e-3, 1.0e-3, 1.0e-3, 5.0e-3, 5.0e-3, 5.0e-3]),
        );

        let _ = self.ekf.update(
            &measurement,
            |state| *state,
            |_| Matrix::<f32, GNC_STATE_LEN, GNC_STATE_LEN>::identity(),
            Matrix::diag(&[5.0e-2, 5.0e-2, 2.0e-1, 1.0e-2, 1.0e-2, 1.0e-2]),
        );

        let (desired_attitude, desired_rates) = self.desired_reference(frame.snapshot.timestamp_ms);
        self.mpc.set_reference(Vector::new([
            desired_attitude[0],
            desired_attitude[1],
            desired_attitude[2],
            desired_rates[0],
            desired_rates[1],
            desired_rates[2],
        ]));
        self.mpc.solve(&self.model);

        let desired_body_torque = self.mpc.control(self.ekf.state);
        let wheel_output = self.wheel_mixer.mix_torque_to_rate_commands(
            &mut self.wheel_state,
            MomentumWheelInput {
                desired_torque_body_nm: desired_body_torque,
                dt_s: self.model.dt,
            },
        );
        self.last_control = wheel_output.achieved_torque_body_nm;

        GncOutput {
            telemetry: GncTelemetry {
                maneuver_stage: self.maneuver_stage as u8,
                curr_roll: self.ekf.state[0] as f32,
                curr_pitch: self.ekf.state[1] as f32,
                curr_yaw: self.ekf.state[2] as f32,
                curr_rate_x: self.ekf.state[3] as f32,
                curr_rate_y: self.ekf.state[4] as f32,
                curr_rate_z: self.ekf.state[5] as f32,
                desired_roll: desired_attitude[0] as f32,
                desired_pitch: desired_attitude[1] as f32,
                desired_yaw: desired_attitude[2] as f32,
                desired_rate_x: desired_rates[0] as f32,
                desired_rate_y: desired_rates[1] as f32,
                desired_rate_z: desired_rates[2] as f32,
                torque_x: self.last_control[0] as f32,
                torque_y: self.last_control[1] as f32,
                torque_z: self.last_control[2] as f32,
            },
            motor_trim_commands: [
                scale_wheel_accel_to_motor_trim(wheel_output.wheel_accel_command_rad_s2[0]),
                scale_wheel_accel_to_motor_trim(wheel_output.wheel_accel_command_rad_s2[1]),
                scale_wheel_accel_to_motor_trim(wheel_output.wheel_accel_command_rad_s2[2]),
            ],
        }
    }
    fn attitude_from_imu(&self, accel_sample: Cartesian<f32, Body<f32>>) -> (f32, f32) {
        let up = self
            .imu
            .up_from_sample(accel_sample)
            .unwrap_or_else(|| Cartesian::new(0.0, 0.0, 1.0));
        let roll = up.y().atan2(up.z());
        let pitch = (-up.x()).atan2((up.y() * up.y() + up.z() * up.z()).sqrt());
        (roll, pitch)
    }

    fn yaw_from_gyro(&self, seq: u32, yaw_rate_rad_s: f32) -> f32 {
        if seq <= 1 {
            0.0
        } else {
            self.ekf.state[2] + self.model.dt * yaw_rate_rad_s
        }
    }

    fn desired_reference(&mut self, timestamp_ms: u32) -> ([f32; 3], [f32; 3]) {
        let current_attitude = [self.ekf.state[0], self.ekf.state[1], self.ekf.state[2]];

        if self.reference_attitude.is_none() {
            self.reference_attitude = Some(current_attitude);
        }

        self.advance_maneuver_stage(timestamp_ms);

        if self.maneuver_stage >= 4 {
            let desired_rates = Self::clamp_body_rates([
                SPACECRAFT_MANEUVER_FINAL_ROLL_RATE_RAD_S as f32,
                0.0,
                0.0,
            ]);
            let desired_attitude = Self::advance_rate_reference(
                self.reference_attitude.unwrap_or(current_attitude),
                desired_rates,
                self.model.dt,
            );
            self.reference_attitude = Some(desired_attitude);
            return (desired_attitude, desired_rates);
        }

        let target_attitude = Self::maneuver_attitude_target(self.maneuver_stage as u32);

        let (desired_attitude, slewed_rates) = Self::slew_attitude_reference(
            self.reference_attitude.unwrap_or(current_attitude),
            target_attitude,
            self.model.dt,
        );
        self.reference_attitude = Some(desired_attitude);

        (desired_attitude, Self::clamp_body_rates(slewed_rates))
    }

    fn advance_maneuver_stage(&mut self, timestamp_ms: u32) {
        if self.maneuver_stage >= 4 {
            return;
        }

        let current_attitude = [self.ekf.state[0], self.ekf.state[1], self.ekf.state[2]];
        let current_rates = [self.ekf.state[3], self.ekf.state[4], self.ekf.state[5]];
        let target_attitude = Self::maneuver_attitude_target(self.maneuver_stage as u32);

        if !Self::attitude_stage_is_settled(current_attitude, current_rates, target_attitude) {
            self.maneuver_settle_started_ms = None;
            return;
        }

        let settle_started_ms = self.maneuver_settle_started_ms.get_or_insert(timestamp_ms);
        let settle_elapsed_ms = timestamp_ms.wrapping_sub(*settle_started_ms);
        if settle_elapsed_ms < Self::maneuver_segment_duration_ms() {
            return;
        }

        self.maneuver_stage += 1;
        self.maneuver_settle_started_ms = None;
    }

    fn maneuver_attitude_target(step: u32) -> [f32; 3] {
        let target_index = step.min(3) as usize;
        [
            SPACECRAFT_MANEUVER_ATTITUDE_TARGETS_RAD[target_index][0] as f32,
            SPACECRAFT_MANEUVER_ATTITUDE_TARGETS_RAD[target_index][1] as f32,
            SPACECRAFT_MANEUVER_ATTITUDE_TARGETS_RAD[target_index][2] as f32,
        ]
    }

    fn maneuver_segment_duration_ms() -> u32 {
        (SPACECRAFT_MANEUVER_SEGMENT_DURATION_S * 1000.0).round() as u32
    }

    fn attitude_stage_is_settled(
        current_attitude: [f32; 3],
        current_rates: [f32; 3],
        target_attitude: [f32; 3],
    ) -> bool {
        let attitude_is_settled = (0..3).all(|axis| {
            Self::wrap_angle(target_attitude[axis] - current_attitude[axis]).abs()
                <= MANEUVER_ATTITUDE_SETTLE_TOLERANCE_RAD
        });
        let rates_are_settled = current_rates
            .into_iter()
            .all(|rate| rate.abs() <= MANEUVER_RATE_SETTLE_TOLERANCE_RAD_S);

        attitude_is_settled && rates_are_settled
    }

    fn advance_rate_reference(
        current_reference: [f32; 3],
        desired_rates: [f32; 3],
        dt: f32,
    ) -> [f32; 3] {
        if dt <= 0.0 {
            return current_reference;
        }

        let mut next_reference = current_reference;
        for axis in 0..3 {
            next_reference[axis] = Self::wrap_angle(current_reference[axis] + desired_rates[axis] * dt);
        }
        next_reference
    }

    fn clamp_body_rates(rates: [f32; 3]) -> [f32; 3] {
        [
            rates[0].clamp(-MAX_DESIRED_BODY_RATE_RAD_S[0], MAX_DESIRED_BODY_RATE_RAD_S[0]),
            rates[1].clamp(-MAX_DESIRED_BODY_RATE_RAD_S[1], MAX_DESIRED_BODY_RATE_RAD_S[1]),
            rates[2].clamp(-MAX_DESIRED_BODY_RATE_RAD_S[2], MAX_DESIRED_BODY_RATE_RAD_S[2]),
        ]
    }

    fn slew_attitude_reference(
        current_reference: [f32; 3],
        target_attitude: [f32; 3],
        dt: f32,
    ) -> ([f32; 3], [f32; 3]) {
        if dt <= 0.0 {
            return (current_reference, [0.0, 0.0, 0.0]);
        }

        let mut next_reference = current_reference;
        let mut desired_rates = [0.0; 3];
        for axis in 0..3 {
            let error = Self::wrap_angle(target_attitude[axis] - current_reference[axis]);
            let max_delta = MAX_DESIRED_BODY_RATE_RAD_S[axis] * dt;
            let delta = error.clamp(-max_delta, max_delta);
            next_reference[axis] = current_reference[axis] + delta;
            desired_rates[axis] = delta / dt;
        }

        (next_reference, desired_rates)
    }

    fn wrap_angle(angle: f32) -> f32 {
        let two_pi = 2.0 * PI;
        let mut wrapped = angle;
        while wrapped > PI {
            wrapped -= two_pi;
        }
        while wrapped < -PI {
            wrapped += two_pi;
        }
        wrapped
    }
}

fn build_momentum_wheel_mixer() -> MomentumWheelMixer3<f32> {
    MomentumWheelMixer3 {
        wheels: core::array::from_fn(|axis| MomentumWheel {
            axis_body: Vector::new(SPACECRAFT_MOMENTUM_WHEEL_AXES_BODY_XYZ[axis]),
            wheel_inertia_kg_m2: MOMENTUM_WHEEL_INERTIA_KG_M2,
            min_rate_command_rad_s: -MOMENTUM_WHEEL_MAX_RATE_COMMAND_RAD_S,
            max_rate_command_rad_s: MOMENTUM_WHEEL_MAX_RATE_COMMAND_RAD_S,
            max_accel_command_rad_s2: MOMENTUM_WHEEL_MAX_RATE_COMMAND_RAD_S,
        }),
    }
}

fn scale_wheel_accel_to_motor_trim(value: f32) -> i16 {
    if MOMENTUM_WHEEL_MAX_TORQUE_COMMAND_NM <= 0.0 {
        return 0;
    }

    let wheel_torque_nm = value * MOMENTUM_WHEEL_INERTIA_KG_M2;
    ((wheel_torque_nm / MOMENTUM_WHEEL_MAX_TORQUE_COMMAND_NM) * MOTOR_TRIM_MAX_PULSE_OFFSET_US)
        .round()
        .clamp(-MOTOR_TRIM_MAX_PULSE_OFFSET_US, MOTOR_TRIM_MAX_PULSE_OFFSET_US) as i16
}

fn process_model(
    model: BodyRateModel,
    state: &Vector<f32, GNC_STATE_LEN>,
    control: &Vector<f32, GNC_INPUT_LEN>,
) -> Vector<f32, GNC_STATE_LEN> {
    let a = model.a(&Body::<f32>::default());
    let b = model.b(&Body::<f32>::default());
    a * *state + b * *control
}