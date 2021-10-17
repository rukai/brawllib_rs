use cgmath::Point3;
use std::f32::consts;

use crate::high_level_fighter::HighLevelSubaction;

/// Uses spherical coordinates to represent the cameras location relative to a target.
/// <https://en.wikipedia.org/wiki/Spherical_coordinate_system>
/// <https://threejs.org/docs/#api/en/math/Spherical>
pub struct Camera {
    pub target: Point3<f32>,
    /// radius from the target to the camera
    pub radius: f32,
    /// polar angle from the y (up) axis
    pub phi: f32,
    /// equator angle around the y (up) axis.
    pub theta: f32,
}

impl Camera {
    pub fn new(subaction: &HighLevelSubaction, width: u16, height: u16) -> Camera {
        let mut subaction_extent = subaction.hurt_box_extent();
        subaction_extent.extend(&subaction.hit_box_extent());
        subaction_extent.extend(&subaction.ledge_grab_box_extent());

        let extent_middle_y = (subaction_extent.up + subaction_extent.down) / 2.0;
        let extent_middle_z = (subaction_extent.left + subaction_extent.right) / 2.0;
        let extent_height = subaction_extent.up - subaction_extent.down;
        let extent_width = subaction_extent.right - subaction_extent.left;
        let extent_aspect = extent_width / extent_height;
        let aspect = width as f32 / height as f32;

        let radius =
            (subaction_extent.up - extent_middle_y).max(subaction_extent.right - extent_middle_z);
        let fov = 40.0;
        let fov_rad = fov * consts::PI / 180.0;

        let mut camera_distance = radius / (fov_rad / 2.0).tan();

        // This logic probably only works because this.pixel_width >= this.pixel_height is always true
        if extent_aspect > aspect {
            camera_distance /= aspect;
        } else if extent_width > extent_height {
            camera_distance /= extent_aspect;
        }

        let target = Point3::new(0.0, extent_middle_y, extent_middle_z);

        Camera {
            target,
            radius: camera_distance,
            phi: std::f32::consts::PI / 2.0,
            theta: std::f32::consts::PI * 3.0 / 2.0,
        }
    }
}
