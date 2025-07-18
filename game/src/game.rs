use core::f32;

use glam::{Vec2, Vec3};
use glyphon::cosmic_text::ttf_parser::math;
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
        gizmo::{GizmoSprite, GizmoSpriteSheet},
        Drawer, EngineColor, RenderingSystem,
    },
    InputSystem, InputSystemConfig, KeyPressGroupHandle,
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

    pub fn _visualize_collisions(
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

    fn line_collides_with_level(
        start: Vec2,
        end: Vec2,
        level: &GameLevelSpec,
        level_origin: &Transform,
    ) -> bool {
        let direction = (end - start).normalize();
        let distance = start.distance(end);
        let width = 0.05; // Very thin

        let line_transform = Transform::new()
            .translate(Vec3::new(start.x, start.y, 0.0))
            .rotate(direction.y.atan2(direction.x), Vec3::Z)
            .scale(Vec3::new(distance, width, 1.0))
            .set_origin(&Transform::new().translate(Vec3::new(0.0, 0.5, 0.0)));

        level.collides_with(level_origin, &line_transform).is_some()
    }
}

pub struct MovementController {
    pub position: Vec2,
    pub movement_speed: f32, // Default speed
}

struct MovementIntention {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl MovementIntention {
    pub fn from_input(input: &InputSystem) -> Self {
        Self {
            up: input.is_physical_key_down(KeyCode::KeyW),
            down: input.is_physical_key_down(KeyCode::KeyS),
            left: input.is_physical_key_down(KeyCode::KeyA),
            right: input.is_physical_key_down(KeyCode::KeyD),
        }
    }

    pub fn is_idle(&self) -> bool {
        !self.up && !self.down && !self.left && !self.right
    }

    pub fn idle() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
        }
    }

    pub fn any(&self) -> bool {
        self.up || self.down || self.left || self.right
    }
}

impl MovementController {
    pub fn new(position: Vec2, movement_speed: f32) -> Self {
        Self {
            position,
            movement_speed, // Default speed
        }
    }

    fn update<F: Fn(&Transform) -> Option<Collision>>(
        &mut self,
        intention: &MovementIntention,
        delta_time: f32,
        check_collision: F,
    ) {
        let speed = self.movement_speed * delta_time;
        let mut movement_vector = Vec2::ZERO;
        if intention.up {
            movement_vector.y -= 1.0;
            //self.direction = 2; // up
        }
        if intention.down {
            movement_vector.y += 1.0;
            //self.direction = 0; // down
        }
        if intention.left {
            movement_vector.x -= 1.0;
            //self.direction = 3; // left
        }
        if intention.right {
            movement_vector.x += 1.0;
            //self.direction = 1; // right
        }
        if movement_vector.length() > 0.0 {
            movement_vector = movement_vector.normalize();
            movement_vector *= speed;

            //self.position += player_direction;
            let previous_x = self.position.x;
            self.position.x += movement_vector.x;
            if check_collision(&self.collider(&Transform::new())).is_some() {
                self.position.x = previous_x; // revert x movement if collision
            }
            let previous_y = self.position.y;
            self.position.y += movement_vector.y;
            if check_collision(&self.collider(&Transform::new())).is_some() {
                self.position.y = previous_y; // revert y movement if collision
            }
        }
    }

    pub fn feet_position(&self) -> Vec2 {
        Vec2::new(self.position.x, self.position.y + 0.25) // Feet position is slightly above the center
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

#[derive(Clone, Copy, Debug)]
enum CharacterOrientation {
    Up,
    Down,
    Left,
    Right,
}

struct CharacterWalkAnimation {
    sheet: GizmoSpriteSheet,
    orientation: CharacterOrientation,
    current_frame: usize,
    frame_duration: f32,
    elapsed_time: f32,
    speed: f32, // Speed of the animation
}

enum AnimationEvent {
    FrameChanged(usize),
    None,
}

impl CharacterWalkAnimation {
    pub fn new(sheet: GizmoSpriteSheet, orientation: CharacterOrientation, speed: f32) -> Self {
        Self {
            sheet,
            orientation,
            current_frame: 0,
            frame_duration: 0.2, // Duration for each frame
            elapsed_time: 0.0,
            speed, // Speed of the animation
        }
    }

    pub fn update(
        &mut self,
        delta_time: f32,
        orientation: Option<CharacterOrientation>,
    ) -> AnimationEvent {
        let mut event = AnimationEvent::None;
        match orientation {
            Some(new_orientation) => {
                self.orientation = new_orientation;
                self.elapsed_time += delta_time * self.speed; // Adjust elapsed time by speed factor
                if self.elapsed_time >= self.frame_duration {
                    self.current_frame = (self.current_frame + 1) % 4; // Cycle through 4 frames
                    event = AnimationEvent::FrameChanged(self.current_frame);
                    self.elapsed_time = 0.0;
                }
            }
            None => {
                self.current_frame = 0;
                self.elapsed_time = 0.0;
            }
        }
        event
    }

    pub fn get_current_sprite(&self) -> GizmoSprite {
        let walk_cycle = [1, 2, 1, 0];
        let offset = walk_cycle[self.current_frame];
        let sprite_index = match self.orientation {
            CharacterOrientation::Up => [offset, 2],
            CharacterOrientation::Down => [offset, 0],
            CharacterOrientation::Left => [offset, 3],
            CharacterOrientation::Right => [offset, 1],
        };
        self.sheet
            .get_sprite(sprite_index)
            .expect("Sprite not found")
    }
}

enum EnemyAIState {
    Idle,
    Chasing(Vec2),
    Wandering(CharacterOrientation),
    Engaging,
}

struct Enemy {
    controller: MovementController,
    state: EnemyAIState,
    animation: CharacterWalkAnimation,
    attack_controller: AttackController,
    health: f32,
    max_health: f32,
    poise: f32,
    max_poise: f32,
}

impl Enemy {
    pub fn new(position: Vec2, walking_sprite_sheet: GizmoSpriteSheet) -> Self {
        Self {
            controller: MovementController::new(position, 1.5),
            state: EnemyAIState::Idle,
            animation: CharacterWalkAnimation::new(
                walking_sprite_sheet,
                CharacterOrientation::Down,
                0.75, // Speed of the animation
            ),
            attack_controller: AttackController::new(),
            health: 20.0,
            max_health: 20.0,
            poise: 50.0,
            max_poise: 50.0,
        }
    }

    pub fn update<CollidesWithWorld: Fn(&Transform) -> Option<Collision>>(
        &mut self,
        delta_time: f32,
        check_collision: CollidesWithWorld,
        player: &MovementController,
        level: &GameLevelSpec,
        rng: &mut StdRng,
    ) -> CharacterEvent {
        let mut event = CharacterEvent::None;

        // Recover some poise
        self.poise = (self.poise + delta_time * 5.0).min(self.max_poise);

        let distance_to_player = self
            .controller
            .feet_position()
            .distance(player.feet_position());

        match self.state {
            EnemyAIState::Idle | EnemyAIState::Wandering(_) => {
                let mut found_something = false;
                if distance_to_player < 3.0 {
                    let can_see = !GameLevelSpec::line_collides_with_level(
                        self.controller.feet_position(),
                        player.feet_position().floor() + 0.5,
                        level,
                        &Transform::new()
                            .set_origin(&Transform::new().translate(Vec3::new(8.0, 8.0, 0.0))),
                    );
                    if can_see {
                        self.state = EnemyAIState::Chasing(player.feet_position().floor() + 0.5);
                        found_something = true;
                    }
                }

                if !found_something && rng.random_bool((2.0 * delta_time as f64).min(1.0)) {
                    // Randomly decide to wander
                    let direction = rng.random_range(0..5);
                    let orientation = match direction {
                        0 => Some(CharacterOrientation::Up),
                        1 => Some(CharacterOrientation::Down),
                        2 => Some(CharacterOrientation::Left),
                        3 => Some(CharacterOrientation::Right),
                        _ => None,
                    };
                    if let Some(orientation) = orientation {
                        self.state = EnemyAIState::Wandering(orientation);
                        info!("Enemy wandering in direction: {:?}", orientation);
                    } else {
                        self.state = EnemyAIState::Idle; // No valid direction, stay idle
                        info!("Enemy idle, no valid wandering direction");
                    }
                }
            }
            EnemyAIState::Chasing(target_position) => {
                let can_see = !GameLevelSpec::line_collides_with_level(
                    self.controller.feet_position(),
                    player.feet_position().floor() + 0.5,
                    level,
                    &Transform::new()
                        .set_origin(&Transform::new().translate(Vec3::new(8.0, 8.0, 0.0))),
                );
                if can_see {
                    self.state = EnemyAIState::Chasing(player.feet_position().floor() + 0.5);

                    let distance_to_target = self
                        .controller
                        .feet_position()
                        .distance(player.feet_position());

                    if distance_to_target < 0.7 {
                        self.state = EnemyAIState::Engaging;
                    }
                }
            }
            EnemyAIState::Engaging => {
                let distance_to_target = self
                    .controller
                    .feet_position()
                    .distance(player.feet_position());
                if distance_to_target > 1.0 && self.attack_controller.is_ready() {
                    self.state = EnemyAIState::Idle;
                }
            }
            _ => {}
        };

        let mut intention = MovementIntention {
            up: false,
            down: false,
            left: false,
            right: false,
        };

        let mut desired_orientation = None;

        if self.attack_controller.is_ready() {
            match self.state {
                EnemyAIState::Chasing(target_position) => {
                    if target_position.y < self.controller.feet_position().y - 0.02 {
                        intention.up = true;
                    } else if target_position.y > self.controller.feet_position().y + 0.02 {
                        intention.down = true;
                    }
                    if target_position.x < self.controller.feet_position().x - 0.02 {
                        intention.left = true;
                    } else if target_position.x > self.controller.feet_position().x + 0.02 {
                        intention.right = true;
                    }

                    let delta_x = target_position.x - self.controller.feet_position().x;
                    let delta_y = target_position.y - self.controller.feet_position().y;
                    if delta_x.abs() > delta_y.abs() {
                        if delta_x < 0.0 {
                            desired_orientation = Some(CharacterOrientation::Left);
                        } else {
                            desired_orientation = Some(CharacterOrientation::Right);
                        }
                    } else if delta_y < 0.0 {
                        desired_orientation = Some(CharacterOrientation::Up);
                    } else {
                        desired_orientation = Some(CharacterOrientation::Down);
                    }
                }
                EnemyAIState::Wandering(orientation) => {
                    match orientation {
                        CharacterOrientation::Up => intention.up = true,
                        CharacterOrientation::Down => intention.down = true,
                        CharacterOrientation::Left => intention.left = true,
                        CharacterOrientation::Right => intention.right = true,
                    }
                    desired_orientation = Some(orientation);
                }
                EnemyAIState::Engaging => {
                    // If the player is closer than 0.5 units, walk away from them
                    info!(
                        "Enemy engaging player at distance: {:?}",
                        distance_to_player
                    );

                    let target_position = player.feet_position();
                    if distance_to_player < 0.6 {
                        if target_position.x < self.controller.feet_position().x {
                            intention.right = true;
                        } else {
                            intention.left = true;
                        }
                        if target_position.y < self.controller.feet_position().y {
                            intention.down = true;
                        } else {
                            intention.up = true;
                        }
                    }

                    if intention.any() {
                        let delta_x = target_position.x - self.controller.feet_position().x;
                        let delta_y = target_position.y - self.controller.feet_position().y;
                        if delta_x.abs() > delta_y.abs() {
                            if delta_x < 0.0 {
                                desired_orientation = Some(CharacterOrientation::Left);
                            } else {
                                desired_orientation = Some(CharacterOrientation::Right);
                            }
                        } else if delta_y < 0.0 {
                            desired_orientation = Some(CharacterOrientation::Up);
                        } else {
                            desired_orientation = Some(CharacterOrientation::Down);
                        }
                    }
                }
                _ => {}
            }
        }

        let animation_event = self.animation.update(delta_time, desired_orientation);
        if let AnimationEvent::FrameChanged(frame) = animation_event {
            if frame == 0 || frame == 2 {
                event = CharacterEvent::WalkCycle; // Trigger walk cycle event
            }
        };

        let last_position = self.controller.position;

        self.controller
            .update(&intention, delta_time, check_collision);

        let attack_controller_event = self.attack_controller.update(
            delta_time,
            if matches!(self.state, EnemyAIState::Engaging) {
                AttackIntention::Duration(0.2)
            } else {
                AttackIntention::None
            },
        );
        if !matches!(attack_controller_event, AttackControllerEvent::None) {
            event = CharacterEvent::AttackControllerEvent(attack_controller_event);
        }

        match self.state {
            EnemyAIState::Chasing(_) | EnemyAIState::Wandering(_) => {
                if last_position == self.controller.position {
                    self.state = EnemyAIState::Idle; // If we didn't move, go back to idle
                    info!("Enemy idle, no movement detected");
                }
            }
            EnemyAIState::Engaging => {
                if self.attack_controller.is_ready() {
                    self.state = EnemyAIState::Idle; // If we are ready to attack, go back to idle
                    info!("Enemy idle, ready to attack");
                }
            }
            _ => {}
        };

        event
    }

    pub fn get_attack_space(&self, base_transform: &Transform) -> Option<(Transform, f32)> {
        self.attack_controller.get_attack_space(
            &self.controller,
            base_transform,
            self.animation.orientation,
        )
    }

    pub fn health_bar_space(&self, base_transform: &Transform, full: bool) -> Transform {
        let health_ratio = if !full {
            self.health / self.max_health
        } else {
            1.0
        };
        let local_space = self.controller.local_space(base_transform);
        local_space
            .translate(Vec3::new(0.5, 0.0, 0.0)) // Position above the enemy
            .translate(Vec3::new(0.0, -0.2, 0.0)) // Position above the enemy
            .scale(Vec3::new(0.8, 0.1, 1.0))
            .set_origin(&Transform::new().translate(Vec3::new(0.5, 0.5, 0.0)))
            .scale(Vec3::new(health_ratio, 1.0, 1.0)) // Scale based on health
    }

    pub fn poise_bar_space(&self, base_transform: &Transform, full: bool) -> Transform {
        let poise_ratio = if !full {
            self.poise / self.max_poise
        } else {
            1.0
        };
        let local_space = self.controller.local_space(base_transform);
        local_space
            .translate(Vec3::new(0.5, 0.0, 0.0)) // Position above the enemy
            .translate(Vec3::new(0.0, -0.1, 0.0)) // Position above the enemy
            .scale(Vec3::new(0.8, 0.1, 1.0))
            .set_origin(&Transform::new().translate(Vec3::new(0.5, 0.5, 0.0)))
            .scale(Vec3::new(poise_ratio, 1.0, 1.0)) // Scale based on poise
    }
}

enum AttackState {
    Ready,
    Windup {
        current_time: f32,
    },
    Attacking {
        duration_left: f32,
        windup_duration: f32,
    },
    Cooldown {
        duration_left: f32,
    },
    Staggered {
        duration_left: f32,
    },
}

enum AttackIntention {
    None,
    Perpetual,
    Duration(f32),
}

struct AttackController {
    state: AttackState,
}

enum AttackControllerEvent {
    StartWindup,
    StartAttack,
    None,
}

impl AttackController {
    pub fn new() -> Self {
        Self {
            state: AttackState::Ready,
        }
    }

    pub fn update(
        &mut self,
        delta_time: f32,
        attack_intention: AttackIntention,
    ) -> AttackControllerEvent {
        let mut event = AttackControllerEvent::None;
        match self.state {
            AttackState::Ready => {
                if !matches!(attack_intention, AttackIntention::None) {
                    //self.state = AttackState::Attacking { duration_left: 0.2 };
                    self.state = AttackState::Windup { current_time: 0.0 };
                    event = AttackControllerEvent::StartWindup;
                }
            }
            AttackState::Windup { current_time } => {
                let mut wants_to_finish_windup = match attack_intention {
                    AttackIntention::None => true,
                    AttackIntention::Perpetual => false,
                    AttackIntention::Duration(duration) => current_time + delta_time >= duration,
                };
                if current_time < 0.2 {
                    wants_to_finish_windup = false; // Windup lasts 0.2 seconds
                }
                if !wants_to_finish_windup {
                    self.state = AttackState::Windup {
                        current_time: current_time + delta_time,
                    };
                } else {
                    self.state = AttackState::Attacking {
                        duration_left: 0.2,
                        windup_duration: current_time,
                    };
                    event = AttackControllerEvent::StartAttack;
                }
            }
            AttackState::Attacking {
                duration_left,
                windup_duration,
            } => {
                if duration_left <= 0.0 {
                    self.state = AttackState::Cooldown { duration_left: 0.1 };
                } else {
                    self.state = AttackState::Attacking {
                        duration_left: duration_left - delta_time,
                        windup_duration,
                    };
                }
            }
            AttackState::Cooldown { duration_left } => {
                if duration_left <= 0.0 {
                    self.state = AttackState::Ready;
                } else {
                    self.state = AttackState::Cooldown {
                        duration_left: duration_left - delta_time,
                    };
                }
            }
            AttackState::Staggered { duration_left } => {
                if duration_left <= 0.0 {
                    self.state = AttackState::Ready;
                } else {
                    self.state = AttackState::Staggered {
                        duration_left: duration_left - delta_time,
                    };
                }
            }
        }
        event
    }

    pub fn get_attack_space(
        &self,
        controller: &MovementController,
        base_transform: &Transform,
        orientation: CharacterOrientation,
    ) -> Option<(Transform, f32)> {
        if let AttackState::Attacking {
            duration_left: _,
            windup_duration,
        } = self.state
        {
            let local_space = controller.local_space(base_transform);

            let degrees = match orientation {
                CharacterOrientation::Up => f32::consts::PI * 0.0,
                CharacterOrientation::Down => f32::consts::PI * 1.0,
                CharacterOrientation::Left => f32::consts::PI * 1.5,
                CharacterOrientation::Right => f32::consts::PI * 0.5,
            };

            Some((
                local_space
                    .translate(Vec3::new(0.5, 0.5, 0.0)) // Attack space is slightly above the center
                    .rotate_2d(degrees)
                    .scale(Vec3::new(1.0, 1.0, 1.0)) // Size of the attack space
                    .translate(Vec3::new(0.0, 0.0, 0.0))
                    .set_origin(&Transform::new().translate(Vec3::new(0.5, 1.0, 0.0))),
                windup_duration,
            ))
        } else {
            None
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.state, AttackState::Ready)
    }

    pub fn make_staggered(&mut self, duration: f32) -> bool {
        if let AttackState::Staggered { duration_left } = self.state {
            self.state = AttackState::Staggered {
                duration_left: duration.max(duration_left),
            };
            false
        } else {
            self.state = AttackState::Staggered {
                duration_left: duration,
            };
            true
        }
    }
}

struct Player {
    controller: MovementController,
    animation: CharacterWalkAnimation,
    direction_group_handle: KeyPressGroupHandle,
    attack_controller: AttackController,
    health: f32,
    poise: f32,
}

enum CharacterEvent {
    None,
    AttackControllerEvent(AttackControllerEvent),
    WalkCycle,
}

impl Player {
    pub fn new(
        position: Vec2,
        walking_sprite_sheet: GizmoSpriteSheet,
        input_config: &mut InputSystemConfig,
    ) -> Self {
        Self {
            controller: MovementController::new(position, 2.0),
            animation: CharacterWalkAnimation::new(
                walking_sprite_sheet,
                CharacterOrientation::Down,
                1.0, // Speed of the animation
            ),
            direction_group_handle: input_config.allocate_group(&[
                KeyCode::KeyW,
                KeyCode::KeyS,
                KeyCode::KeyA,
                KeyCode::KeyD,
            ]),
            attack_controller: AttackController::new(),
            health: 100.0, // Default health
            poise: 50.0,
        }
    }

    pub fn update<CollidesWithWorld: Fn(&Transform) -> Option<Collision>>(
        &mut self,
        input: &InputSystem,
        delta_time: f32,
        check_collision: CollidesWithWorld,
    ) -> CharacterEvent {
        let mut event = CharacterEvent::None;

        // Recover some poise
        self.poise = (self.poise + delta_time * 5.0).min(50.0);

        let movement_intention = if self.attack_controller.is_ready() {
            MovementIntention::from_input(input)
        } else {
            MovementIntention::idle()
        };

        self.controller
            .update(&movement_intention, delta_time, check_collision);

        // Simple logic
        let desired_orientation = if movement_intention.is_idle() {
            None
        } else {
            match input.get_last_key_pressed(&self.direction_group_handle) {
                Some(KeyCode::KeyW) => Some(CharacterOrientation::Up),
                Some(KeyCode::KeyS) => Some(CharacterOrientation::Down),
                Some(KeyCode::KeyA) => Some(CharacterOrientation::Left),
                Some(KeyCode::KeyD) => Some(CharacterOrientation::Right),
                _ => None,
            }
        };

        let animation_event = self.animation.update(delta_time, desired_orientation);
        if let AnimationEvent::FrameChanged(frame) = animation_event {
            if frame == 0 || frame == 2 {
                event = CharacterEvent::WalkCycle;
            }
        }

        let wants_to_attack = input.is_physical_key_down(KeyCode::KeyL);
        let attack_event = self.attack_controller.update(
            delta_time,
            if wants_to_attack {
                AttackIntention::Perpetual
            } else {
                AttackIntention::None
            },
        );
        if !matches!(attack_event, AttackControllerEvent::None) {
            event = CharacterEvent::AttackControllerEvent(attack_event);
        }
        event
    }

    pub fn get_attack_space(&self, base_transform: &Transform) -> Option<(Transform, f32)> {
        self.attack_controller.get_attack_space(
            &self.controller,
            base_transform,
            self.animation.orientation,
        )
    }
}

pub struct Game {
    player: Player,
    camera: OrthoCamera,
    walk_audio: AudioHandle,
    rng: StdRng,
    test_level: GameLevelSpec,
    enemy: Enemy,
    attacking_enemy: bool,

    windup_audio: AudioHandle,
    attack_audio: AudioHandle,
    staggered_audio: AudioHandle,
    stance_broken_audio: AudioHandle,
}

impl Game {
    pub fn target_size() -> (u32, u32) {
        (320, 240)
    }

    pub fn alignment_hint() -> u32 {
        32
    }

    pub fn init(
        rendering_system: &mut RenderingSystem,
        audio_system: &mut AudioSystem,
        input_config: &mut InputSystemConfig,
    ) -> Self {
        let rng = StdRng::from_seed([0; 32]); // Seed with zeros for reproducibility
        Self {
            player: Player::new(
                Vec2::new(1.0, 1.0),
                rendering_system.gizmo_sprite_sheet_from_encoded_image(
                    include_bytes!("assets/char_template.png"),
                    [0.0, 0.0],
                    [1.0, 1.0],
                    [3, 4],
                ),
                input_config,
            ),
            camera: {
                let (width, height) = Game::target_size();
                OrthoCamera::new(width as f32, height as f32, 32.0)
            },
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

            enemy: Enemy::new(
                Vec2::new(2.0, -2.0),
                rendering_system.gizmo_sprite_sheet_from_encoded_image(
                    include_bytes!("assets/char_template.png"),
                    [0.0, 0.0],
                    [1.0, 1.0],
                    [3, 4],
                ),
            ),

            attacking_enemy: false,

            windup_audio: audio_system.load_buffer(include_bytes!("assets/windup_2.wav")),
            attack_audio: audio_system.load_buffer(include_bytes!("assets/attack_1.wav")),
            staggered_audio: audio_system.load_buffer(include_bytes!("assets/staggered_1.wav")),
            stance_broken_audio: audio_system
                .load_buffer(include_bytes!("assets/stance_broken_1.wav")),
        }
    }

    pub fn update(&mut self, input: &InputSystem, audio_system: &mut AudioSystem, delta_time: f32) {
        let level_origin =
            Transform::new().set_origin(&Transform::new().translate(Vec3::new(8.0, 8.0, 0.0)));

        if self.player.health > 0.0 {
            let player_event = self.player.update(input, delta_time, |player_space| {
                self.test_level.collides_with(&level_origin, player_space)
            });

            match player_event {
                CharacterEvent::None => {}
                CharacterEvent::AttackControllerEvent(attack_event) => match attack_event {
                    AttackControllerEvent::StartWindup => {
                        audio_system.play(&self.windup_audio, self.rng.random_range(0.8..1.2));
                    }
                    AttackControllerEvent::StartAttack => {
                        audio_system.play(&self.attack_audio, self.rng.random_range(0.8..1.2));
                    }
                    AttackControllerEvent::None => {}
                },
                CharacterEvent::WalkCycle => {
                    audio_system.play(&self.walk_audio, self.rng.random_range(0.8..1.2));
                }
            }
        }

        if self.enemy.health > 0.0 {
            let enemy_event = self.enemy.update(
                delta_time,
                |enemy_space| self.test_level.collides_with(&level_origin, enemy_space),
                &self.player.controller,
                &self.test_level,
                &mut self.rng,
            );

            match enemy_event {
                CharacterEvent::None => {}
                CharacterEvent::AttackControllerEvent(attack_event) => match attack_event {
                    AttackControllerEvent::StartWindup => {
                        audio_system.play(&self.windup_audio, self.rng.random_range(0.6..1.0));
                    }
                    AttackControllerEvent::StartAttack => {
                        audio_system.play(&self.attack_audio, self.rng.random_range(0.6..1.0));
                    }
                    AttackControllerEvent::None => {}
                },
                CharacterEvent::WalkCycle => {
                    audio_system.play(&self.walk_audio, self.rng.random_range(0.6..1.0));
                }
            }

            if let Some((attack_space, windup_duration)) =
                self.enemy.get_attack_space(&level_origin)
            {
                if Collision::do_spaces_collide(
                    &attack_space,
                    &self.player.controller.collider(&level_origin),
                )
                .is_some()
                {
                    self.player.health -= 400.0 * delta_time * windup_duration; // Deal damage to the player
                    self.player.poise -= 400.0 * delta_time * windup_duration; // Deal poise damage to the player
                    if self
                        .player
                        .attack_controller
                        .make_staggered(windup_duration)
                    {
                        audio_system.play(&self.staggered_audio, self.rng.random_range(0.8..1.2));
                    }
                    if self.player.poise <= 0.0 {
                        self.player.poise = 50.0; // Prevent negative poise
                        self.player.attack_controller.make_staggered(1.0);
                        audio_system
                            .play(&self.stance_broken_audio, self.rng.random_range(0.8..1.2));
                    }
                    if self.player.health <= 0.0 {
                        self.player.health = 0.0; // Prevent negative health
                        info!("Player defeated!");
                    }
                }
            }
        }

        if let Some((attack_space, windup_duration)) = self.player.get_attack_space(&level_origin) {
            self.attacking_enemy = Collision::do_spaces_collide(
                &attack_space,
                &self.enemy.controller.collider(&level_origin),
            )
            .is_some();
            if self.attacking_enemy {
                self.enemy.health -= 20.0 * delta_time * windup_duration; // Deal damage to the enemy
                self.enemy.poise -= 200.0 * delta_time * windup_duration; // Deal poise damage to the enemy
                if self
                    .enemy
                    .attack_controller
                    .make_staggered(windup_duration * 0.25)
                {
                    audio_system.play(&self.staggered_audio, self.rng.random_range(0.6..1.0));
                }
                if self.enemy.poise <= 0.0 {
                    self.enemy.poise = 50.0; // Prevent negative poise
                    self.enemy.attack_controller.make_staggered(1.0);
                    audio_system.play(&self.stance_broken_audio, self.rng.random_range(0.6..1.0));
                }
                if self.enemy.health <= 0.0 {
                    self.enemy.health = 0.0; // Prevent negative health
                    info!("Enemy defeated!");
                }
            }
        } else {
            self.attacking_enemy = false;
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
                .controller
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

        let color = if self.player.health > 0.0 {
            EngineColor::WHITE
        } else {
            EngineColor::BLACK
        };
        drawer.draw_square_slow(
            Some(&self.player.controller.local_space(&view_transform)),
            Some(&color),
            self.player.animation.get_current_sprite(),
        );

        let white_sprite = drawer.white_sprite();

        let color = if self.attacking_enemy {
            EngineColor::RED
        } else {
            EngineColor::BLUE
        };

        if let Some((attack_space, _)) = self.player.get_attack_space(&view_transform) {
            drawer.draw_square_slow(Some(&attack_space), Some(&color), white_sprite);
        }

        //let frame = frames[self.enemy.controller.walking_index as usize] as u32;

        if self.enemy.health > 0.0 {
            let color = if let EnemyAIState::Chasing(_) = self.enemy.state {
                EngineColor::RED
            } else {
                EngineColor::BLUE
            };

            drawer.draw_square_slow(
                Some(&self.enemy.controller.local_space(&view_transform)),
                Some(&color),
                self.enemy.animation.get_current_sprite(),
            );

            let white_sprite = drawer.white_sprite();

            if let Some((attack_space, _)) = self.enemy.get_attack_space(&view_transform) {
                drawer.draw_square_slow(
                    Some(&attack_space),
                    Some(&EngineColor::GREEN),
                    white_sprite,
                );
            }

            // Draw enemy health bar
            drawer.draw_square_slow(
                Some(&self.enemy.health_bar_space(&view_transform, true)),
                Some(&EngineColor::RED.additive_darken(0.7)),
                white_sprite,
            );
            drawer.draw_square_slow(
                Some(&self.enemy.health_bar_space(&view_transform, false)),
                Some(&EngineColor::RED),
                white_sprite,
            );

            // Draw enemy poise bar
            drawer.draw_square_slow(
                Some(&self.enemy.poise_bar_space(&view_transform, true)),
                Some(&EngineColor::YELLOW.additive_darken(0.7)),
                white_sprite,
            );
            drawer.draw_square_slow(
                Some(&self.enemy.poise_bar_space(&view_transform, false)),
                Some(&EngineColor::YELLOW),
                white_sprite,
            );
        }

        // Draw player health
        let ui_transform = drawer.ortho;

        let white_sprite = drawer.white_sprite();
        drawer.draw_square_slow(
            Some(
                &ui_transform
                    .translate(Vec3::new(16.0, 16.0, 0.0))
                    .scale(Vec3::new(100.0, 16.0, 1.0)),
            ),
            Some(&EngineColor::RED.additive_darken(0.7)),
            white_sprite,
        );
        drawer.draw_square_slow(
            Some(
                &ui_transform
                    .translate(Vec3::new(16.0, 16.0, 0.0))
                    .scale(Vec3::new(self.player.health, 16.0, 1.0)),
            ),
            Some(&EngineColor::RED),
            white_sprite,
        );

        // Draw player poise
        drawer.draw_square_slow(
            Some(
                &ui_transform
                    .translate(Vec3::new(16.0, 32.0, 0.0))
                    .scale(Vec3::new(100.0, 16.0, 1.0)),
            ),
            Some(&EngineColor::YELLOW.additive_darken(0.7)),
            white_sprite,
        );
        drawer.draw_square_slow(
            Some(
                &ui_transform
                    .translate(Vec3::new(16.0, 32.0, 0.0))
                    .scale(Vec3::new(self.player.poise * 2.0, 16.0, 1.0)),
            ),
            Some(&EngineColor::YELLOW),
            white_sprite,
        );
    }
}
