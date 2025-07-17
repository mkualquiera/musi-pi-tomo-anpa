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
        gizmo::{GizmoBindableTexture, GizmoSprite, GizmoSpriteSheet, SpriteSpec},
        Drawer, EngineColor, RenderingSystem,
    },
    InputSystem,
};

struct GameLevelSpec {
    pub background: GizmoSpriteSheet,
    pub decoration: GizmoSpriteSheet,
    collision: Vec<Transform>,
    num_tiles: (usize, usize),
    tile_size: f32,
}

struct GameLevelLoadData {
    background_bytes: &'static [u8],
    decoration_bytes: &'static [u8],
    collision_csv: &'static str,
}

impl GameLevelSpec {
    pub fn load(
        load_data: GameLevelLoadData,
        rendering_system: &mut RenderingSystem,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let background = rendering_system.gizmo_sprite_sheet_from_encoded_image(
            load_data.background_bytes,
            [0.0, 0.0],
            [1.0, 1.0],
            [1, 1],
        );

        let decoration = rendering_system.gizmo_sprite_sheet_from_encoded_image(
            load_data.decoration_bytes,
            [0.0, 0.0],
            [1.0, 1.0],
            [1, 1],
        );

        // Let's do the 0 iq collisions for now
        let mut colliders = Vec::new();
        for (y, row) in load_data.collision_csv.lines().enumerate() {
            for (x, tile_id) in row.split(',').enumerate() {
                if tile_id.trim() == "1" {
                    let transform = Transform::new()
                        .translate(Vec3::new(x as f32, y as f32, 0.0))
                        .scale(Vec3::new(1.0, 1.0, 1.0));
                    colliders.push(transform);
                }
            }
        }

        Ok(Self {
            background,
            decoration,
            collision: colliders,
            num_tiles: (16, 16),
            tile_size: 32.0,
        })
    }

    pub fn get_local_space(&self, base_transform: &Transform) -> Transform {
        let (width, height) = self.num_tiles;
        base_transform.scale(Vec3::new(width as f32, height as f32, 1.0))
    }

    pub fn collides_with(&self, origin: &Transform, other_space: &Transform) -> Option<Collision> {
        for collider in &self.collision {
            if let Some(collision) =
                Collision::do_spaces_collide(&origin.then(collider), other_space)
            {
                return Some(collision);
            }
        }
        None
    }

    pub fn visualize_collisions(
        &self,
        origin: &Transform,
        drawer: &mut Drawer,
        sprite: GizmoSprite,
    ) {
        for collider in &self.collision {
            let transform = origin.then(collider);
            drawer.draw_square_slow(Some(&transform), Some(&EngineColor::RED), sprite.clone());
        }
    }
}

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

    pub fn update<F: Fn(&Transform) -> Option<Collision>>(
        &mut self,
        input: &InputSystem,
        delta_time: f32,
        check_collision: F,
    ) {
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
            self.walking_counter += delta_time;
            if self.walking_counter > 0.15 {
                self.walking_counter = 0.0;
                self.walking_index = (self.walking_index + 1) % 4;
            }

            //self.position += player_direction;
            let previous_x = self.position.x;
            self.position.x += player_direction.x;
            if check_collision(&self.collider(&Transform::new())).is_some() {
                self.position.x = previous_x; // revert x movement if collision
            }
            let previous_y = self.position.y;
            self.position.y += player_direction.y;
            if check_collision(&self.collider(&Transform::new())).is_some() {
                self.position.y = previous_y; // revert y movement if collision
            }
        } else {
            self.walking_counter = 0.20;
            self.walking_index = 1;
            self.direction = 0; // reset direction to down when idle
        }
    }

    pub fn local_space(&self, base_transform: &Transform) -> Transform {
        base_transform
            .translate(Vec3::new(self.position.x, self.position.y, 0.0))
            .set_origin(&Transform::new().translate(Vec3::new(0.5, 0.5, 0.0)))
    }

    pub fn collider(&self, base_transform: &Transform) -> Transform {
        base_transform
            .translate(Vec3::new(self.position.x, self.position.y, 0.0))
            .translate(Vec3::new(0.0, 0.25, 0.0))
            .scale(Vec3::new(0.5, 0.5, 1.0)) // half size for collider
            .set_origin(&Transform::new().translate(Vec3::new(0.5, 0.5, 0.0)))
    }
}

pub struct Game {
    player: Player,
    camera: OrthoCamera,
    player_texture: GizmoSpriteSheet,
    walk_audio: AudioHandle,
    rng: StdRng,

    test_level: GameLevelSpec,
}

impl Game {
    pub fn target_size() -> (u32, u32) {
        (320, 240)
    }

    pub fn alignment_hint() -> u32 {
        32
    }

    pub fn init(rendering_system: &mut RenderingSystem, audio_system: &mut AudioSystem) -> Self {
        let mut rng = StdRng::from_seed([0; 32]); // Seed with zeros for reproducibility
        Self {
            player: Player::new(Vec2::new(1.0, 1.0)),
            camera: {
                let (width, height) = Game::target_size();
                OrthoCamera::new(width as f32, height as f32, 32.0)
            },
            player_texture: rendering_system.gizmo_sprite_sheet_from_encoded_image(
                include_bytes!("assets/char_template.png"),
                [0.0, 0.0],
                [1.0, 1.0],
                [3, 4],
            ),
            walk_audio: audio_system.load_buffer(include_bytes!("assets/walk.wav")),
            rng: StdRng::from_seed([0; 32]), // Seed with zeros for reproducibility
            test_level: GameLevelSpec::load(
                GameLevelLoadData {
                    background_bytes: include_bytes!("assets/level_generated/floor.png"),
                    decoration_bytes: include_bytes!("assets/level_generated/test_with_walls.png"),
                    collision_csv: include_str!("assets/level_generated/collision.csv"),
                },
                rendering_system,
            )
            .expect("Failed to load game level"),
        }
    }

    pub fn update(&mut self, input: &InputSystem, audio_system: &mut AudioSystem, delta_time: f32) {
        let frames = [0, 1, 2, 1];

        let previous_frame = frames[self.player.walking_index as usize] as u32;

        let level_origin =
            Transform::new().set_origin(&Transform::new().translate(Vec3::new(8.0, 8.0, 0.0)));

        self.player.update(input, delta_time, |player_space| {
            self.test_level.collides_with(&level_origin, player_space)
        });

        let frame = frames[self.player.walking_index as usize] as u32;

        if frame == 1 && previous_frame != 1 {
            audio_system.play(&self.walk_audio, self.rng.random_range(0.8..1.2));
        }
    }

    pub fn render(&self, drawer: &mut Drawer) {
        drawer.clear_slow(Color {
            r: 0.001,
            g: 0.001,
            b: 0.001,
            a: 255.0,
        });

        let view_transform = self.camera.get_transform().set_origin(
            &self
                .player
                .local_space(&Transform::new().translate(Vec3::new(0.5, 0.5, 0.0))),
        );

        // Draw level
        let level_transform = self.test_level.get_local_space(
            &view_transform.set_origin(&Transform::new().translate(Vec3::new(8.0, 8.0, 0.0))),
        );
        drawer.draw_square_slow(
            Some(&level_transform),
            Some(&EngineColor::WHITE),
            self.test_level.background.get_sprite([0, 0]).unwrap(),
        );
        drawer.draw_square_slow(
            Some(&level_transform),
            Some(&EngineColor::WHITE),
            self.test_level.decoration.get_sprite([0, 0]).unwrap(),
        );

        let frames = [0, 1, 2, 1];
        let frame = frames[self.player.walking_index as usize] as u32;

        drawer.draw_square_slow(
            Some(&self.player.local_space(&view_transform)),
            Some(&EngineColor::WHITE),
            self.player_texture
                .get_sprite([frame, self.player.direction as u32])
                .unwrap(),
        );
    }
}
