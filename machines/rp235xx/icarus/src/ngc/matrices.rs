use nalgebra::{Matrix, Matrix3, Matrix3x1};

pub fn rotate_x(angle_rad: f64) -> Matrix3<f64>{
    let rotation = Matrix3::new(
        1.0, 0.0, 0.0,
        0.0, angle_rad.cos(), -1.0 * angle_rad.sin(),
        0.0, angle_rad.sin(), angle_rad.cos(),
    );
    return rotation;
}
pub fn rotate_y(angle_rad: f64) -> Matrix3<f64>{
    let rotation = Matrix3::new(
        angle_rad.cos(), 0.0, angle_rad.sin(),
        0.0, 1.0, 0.0,
        -1.0 * angle_rad.sin(), 0.0, angle_rad.cos()
    );
    return rotation;
}
pub fn rotate_z(angle_rad: f64) -> Matrix3<f64>{
    let rotation = Matrix3::new(
        angle_rad.cos(), -1.0 * angle_rad.sin(), 0.0,
        angle_rad.sin(), angle_rad.cos(), 0.0,
        0.0, 0.0, 1.0 
    );
    return rotation;
}

pub fn get_euler_angles(matrix: Matrix3<f64>) -> Matrix3x1<f64>{
    let x = matrix.m32.atan2(matrix.m33);
    let y = -matrix.m31.atan2((matrix.m33.powf(2.0)+matrix.m32.powf(2.0)).sqrt());
    let z = matrix.m21.atan2(matrix.m11);

    let xyz = Matrix3x1::new(x,y,z);
    return xyz;
}
pub fn euler_to_dcm(roll: f64, pitch: f64, yaw: f64)->Matrix3<f64>{
    return Matrix3::new(
        pitch.cos(), pitch.sin() * roll.sin(), -pitch.sin() * roll.cos(),
        yaw.sin() * pitch.sin(), yaw.cos()*roll.cos()-yaw.sin()*pitch.cos()*roll.sin(), yaw.cos()*roll.sin()+yaw.sin()*pitch.cos()*roll.cos(),
        yaw.cos()*pitch.sin(), -yaw.sin()*pitch.cos()-pitch.cos()*roll.sin()*yaw.cos(), -yaw.sin()*roll.sin()+yaw.cos()*pitch.cos()*roll.cos()
    );
}