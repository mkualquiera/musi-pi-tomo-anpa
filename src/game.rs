use glam::{Vec2, Vec3};
use log::info;
use wgpu::Color;
use winit::keyboard::KeyCode;

use crate::{
    audio::{AudioHandle, AudioSystem},
    collision::Collision,
    geometry::Transform,
    renderer::{Drawer, EngineColor, RenderingSystem},
    InputSystem,
};

struct PaddleState {
    position: f32,
    last_velocity: f32,
}

impl Default for PaddleState {
    fn default() -> Self {
        Self {
            position: 0.5,
            last_velocity: 0.0,
        }
    }
}

impl PaddleState {
    const PADDLE_WIDTH: f32 = 0.2;
    const PADDLE_HEIGHT: f32 = PaddleState::PADDLE_WIDTH / 4.0;
    const PADDLE_SPEED: f32 = 0.5; // Speed in normalized units

    pub fn local_space(&self, ortho_si: &Transform, is_player_a: bool) -> Transform {
        // Position the origin at the top left

        let horizontal_range = 1.0 - PaddleState::PADDLE_WIDTH;
        let vertical_range = 1.0 - PaddleState::PADDLE_HEIGHT;

        let vertical_position = if is_player_a { 0.0 } else { 1.0 };

        let x = self.position * horizontal_range;
        let y = vertical_position * vertical_range;

        ortho_si.translate(Vec3::new(x, y, 0.0)).scale(Vec3 {
            x: PaddleState::PADDLE_WIDTH,
            y: PaddleState::PADDLE_HEIGHT,
            z: 1.0,
        })
    }

    pub fn goal_local_space(&self, ortho_si: &Transform, is_player_a: bool) -> Transform {
        let vertical_position = if is_player_a { 0.0 } else { 1.0 };

        let vertical_range = 1.0 - PaddleState::PADDLE_HEIGHT;

        let y = vertical_position * vertical_range;

        ortho_si.translate(Vec3::new(0.0, y, 0.0)).scale(Vec3 {
            x: 1.0,
            y: PaddleState::PADDLE_HEIGHT,
            z: 1.0,
        })
    }

    pub fn move_left(&mut self, delta_time: f32) {
        self.position -= PaddleState::PADDLE_SPEED * delta_time;
        if self.position < 0.0 {
            self.position = 0.0;
        } else {
            self.last_velocity = -PaddleState::PADDLE_SPEED;
        }
    }
    pub fn move_right(&mut self, delta_time: f32) {
        self.position += PaddleState::PADDLE_SPEED * delta_time;
        if self.position > 1.0 {
            self.position = 1.0;
        } else {
            self.last_velocity = PaddleState::PADDLE_SPEED;
        }
    }
    pub fn reset_velocity(&mut self) {
        self.last_velocity = 0.0;
    }
}

#[derive(Default)]
struct DualPaddleState {
    player_a: PaddleState,
    player_b: PaddleState,
}

impl DualPaddleState {
    pub fn local_spaces(&self, ortho_si: &Transform) -> (Transform, Transform) {
        (
            self.player_a.local_space(ortho_si, true),
            self.player_b.local_space(ortho_si, false),
        )
    }

    pub fn move_paddles(&mut self, input: &InputSystem, delta_time: f32) {
        self.player_a.reset_velocity();
        self.player_b.reset_velocity();

        if input.is_physical_key_down(KeyCode::KeyA) {
            self.player_a.move_left(delta_time);
        }
        if input.is_physical_key_down(KeyCode::KeyD) {
            self.player_a.move_right(delta_time);
        }
        if input.is_physical_key_down(KeyCode::ArrowLeft) {
            self.player_b.move_left(delta_time);
        }
        if input.is_physical_key_down(KeyCode::ArrowRight) {
            self.player_b.move_right(delta_time);
        }
    }
}

struct Ball {
    position: Vec2,
    velocity: Vec2,
}

impl Ball {
    const RADIUS: f32 = 0.05; // Radius in normalized units
    const BALL_SPEED: f32 = 0.5; // Speed in normalized units

    pub fn update(
        &mut self,
        delta_time: f32,
        paddles: &DualPaddleState,
        ortho_si: &Transform,
        bounce_sound: &AudioHandle,
        wall_sound: &AudioHandle,
        audio_system: &mut AudioSystem,
    ) {
        // At every update, convert some percentage of x velocity into y velocity
        let amount = self.velocity.x * 0.3 * delta_time;
        self.velocity.x -= amount;
        if self.velocity.y > 0.0 {
            self.velocity.y += amount;
        } else {
            self.velocity.y -= amount;
        }
        self.velocity = self.velocity.normalize() * Ball::BALL_SPEED; // Normalize speed
        self.position += self.velocity * delta_time;
        if self.position.x < 0.0 {
            self.position.x = 0.0;
            self.velocity.x = -self.velocity.x; // Bounce off left wall
            audio_system.play(wall_sound, self.velocity.dot(Vec2::X).abs() + 0.5);
        } else if self.position.x > (1.0 - Self::RADIUS) {
            self.position.x = 1.0 - Self::RADIUS;
            self.velocity.x = -self.velocity.x; // Bounce off right wall
            audio_system.play(wall_sound, self.velocity.dot(Vec2::X).abs() + 0.5);
        }
        if self.position.y < 0.0 {
            self.position.y = 0.0;
            self.velocity.y = -self.velocity.y; // Bounce off top wall
        } else if self.position.y > (1.0 - Self::RADIUS) {
            self.position.y = 1.0 - Self::RADIUS;
            self.velocity.y = -self.velocity.y; // Bounce off bottom wall
        }
        if Collision::do_spaces_collide(
            &self.local_space(ortho_si),
            &paddles.player_a.local_space(ortho_si, true),
        )
        .is_some()
        {
            self.velocity.y = -self.velocity.y; // Bounce off player A paddle
            let previous_velocity = self.velocity;
            self.velocity.x += paddles.player_a.last_velocity * 2.0; // Add paddle velocity
            self.velocity = self.velocity.normalize() * Ball::BALL_SPEED; // Normalize speed
            self.position.y = PaddleState::PADDLE_HEIGHT;
            audio_system.play(
                bounce_sound,
                previous_velocity.dot(self.velocity).abs() + 0.5,
            );
        } else if Collision::do_spaces_collide(
            &self.local_space(ortho_si),
            &paddles.player_b.local_space(ortho_si, false),
        )
        .is_some()
        {
            self.velocity.y = -self.velocity.y; // Bounce off player B paddle
            let previous_velocity = self.velocity;
            self.velocity.x += paddles.player_b.last_velocity * 2.0; // Add paddle velocity
            self.velocity = self.velocity.normalize() * Ball::BALL_SPEED; // Normalize speed
            self.position.y = 1.0 - PaddleState::PADDLE_HEIGHT - Ball::RADIUS;
            audio_system.play(
                bounce_sound,
                previous_velocity.dot(self.velocity).abs() + 0.5,
            );
        } else {
            // Check if the ball is inside the goal area of either player
            if Collision::do_spaces_collide(
                &self.local_space(ortho_si),
                &paddles.player_a.goal_local_space(ortho_si, true),
            )
            .is_some()
            {
                info!("Player B scores!");
                self.position = Vec2::new(0.5, 0.5); // Reset ball position
                self.velocity = Vec2::new(0.1, 0.1).normalize() * Ball::BALL_SPEED;
            // Reset velocity
            } else if Collision::do_spaces_collide(
                &self.local_space(ortho_si),
                &paddles.player_b.goal_local_space(ortho_si, false),
            )
            .is_some()
            {
                info!("Player A scores!");
                self.position = Vec2::new(0.5, 0.5); // Reset ball position
                self.velocity = Vec2::new(0.1, 0.1).normalize() * Ball::BALL_SPEED;
                // Reset velocity
            }
        }
    }

    pub fn local_space(&self, ortho_si: &Transform) -> Transform {
        let x = self.position.x;
        let y = self.position.y;

        ortho_si
            .translate(Vec3::new(x, y, 0.0))
            .scale(Vec3::splat(Self::RADIUS))
    }
}

impl Default for Ball {
    fn default() -> Self {
        Self {
            position: Vec2::new(0.5, 0.5),
            velocity: Vec2::new(0.1, 0.1).normalize() * Ball::BALL_SPEED, // Initial velocity
        }
    }
}

pub struct Game {
    paddles: DualPaddleState,
    ball: Ball,
    bouce_sound: AudioHandle,
    wall_sound: AudioHandle,
}

impl Game {
    pub fn target_size() -> (u32, u32) {
        (320, 240)
    }

    pub fn init(rendering_system: &mut RenderingSystem, audio_system: &mut AudioSystem) -> Self {
        Self {
            paddles: DualPaddleState::default(),
            ball: Ball::default(),
            bouce_sound: audio_system.load_buffer(include_bytes!("assets/bounce_1.wav")),
            wall_sound: audio_system.load_buffer(include_bytes!("assets/wall_1.wav")),
        }
    }

    pub fn update(&mut self, input: &InputSystem, audio_system: &mut AudioSystem, delta_time: f32) {
        self.paddles.move_paddles(input, delta_time);
        self.ball.update(
            delta_time,
            &self.paddles,
            &Transform::ortographic_size_invariant(),
            &self.bouce_sound,
            &self.wall_sound,
            audio_system,
        );
    }

    pub fn render(&self, drawer: &mut Drawer) {
        drawer.clear_slow(Color::BLACK);

        let t = &Transform::ortographic_size_invariant();

        let (player_a_space, player_b_space) = self.paddles.local_spaces(t);
        drawer.draw_square_slow(Some(&player_a_space), Some(&EngineColor::RED));
        drawer.draw_square_slow(Some(&player_b_space), Some(&EngineColor::BLUE));

        let ball_space = self.ball.local_space(t);
        drawer.draw_square_slow(Some(&ball_space), Some(&EngineColor::WHITE));
    }
}
