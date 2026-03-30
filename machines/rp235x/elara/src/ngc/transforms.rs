use nalgebra::{Matrix3, Matrix3x1};

pub fn angular_rate_dcm(roll: f64, pitch: f64, yaw: f64)-> Matrix3<f64>{
    let rotation_matrix = Matrix3::new(
        0.0, -yaw, pitch,
        yaw, 0.0, -roll,
        -pitch, roll, 0.0    
    );
    return rotation_matrix;
}

pub fn aircraft_wind_to_body(angle_of_attack: f64, side_slip: f64)->Matrix3<f64>{
    let t_bs = Matrix3::new(
        angle_of_attack.cos(), 0.0, -angle_of_attack.sin(),
        0.0, 1.0, 0.0,
        angle_of_attack.sin(), 0.0, angle_of_attack.cos()
    );

    let t_ws = Matrix3::new(
        side_slip.cos(), side_slip.sin(), 0.0,
        -side_slip.sin(), side_slip.cos(), 0.0,
        0.0, 0.0, 1.0
    );

    let t_wb = t_ws * t_bs.transpose();
    return t_wb;   
}

pub fn aeroballistic_wind_to_body(angle_of_attack: f64, aerodynamic_roll_angle: f64)->Matrix3<f64>{
    let t_ab = Matrix3::new(
        angle_of_attack.cos(),  angle_of_attack.sin()*aerodynamic_roll_angle.sin(), angle_of_attack.sin()*aerodynamic_roll_angle.cos(),
        0.0,                    aerodynamic_roll_angle.cos(),                       -aerodynamic_roll_angle.sin(),
        -angle_of_attack.sin(), angle_of_attack.cos()*aerodynamic_roll_angle.sin(), angle_of_attack.cos()*aerodynamic_roll_angle.cos() 
    );
    return t_ab;
}

pub fn flight_path_to_geographic(heading_angle: f64, flight_path_angle: f64)->Matrix3<f64>{
    let rotation_matrix = Matrix3::new(
        flight_path_angle.cos()*heading_angle.cos(),    flight_path_angle.cos()*heading_angle.sin(),    -flight_path_angle.sin(),
        -heading_angle.sin(),                           heading_angle.cos(),                            0.0,
        flight_path_angle.sin()*heading_angle.cos(),    flight_path_angle.sin()*heading_angle.sin(),    flight_path_angle.cos()
    );
    return rotation_matrix;
}

pub fn body_to_ned(roll: f64, pitch: f64, yaw: f64)-> Matrix3<f64>{
    let phi = roll;
    let tht = pitch;
    let psi = yaw;
    let rotation_matrix = Matrix3::new(
        psi.cos()*tht.cos(),                                  psi.sin()*tht.cos(),                                     -tht.sin(),
        psi.cos()*tht.sin()*phi.sin()-psi.sin()*phi.cos(),    psi.sin()*tht.sin()*phi.sin()+psi.cos()*phi.cos(),  tht.cos()*phi.sin(),
        psi.cos()*tht.sin()*phi.cos()+psi.sin()*phi.sin(),    psi.sin()*tht.sin()*phi.cos()-psi.cos()*phi.sin(),  tht.cos()*phi.cos()
    );
    return rotation_matrix;
}

pub fn ecef_to_ned(latitude: f64, longitude: f64)->Matrix3<f64>{
    let rotation_matrix = Matrix3::new(
        -latitude.sin()*longitude.cos(),    -latitude.sin()*longitude.sin(),    latitude.cos(),
        -longitude.sin(),                   longitude.cos(),                    0.0,
        -latitude.cos()*longitude.cos(),    -latitude.cos()*longitude.sin(),    -latitude.sin()
    );
    return rotation_matrix;
}

pub fn rx(angle: f64)->Matrix3<f64>{
    let rx = matrix![
        1.0, 0.0, 0.0;
        0.0, angle.cos(), -angle.sin();
        0.0, angle.sin(), angle.cos();
    ];
    return rx;
}
pub fn ry(angle: f64)->Matrix3<f64>{
    let ry = matrix![
        angle.cos(), 0.0, angle.sin();
        0.0, 1.0, 0.0;
        -angle.sin(), 0.0, angle.cos();
    ];
    return ry;
}
pub fn rz(angle: f64)->Matrix3<f64>{
    let rz = matrix![
        angle.cos(), -angle.sin(), 0.0;
        angle.sin(), angle.cos(), 0.0;
        0.0, 0.0, 1.0;
    ];
    return rz;
}

pub fn euler_313_rotation(psi: f64, theta: f64, phi: f64)->Matrix3<f64>{
    let rotation = rz(psi)*rx(theta)*rz(phi);
    return rotation;
}

pub fn spherical_to_rectangular(radius: f64, polar: f64, azimuth: f64)->Matrix3x1<f64>{
    let x = radius * polar.cos()*azimuth.cos();
    let y = radius * polar.cos()*azimuth.sin();
    let z = radius * polar.sin();
    return Matrix3x1::new(x,y,z);
}
