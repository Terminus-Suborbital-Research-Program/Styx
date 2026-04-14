use nalgebra::{Matrix3, Matrix3x1};
use wgs84::transforms;
use crate::reference::matrices;
use crate::reference::standards;
use crate::reference::constants::{*};
use crate::reference::matrices::*;
use hifitime::Epoch;
use crate::interface::transforms::angular_rate_dcm;
use united_states_standard_atmosphere::ussa;

#[derive(Clone, Debug)]
pub struct Earth{
    pub mass: f64,
    pub radius: f64,
    pub rotational_velocity: Matrix3x1<f64>,
    pub atmosphere: ussa::USSA
}

impl Default for Earth{
    fn default() -> Self{
        Earth{
            mass: 0.0,
            radius: 0.0,
            rotational_velocity: Matrix3x1::new(0.0,0.0,7.292115146706979e-5),
            atmosphere: ussa::USSA::new()
        }
    }
}

impl Earth{
    pub fn new()->Self{
        Earth{
            mass: standards::iers::geocentric_gravitational_constant / standards::iers::gravitational_constant,
            radius: standards::iers::earth_equatorial_radius,
            rotational_velocity: Matrix3x1::new(0.0,0.0,7.292115146706979e-5),
            atmosphere: ussa::USSA::new()
        }
    }

    pub fn solve_euler_force(&self, position_from_center: Matrix3x1<f64>, angular_acceleration: Matrix3x1<f64>, mass: f64) -> Matrix3x1<f64>{
        let euler_force = mass * angular_acceleration.cross(&position_from_center);   
        return euler_force
    }

    pub fn solve_coriolis_force(&self, velocity_from_center: Matrix3x1<f64>, angular_velocity: Matrix3x1<f64>, mass: f64) -> Matrix3x1<f64>{
        let coriolis_force = 2.0 * mass * angular_velocity.cross(&velocity_from_center);
        return coriolis_force
    }

    pub fn solve_centrifugal_force(&self, position_from_center: Matrix3x1<f64>, angular_velocity: Matrix3x1<f64>, mass: f64) -> Matrix3x1<f64>{
        let centrifugal_force = mass * angular_velocity.cross(&(angular_velocity.cross(&position_from_center)));
        return centrifugal_force
    }
    
    pub fn solve_gravitational_force(&self, position_from_center: Matrix3x1<f64>, mass: f64) -> Matrix3x1<f64>{
        let gravitational_force = wgs84::gravity::gravity::gravity_rectangular(position_from_center[0],position_from_center[1],position_from_center[2]) * mass;
        return gravitational_force;
    }
    pub fn solve_gravitational_torque(&self, position_from_center: Matrix3x1<f64>, inertia_matrix: Matrix3<f64>)->Matrix3x1<f64>{
        let value = inertia_matrix * position_from_center;
        let local = position_from_center.cross(&value);
        let scalar = (3.0 * standards::iers::geocentric_gravitational_constant) / (position_from_center.magnitude().powf(5.0));
        let gravitational_torque =  scalar * local;
        return gravitational_torque;
    }
    pub fn geocentric_to_ecef(&self, latitude: f64, longitude: f64, altitude: f64)->Matrix3x1<f64>{
        return wgs84::transforms::transforms::geocentric_to_ecef(latitude, longitude, altitude);
    }

    pub fn ecef_to_geocentric_ferrari(&self, x: f64, y: f64, z: f64) -> Matrix3x1<f64>{
        return wgs84::transforms::transforms::ecef_to_geocentric_ferrari(x, y, z);
    }

    pub fn ecef_to_geocentric(&self, x: f64, y: f64, z: f64) -> Matrix3x1<f64> {
        return wgs84::transforms::transforms::ecef_to_geocentric(x, y, z);
    }

    pub fn eci_to_ecef(&self, time: f64)->Matrix3<f64>{
        return Matrix3::new(
            (self.rotational_velocity.z*time).cos(), (self.rotational_velocity.z*time).sin(), 0.0,
            (-self.rotational_velocity.z*time).sin(), (self.rotational_velocity.z*time).cos(), 0.0,
            0.0 ,0.0 ,1.0
        );
    }
    pub fn ecef_to_eci(&self, time: f64)->Matrix3<f64>{
        return Matrix3::new(
            (self.rotational_velocity.z*time).cos(), (-self.rotational_velocity.z*time).sin(), 0.0,
            (self.rotational_velocity.z*time).sin(), (self.rotational_velocity.z*time).cos(), 0.0,
            0.0 ,0.0 ,1.0
        );
    }

    pub fn get_temperature(&self, geometric_height: f64)->Result<f64, &'static str>{
        return Ok(self.atmosphere.temperature(geometric_height).expect("Could not get temperature."));
    }
    pub fn get_pressure(&self, geometric_height: f64)->Result<f64, &'static str>{
        return Ok(self.atmosphere.pressure(geometric_height).expect("Could not get pressure."));
    }
    pub fn get_density(&self, geometric_height: f64)->Result<f64, &'static str>{
        return Ok(self.atmosphere.density(geometric_height).expect("Could not get density."));
    }
    pub fn get_speed_of_sound(&self, geometric_height: f64)->Result<f64, &'static str>{
        return Ok(self.atmosphere.speed_of_sound(geometric_height).expect("Could not get speed of sound."));
    }
}

mod tests{
    use hifitime::Epoch;
    use crate::Earth;

    #[test]
    fn test_update_crs_to_trs_dcm(){
        // let mut earth = Earth::new();
        // let crs_to_trs_dcm = earth.update_crs_to_trs_dcm(Epoch::now().unwrap().to_et_seconds());
        // let theta_x = crs_to_trs_dcm.m32.atan2(crs_to_trs_dcm.m33);
        // let theta_y = -crs_to_trs_dcm.m31.atan2((crs_to_trs_dcm.m32.powf(2.0) + crs_to_trs_dcm.m33.powf(2.0)).sqrt());
        // let theta_z = crs_to_trs_dcm.m21.atan2(crs_to_trs_dcm.m11);
        // println!("Angles: {},{},{}", theta_x.to_degrees(), theta_y.to_degrees(), theta_z.to_degrees());
    }
}