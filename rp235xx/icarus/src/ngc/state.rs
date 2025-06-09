use nalgebra::{Matrix3, Matrix3x1, Quaternion, Rotation3, UnitQuaternion};
use serde::Deserialize;

#[derive(Clone, Debug, Default)]
pub struct AssetState{
    pub ecef_state_original: State,
    pub body_state_current: State,
    pub body_state_previous: State,
    pub ll_state_current: State,
    pub ll_state_previous: State,
    pub eci_state_current: State,
    pub eci_state_previous: State,
    pub ecef_state_current: State,
    pub trs_to_crs_dcm: Matrix3<f64>,
    pub crs_to_trs_dcm: Matrix3<f64>
}
impl AssetState{
    pub fn set_current_time(&mut self, time: &f64){
        self.body_state_previous.time = time.clone();
        self.body_state_current.time = time.clone();
        self.ll_state_previous.time = time.clone();
        self.ll_state_current.time = time.clone();
        self.eci_state_previous.time = time.clone();
        self.eci_state_current.time = time.clone();
        self.ecef_state_current.time = time.clone();
    }
}

#[derive(Clone, Debug)]
pub struct IMUState{
    pub time:               u64,
    pub lin_acc:            Matrix3x1<f64>,
    pub ang_vel:            Matrix3x1<f64>,
}

impl Default for IMUState{
    fn default()-> IMUState{
        IMUState{
            time:               0_u64,
            lin_acc:            Matrix3x1::zeros(),
            ang_vel:            Matrix3x1::zeros(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct State{
    pub time:               f32,
    pub lin_acc:            Matrix3x1<f64>,
    pub lin_vel:            Matrix3x1<f64>,
    pub lin_pos:            Matrix3x1<f64>,
    pub ang_vel:            Matrix3x1<f64>,
    pub ang_pos:            Matrix3x1<f64>,
    pub quaternion:         Quaternion<f64>
}

impl Default for State{
    fn default()-> State{
        State{
            time:               0.0_f64,
            lin_acc:            Matrix3x1::zeros(),
            lin_vel:            Matrix3x1::zeros(),
            lin_pos:            Matrix3x1::zeros(),
            ang_vel:            Matrix3x1::zeros(),
            ang_pos:            Matrix3x1::zeros(),
            quaternion:         Quaternion::identity(),
        }
    }
}


impl State{
    pub fn set_output_rate(&mut self, output_rate: f64){
        self.output_rate = output_rate;
    }

    fn integrate(&self, dt: f64, current: Matrix3x1<f64>, last: Matrix3x1<f64>)->Matrix3x1<f64>{
        // Trapezoidal for now )))):
        return 0.5 * dt * (current + last);
    }

    pub fn integrate_linear(&mut self, state_last: &State){
        let dt = self.time - state_last.time;
        self.lin_vel += self.integrate(dt, self.lin_acc, state_last.lin_acc);
        self.lin_pos += self.integrate(dt, self.lin_vel, state_last.lin_vel);
    }

    pub fn integrate_angular(&mut self, state_last: &State){
        let dt = self.time - state_last.time;
        // self.ang_vel += self.integrate(dt, self.ang_acc, state_last.ang_acc);
        // self.ang_pos += self.integrate(dt, self.ang_vel, state_last.ang_vel);

        // Quaternion Angular Integration from Angular Velocity
        let ang_acc_quat = Quaternion::from_parts(1.0, self.ang_acc);
        let ang_vel_quat = Quaternion::from_parts(1.0, self.ang_vel);

        // let mut q = UnitQuaternion::identity();
        // if self.quaternion != Quaternion::identity(){
        //     q = UnitQuaternion::from_quaternion(self.quaternion);
        // }
        // else{
        //     q = UnitQuaternion::identity();
        // }
        
        let q_quat = q.quaternion();

        // Quaternion Integration
        let q_dot = 0.5 * q_quat * ang_vel_quat;
        let q_ddot = 0.25 * q_quat * ang_vel_quat * ang_vel_quat + 0.5 * q_quat * ang_acc_quat;
        // let q_dddot = 1.0 / 6.0 * q_quat * ang_vel_quat * ang_vel_quat * ang_vel_quat + 0.25 * q_quat * ang_acc_quat * ang_vel_quat + 0.5 * q_quat * ang_vel_quat * ang_acc_quat;
 
        let new_quaternion = q_quat + q_dot * dt + 1.0 / 4.0 * q_ddot * dt.powf(2.0); //+ 1.0 / 9.0 * q_dddot * dt.powf(3.0);
        // self.quaternion = new_quaternion.normalize();

        let unit_quaternion = UnitQuaternion::from_quaternion(self.quaternion);
        let euler_angles = unit_quaternion.euler_angles();
        self.ang_pos = Matrix3x1::new(euler_angles.0, euler_angles.1, euler_angles.2);

    }

    pub fn transform_state(&self, rotation_matrix: &Matrix3<f64>)->State{
        let state_from = self; // Kinda dumb but I would like to keep the from -> to naming convention here...
        let force_state_to = rotation_matrix * state_from.force;
        let lin_acc_state_to = rotation_matrix * state_from.lin_acc; 
        let lin_vel_state_to = rotation_matrix * state_from.lin_vel; 
        let lin_pos_state_to = rotation_matrix * state_from.lin_pos; 

        let torque_state_to = rotation_matrix * state_from.torque;
        let ang_acc_state_to = rotation_matrix * state_from.ang_acc;
        let ang_vel_state_to = rotation_matrix * state_from.ang_vel;
        let ang_pos_state_to = rotation_matrix * state_from.ang_pos;

        let rotation = Rotation3::from_matrix(rotation_matrix);

        let rotation_quaternion = UnitQuaternion::from_rotation_matrix(&rotation);
        let quaternion_to = rotation_quaternion.quaternion() * state_from.quaternion * rotation_quaternion.quaternion().try_inverse().expect("Couldn't inverse rotation quaternion.");

        State { 
                time_last_output: self.time_last_output,
                output_rate: self.output_rate,
                time: self.time, 
                force:    force_state_to, 
                lin_acc:  lin_acc_state_to, 
                lin_vel:  lin_vel_state_to, 
                lin_pos:  lin_pos_state_to, 
                torque:   torque_state_to, 
                ang_acc:  ang_acc_state_to, 
                ang_vel:  ang_vel_state_to, 
                ang_pos:  ang_pos_state_to, 
                quaternion: quaternion_to
            }
    }

    pub fn integrate_from(&mut self, state_last: &State){
        self.integrate_linear(state_last);
        self.integrate_angular(state_last);
    }
}