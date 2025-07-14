use glam::{Vec2, Vec3};

use crate::geometry::Transform;

pub struct OrthoCamera {
    screen_width: f32,
    screen_height: f32,
    zoom: f32,
}

impl OrthoCamera {
    pub fn new(screen_width: f32, screen_height: f32, zoom: f32) -> Self {
        Self {
            screen_width,
            screen_height,
            zoom,
        }
    }

    pub fn get_transform(&self) -> Transform {
        Transform::ortographic_size_invariant()
            .translate(Vec3::new(0.5, 0.5, 0.0))
            .scale(Vec3::new(
                1.0 / self.screen_width,
                1.0 / self.screen_height,
                1.0,
            ))
            .scale(Vec3::new(self.zoom, self.zoom, 1.0))
    }
}
