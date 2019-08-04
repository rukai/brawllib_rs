use cgmath::{Quaternion, Point3};

pub(crate) struct Camera {
    pub rotation: Quaternion<f32>,
    pub distance: f32,
    pub target: Point3<f32>,
}
