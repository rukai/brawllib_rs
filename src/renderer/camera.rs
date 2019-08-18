use cgmath::Point3;

/// Uses spherical coordinates to represent the cameras location relative to a target.
/// https://en.wikipedia.org/wiki/Spherical_coordinate_system
/// https://threejs.org/docs/#api/en/math/Spherical
pub(crate) struct Camera {
    pub target: Point3<f32>,
    /// radius from the target to the camera
    pub radius: f32,
    /// polar angle from the y (up) axis
    pub phi: f32,
    /// equator angle around the y (up) axis.
    pub theta: f32,
}
