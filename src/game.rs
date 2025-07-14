use glam::{Vec2, Vec3};
use log::info;
use wgpu::Color;
use winit::keyboard::KeyCode;

use crate::{
    audio::{AudioHandle, AudioSystem},
    collision::Collision,
    geometry::Transform,
    ortographic_camera::OrthoCamera,
    renderer::{gizmo::GizmoBindableTexture, Drawer, EngineColor, RenderingSystem},
    InputSystem,
};

pub struct Player {
    pub position: Vec2,
}

impl Player {
    pub fn new(position: Vec2) -> Self {
        Self { position }
    }

    const PLAYER_SPEED: f32 = 4.0;

    pub fn update(&mut self, input: &InputSystem, delta_time: f32) {
        let speed = Player::PLAYER_SPEED * delta_time;
        let mut player_direction = Vec2::ZERO;
        if input.is_physical_key_down(KeyCode::KeyW) {
            player_direction.y -= 1.0;
        }
        if input.is_physical_key_down(KeyCode::KeyS) {
            player_direction.y += 1.0;
        }
        if input.is_physical_key_down(KeyCode::KeyA) {
            player_direction.x -= 1.0;
        }
        if input.is_physical_key_down(KeyCode::KeyD) {
            player_direction.x += 1.0;
        }
        if player_direction.length() > 0.0 {
            player_direction = player_direction.normalize();
            player_direction *= speed;
            self.position += player_direction;
        }
    }

    pub fn local_space(&self, base_transform: &Transform) -> Transform {
        base_transform.translate(Vec3::new(self.position.x, self.position.y, 0.0))
    }
}

pub struct Game {
    player: Player,
    objects: Vec<Vec2>,
    camera: OrthoCamera,
    player_texture: GizmoBindableTexture,
}

impl Game {
    pub fn target_size() -> (u32, u32) {
        (320, 240)
    }

    pub fn init(rendering_system: &mut RenderingSystem, audio_system: &mut AudioSystem) -> Self {
        Self {
            player: Player::new(Vec2::new(0.0, 0.0)),
            objects: Vec::from([
                Vec2::new(9.0, 4.0),
                Vec2::new(7.0, 1.0),
                Vec2::new(-3.0, -2.0),
            ]),
            camera: {
                let (width, height) = Game::target_size();
                OrthoCamera::new(width as f32, height as f32, 20.0)
            },
            player_texture: rendering_system
                .gizmo_texture_from_encoded_image(include_bytes!("assets/char_template.png")),
        }
    }

    pub fn update(&mut self, input: &InputSystem, audio_system: &mut AudioSystem, delta_time: f32) {
        self.player.update(input, delta_time);
    }

    pub fn render(&self, drawer: &mut Drawer) {
        drawer.clear_slow(Color::BLACK);

        let view_transform = self
            .camera
            .get_transform()
            .set_origin(&self.player.local_space(&Transform::new()));

        // Draw objects
        for object in &self.objects {
            drawer.draw_square_slow(
                Some(&view_transform.translate(Vec3::new(object.x, object.y, 0.0))),
                Some(&EngineColor::RED),
                &self.player_texture,
            );
        }

        // draw player as a square
        drawer.draw_square_slow(
            Some(&self.player.local_space(&view_transform)),
            Some(&EngineColor::WHITE),
            &self.player_texture,
        );
    }
}
