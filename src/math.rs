use cgmath::{Matrix4, Vector3};
use std::f32::consts::PI;

pub fn gen_transform(
    scale: Vector3<f32>,
    rot: Vector3<f32>,
    translate: Vector3<f32>,
) -> Matrix4<f32> {
    let (sinx, cosx) = ((rot.x / 180.0) * PI).sin_cos();
    let (siny, cosy) = ((rot.y / 180.0) * PI).sin_cos();
    let (sinz, cosz) = ((rot.z / 180.0) * PI).sin_cos();

    Matrix4::new(
        scale.x * cosy * cosz,
        scale.x * cosy * sinz,
        -scale.x * siny,
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
