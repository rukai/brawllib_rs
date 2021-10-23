use cgmath::Point3;
use std::f32::consts;

use crate::high_level_fighter::{Extent, HighLevelSubaction};

pub enum CharacterFacing {
    Left,
    Right,
}

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
    extent: Extent,
}

impl Camera {
    pub fn new(subaction: &HighLevelSubaction, window_width: u16, window_height: u16) -> Camera {
        let mut extent = subaction.hurt_box_extent();
        extent.extend(&subaction.hit_box_extent());
        extent.extend(&subaction.ledge_grab_box_extent());
        Camera::new_from_extent(extent, window_width, window_height, CharacterFacing::Right)
    }

    fn new_from_extent(
        extent: Extent,
        window_width: u16,
        window_height: u16,
        facing: CharacterFacing,
    ) -> Camera {
        let extent_middle_y = (extent.up + extent.down) / 2.0;
        let extent_middle_z = (extent.left + extent.right) / 2.0;
        let extent_height = extent.up - extent.down;
        let extent_width = extent.right - extent.left;
        let extent_aspect = extent_width / extent_height;
        let aspect = window_width as f32 / window_height as f32;

        let radius = (extent.up - extent_middle_y).max(extent.right - extent_middle_z);
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

        let theta = match facing {
            CharacterFacing::Left => std::f32::consts::PI / 2.0,
            CharacterFacing::Right => std::f32::consts::PI * 3.0 / 2.0,
        };

        Camera {
            target,
            radius: camera_distance,
            phi: std::f32::consts::PI / 2.0,
            theta,
            extent,
        }
    }

    pub fn reset(&mut self, window_width: u16, window_height: u16, facing: CharacterFacing) {
        *self = Camera::new_from_extent(self.extent.clone(), window_width, window_height, facing)
    }
}
