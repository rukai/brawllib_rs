use cgmath::{Vector3, Matrix4};
use std::f32::consts::PI;

pub fn gen_transform(scale: Vector3<f32>, rot: Vector3<f32>, translate: Vector3<f32>) -> Matrix4<f32> {
    let cosx = (rot.x / 180.0 * PI).cos();
    let sinx = (rot.x / 180.0 * PI).sin();
    let cosy = (rot.y / 180.0 * PI).cos();
    let siny = (rot.y / 180.0 * PI).sin();
    let cosz = (rot.z / 180.0 * PI).cos();
    let sinz = (rot.z / 180.0 * PI).sin();

    Matrix4::new(
        scale.x * cosy * cosz,
        scale.x * cosy * sinz,
        scale.x * siny,
        0.0,

        scale.y * (sinx * siny * cosz - cosx * sinz),
        scale.y * (sinx * siny * sinz + cosx * cosz),
        scale.y * sinx * cosy,
        0.0,

        scale.z * (cosx * siny * cosz + sinx * sinz),
        scale.z * (cosx * siny * sinz - sinx * cosz),
        scale.z * cosx * cosy,
        0.0,

        translate.x,
        translate.y,
        translate.z,
        1.0,
    )
}
