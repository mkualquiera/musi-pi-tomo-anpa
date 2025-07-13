use glam::Vec3;
use wgpu::{Buffer, Queue};

pub struct Transform {
    matrix: glam::Mat4,
    raw: [[f32; 4]; 4],
}

impl Transform {
    pub fn new() -> Self {
        let mat = glam::Mat4::IDENTITY;
        Self {
            matrix: mat,
            raw: mat.to_cols_array_2d(),
        }
    }

    pub fn from_matrix(matrix: glam::Mat4) -> Self {
        let raw = matrix.to_cols_array_2d();
        Self { matrix, raw }
    }

    pub fn translate(&self, translation: Vec3) -> Self {
        let mat = self.matrix * glam::Mat4::from_translation(translation);
        Self {
            matrix: mat,
            raw: mat.to_cols_array_2d(),
        }
    }

    pub fn rotate(&self, angle: f32, axis: Vec3) -> Self {
        let mat = self.matrix * glam::Mat4::from_axis_angle(axis, angle);
        Self {
            matrix: mat,
            raw: mat.to_cols_array_2d(),
        }
    }

    pub fn scale(&self, scale: Vec3) -> Self {
        let mat = self.matrix * glam::Mat4::from_scale(scale);
        Self {
            matrix: mat,
            raw: mat.to_cols_array_2d(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.raw)
    }

    pub fn write_buffer(&self, buffer: &Buffer, queue: &Queue) {
        queue.write_buffer(buffer, 0, self.as_bytes());
    }

    pub fn ortographic_size_invariant() -> Self {
        // Creates a size invariant orthographic transform
        let mat = glam::Mat4::orthographic_rh(0.0, 1.0, 1.0, 0.0, -100.0, 100.0);
        Self {
            matrix: mat,
            raw: mat.to_cols_array_2d(),
        }
    }

    pub fn project(&self, point: Vec3) -> Vec3 {
        // Projects a point using the transform matrix
        (self.matrix * point.extend(1.0)).truncate()
    }

    pub fn map_towards(&self, other: &Self) -> Self {
        let mat = other.matrix.inverse() * self.matrix;
        Self {
            matrix: mat,
            raw: mat.to_cols_array_2d(),
        }
    }
}
