use glam::{Vec2, Vec3};
use log::info;
use rand::{rngs::StdRng, Rng, SeedableRng};
use wgpu::Color;
use winit::keyboard::KeyCode;

use crate::{
    audio::{AudioHandle, AudioSystem},
    collision::Collision,
    geometry::Transform,
    ortographic_camera::OrthoCamera,
    renderer::{
        gizmo::{GizmoBindableTexture, GizmoSprite, SpriteSpec},
        Drawer, EngineColor, RenderingSystem,
    },
    InputSystem,
};

pub struct Player {
    pub position: Vec2,
    pub walking_index: u8,
    pub walking_counter: f32,
    pub direction: u8,
}

impl Player {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            walking_index: 0,
            walking_counter: 0.0,
            direction: 0, // 0: down, 1: left, 2: up, 3: right
        }
    }

    const PLAYER_SPEED: f32 = 4.0;

    pub fn update(&mut self, input: &InputSystem, delta_time: f32) {
        let speed = Player::PLAYER_SPEED * delta_time;
        let mut player_direction = Vec2::ZERO;
        if input.is_physical_key_down(KeyCode::KeyW) {
            player_direction.y -= 1.0;
            //self.direction = 2; // up
        }
        if input.is_physical_key_down(KeyCode::KeyS) {
            player_direction.y += 1.0;
            //self.direction = 0; // down
        }
        if input.is_physical_key_down(KeyCode::KeyA) {
            player_direction.x -= 1.0;
            //self.direction = 3; // left
        }
        if input.is_physical_key_down(KeyCode::KeyD) {
            player_direction.x += 1.0;
            //self.direction = 1; // right
        }
        if player_direction.length() > 0.0 {
            player_direction = player_direction.normalize();
            player_direction *= speed;
            if player_direction.x < 0.0 {
                self.direction = 3; // left
            } else if player_direction.x > 0.0 {
                self.direction = 1; // right
            } else if player_direction.y < 0.0 {
                self.direction = 2; // up
            } else if player_direction.y > 0.0 {
                self.direction = 0; // down
            }
            self.position += player_direction;
            self.walking_counter += delta_time;
            if self.walking_counter > 0.15 {
                self.walking_counter = 0.0;
                self.walking_index = (self.walking_index + 1) % 4;
            }
        } else {
            self.walking_counter = 0.0;
            self.walking_index = 1;
            self.direction = 0; // reset direction to down when idle
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
    walk_audio: AudioHandle,
    rng: StdRng,
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
                OrthoCamera::new(width as f32, height as f32, 32.0)
            },
            player_texture: rendering_system
                .gizmo_texture_from_encoded_image(include_bytes!("assets/char_template.png")),
            walk_audio: audio_system.load_buffer(include_bytes!("assets/walk.wav")),
            rng: StdRng::from_seed([0; 32]), // Seed with zeros for reproducibility
        }
    }

    pub fn update(&mut self, input: &InputSystem, audio_system: &mut AudioSystem, delta_time: f32) {
        let frames = [0, 1, 2, 1];

        let previous_frame = frames[self.player.walking_index as usize] as u32;
        self.player.update(input, delta_time);
        let frame = frames[self.player.walking_index as usize] as u32;

        if frame == 1 && previous_frame != 1 {
            audio_system.play(&self.walk_audio, self.rng.random_range(0.8..1.2));
        }
    }

    pub fn render(&self, drawer: &mut Drawer) {
        drawer.clear_slow(Color {
            r: 0.2,
            g: 1.0,
            b: 0.2,
            a: 1.0,
        });

        let view_transform = self
            .camera
            .get_transform()
            .set_origin(&self.player.local_space(&Transform::new()));

        // Draw objects
        for object in &self.objects {
            drawer.draw_square_slow(
                Some(&view_transform.translate(Vec3::new(object.x, object.y, 0.0))),
                Some(&EngineColor::RED),
                GizmoSprite {
                    texture: &self.player_texture,
                    sprite_spec: SpriteSpec {
                        use_texture: 1,
                        region_start: [0.0, 0.0],
                        region_end: [1.0, 1.0],
                        num_tiles: [3, 4],
                        selected_tile: [1, 0],
                    },
                },
            );
        }

        let frames = [0, 1, 2, 1];
        let frame = frames[self.player.walking_index as usize] as u32;

        // draw player as a square
        drawer.draw_square_slow(
            Some(
                &self
                    .player
                    .local_space(&view_transform)
                    .translate(Vec3::new(2.0 - 0.25 + 0.25 / 2.0, 0.0, 0.0))
                    .shear(-2.0, 0.0),
            ),
            Some(&EngineColor::BLACK),
            GizmoSprite {
                texture: &self.player_texture,
                sprite_spec: SpriteSpec {
                    use_texture: 1,
                    region_start: [0.0, 0.0],
                    region_end: [1.0, 1.0],
                    num_tiles: [3, 4],
                    selected_tile: [frame, self.player.direction as u32],
                },
            },
        );
        drawer.draw_square_slow(
            Some(&self.player.local_space(&view_transform)),
            Some(&EngineColor::WHITE),
            GizmoSprite {
                texture: &self.player_texture,
                sprite_spec: SpriteSpec {
                    use_texture: 1,
                    region_start: [0.0, 0.0],
                    region_end: [1.0, 1.0],
                    num_tiles: [3, 4],
                    selected_tile: [frame, self.player.direction as u32],
                },
            },
        );
    }
}
