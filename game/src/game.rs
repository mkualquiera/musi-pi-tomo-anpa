use core::{f32, num};
use std::{collections::HashMap, rc::Rc};

use glam::{Vec2, Vec3};
use glyphon::{
    cosmic_text::{ttf_parser::math, Align, CacheKeyFlags, FeatureTag, FontFeatures},
    Attrs, Color as GlyphonColor,
};
use log::info;
use rand::{rngs::StdRng, seq::IndexedRandom, Rng, SeedableRng};
use wgpu::Color;
use winit::keyboard::KeyCode;

use crate::{
    audio::{AudioHandle, AudioSystem},
    collision::Collision,
    geometry::Transform,
    nimi::{convert_latin_to_ucsur, number_to_toki_pona},
    ortographic_camera::OrthoCamera,
    renderer::{
        gizmo::{GizmoSprite, GizmoSpriteSheet},
        text::FeaturedTextBuffer,
        Drawer, EngineColor, RenderingSystem,
    },
    InputSystem, InputSystemConfig, KeyPressGroupHandle,
};

struct GameLevelSpec {
    pub background: GizmoSpriteSheet,
    pub decoration: GizmoSpriteSheet,
    collision: Vec<(Transform, u32)>, // (Transform, tile_id)
    enemy_locations: Vec<Vec2>,
    num_tiles: (usize, usize),
    tile_size: f32,
}

struct GameLevelLoadData {
    background_bytes: &'static [u8],
    decoration_bytes: &'static [u8],
    collision_csv: &'static str,
    enemies_csv: &'static str,
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
                let tile_id: u32 = tile_id.trim().parse().expect("Failed to parse tile ID");
                if tile_id != 0 {
                    let transform = Transform::new()
                        .translate(Vec3::new(x as f32, y as f32, 0.0))
                        .scale(Vec3::new(1.0, 1.0, 1.0));
                    colliders.push((transform, tile_id));
                }
            }
        }

        let mut enemy_locations = Vec::new();
        for (y, row) in load_data.enemies_csv.lines().enumerate() {
            for (x, tile_id) in row.split(',').enumerate() {
                let tile_id: u32 = tile_id.trim().parse().expect("Failed to parse tile ID");
                if tile_id != 0 {
                    enemy_locations.push(Vec2::new(x as f32 + 0.5, y as f32 + 0.25));
                }
            }
        }

        Ok(Self {
            background,
            decoration,
            collision: colliders,
            enemy_locations,
            num_tiles: (16, 16),
            tile_size: 32.0,
        })
    }

    pub fn get_local_space(&self, base_transform: &Transform) -> Transform {
        let (width, height) = self.num_tiles;
        base_transform.scale(Vec3::new(width as f32, height as f32, 1.0))
    }

    pub fn collides_with<CollisionHandler: FnMut(Collision, u32)>(
        &self,
        origin: &Transform,
        other_space: &Transform,
        handler: &mut CollisionHandler,
    ) {
        for (collider, id) in &self.collision {
            if let Some(collision) =
                Collision::do_spaces_collide(&origin.then(collider), other_space)
            {
                handler(collision, *id);
            }
        }
    }

    pub fn _visualize_collisions(
        &self,
        origin: &Transform,
        drawer: &mut Drawer,
        sprite: GizmoSprite,
    ) {
        for (collider, id) in &self.collision {
            let transform = origin.then(collider);
            drawer.draw_square_slow(Some(&transform), Some(&EngineColor::RED), sprite.clone());
        }
    }

    fn line_collides_with_level(
        start: Vec2,
        end: Vec2,
        level: &GameLevelSpec,
        level_origin: &Transform,
        query_value: u32,
    ) -> bool {
        let direction = (end - start).normalize();
        let distance = start.distance(end);
        let width = 0.05; // Very thin

        let line_transform = Transform::new()
            .translate(Vec3::new(start.x, start.y, 0.0))
            .rotate(direction.y.atan2(direction.x), Vec3::Z)
            .scale(Vec3::new(distance, width, 1.0))
            .set_origin(&Transform::new().translate(Vec3::new(0.0, 0.5, 0.0)));

        let mut collides = false;
        level.collides_with(level_origin, &line_transform, &mut |_collision, id| {
            if id == query_value {
                collides = true;
            }
        });
        collides
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
                            .set_origin(&Transform::new().translate(Vec3::new(0.0, 0.0, 0.0))),
                        1,
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
                        .set_origin(&Transform::new().translate(Vec3::new(0.0, 0.0, 0.0))),
                    1,
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

    healing_flasks: u32,
    max_healing_flasks: u32,
    healing_state: HealingState,
    healing_group_handle: KeyPressGroupHandle,

    num_crystals: u32,
}

enum CharacterEvent {
    None,
    AttackControllerEvent(AttackControllerEvent),
    WalkCycle,
}

enum HealingState {
    Ready,
    Healing { current_time: f32 },
}

impl HealingState {
    pub fn is_ready(&self) -> bool {
        matches!(self, HealingState::Ready)
    }

    pub fn start_healing(&mut self) {
        *self = HealingState::Healing { current_time: 0.0 };
    }

    pub fn cancel_healing(&mut self) {
        *self = HealingState::Ready;
    }

    pub fn update(&mut self, delta_time: f32) -> bool {
        if let HealingState::Healing { current_time } = self {
            *current_time += delta_time;
            if *current_time >= 1.0 {
                // Healing takes 1 second
                *self = HealingState::Ready;
            }
            true
        } else {
            false
        }
    }
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
            healing_flasks: 5,
            max_healing_flasks: 5,
            healing_state: HealingState::Ready,
            healing_group_handle: input_config.allocate_group(&[KeyCode::KeyH]),
            num_crystals: 0, // Default number of crystals
        }
    }

    pub fn update<CollidesWithWorld: Fn(&Transform) -> Option<Collision>>(
        &mut self,
        input: &mut InputSystem,
        delta_time: f32,
        check_collision: CollidesWithWorld,
    ) -> CharacterEvent {
        let mut event = CharacterEvent::None;

        let wants_to_attack = input.is_physical_key_down(KeyCode::KeyL);
        let wants_to_heal = input
            .get_last_key_pressed(&self.healing_group_handle)
            .is_some()
            && self.healing_flasks > 0
            && self.attack_controller.is_ready();
        input.debounce(&self.healing_group_handle);

        if wants_to_heal {
            self.healing_flasks -= 1;
            self.healing_state.start_healing();
        }

        if self.healing_state.update(delta_time) {
            self.health = (self.health + delta_time * 40.0).min(100.0);
            self.controller.movement_speed = 1.0;
        } else {
            self.controller.movement_speed = 2.0;
        }

        // Recover some poise
        self.poise = (self.poise + delta_time * 5.0).min(50.0);

        let movement_intention = if self.attack_controller.is_ready() {
            MovementIntention::from_input(input)
        } else {
            MovementIntention::idle()
        };

        self.controller
            .update(&movement_intention, delta_time, check_collision);

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

        let attack_event = self.attack_controller.update(
            delta_time,
            if wants_to_attack {
                self.healing_state.cancel_healing();
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

    pub fn stagger(&mut self, duration: f32) -> bool {
        self.healing_state.cancel_healing();
        self.attack_controller.make_staggered(duration)
    }
}

struct ActiveRoom {
    spec: Rc<GameLevelSpec>,
    enemies: Vec<Enemy>,
}

impl ActiveRoom {
    pub fn from_spec(spec: Rc<GameLevelSpec>, enemy_sprite_sheet: GizmoSpriteSheet) -> Self {
        let mut enemies = Vec::new();
        for enemy_position in &spec.enemy_locations {
            let enemy = Enemy::new(*enemy_position, enemy_sprite_sheet.clone());
            enemies.push(enemy);
        }

        Self { spec, enemies }
    }
}

struct RoomManager {
    room_pool: Vec<Rc<GameLevelSpec>>,
    rooms: HashMap<(i32, i32, i32), ActiveRoom>,
    current_room: (i32, i32, i32),
    rng: StdRng,
    enemy_sprite_sheet: GizmoSpriteSheet,
}

impl RoomManager {
    pub fn new(spawn_spec: GameLevelSpec, enemy_sprite_sheet: GizmoSpriteSheet) -> Self {
        let mut rooms = HashMap::new();
        rooms.insert(
            (0, 0, 0),
            ActiveRoom::from_spec(Rc::new(spawn_spec), enemy_sprite_sheet.clone()),
        );
        Self {
            room_pool: Vec::new(),
            rooms,
            current_room: (0, 0, 0),         // Starting room
            rng: StdRng::from_seed([0; 32]), // Seed with zeros for reproducibility
            enemy_sprite_sheet: enemy_sprite_sheet.clone(),
        }
    }

    pub fn add_room_spec(mut self, spec: GameLevelSpec) -> Self {
        self.room_pool.push(Rc::new(spec));
        self
    }

    pub fn get_current_room(&self) -> &ActiveRoom {
        self.rooms
            .get(&self.current_room)
            .expect("Current room not found")
    }

    pub fn get_current_room_mut(&mut self) -> &mut ActiveRoom {
        self.rooms
            .get_mut(&self.current_room)
            .expect("Current room not found")
    }

    pub fn change_room(&mut self, position: (i32, i32, i32)) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.rooms.entry(position) {
            let new_room_spec = self
                .room_pool
                .choose(&mut self.rng)
                .expect("No room available for spawning");

            let new_room =
                ActiveRoom::from_spec(new_room_spec.clone(), self.enemy_sprite_sheet.clone());
            e.insert(new_room);
            self.current_room = position; // Update current room to the newly created one
        } else {
            self.current_room = position;
        }
    }
}

enum CrystalCountState {
    None,
    Counting { duration: f32 },
}

struct CrystalCountBuffer {
    target_num: f32,
    current_num: f32,
    state: CrystalCountState,
    speed: f32,
}

impl CrystalCountBuffer {
    pub fn new(current_num: f32, speed: f32) -> Self {
        Self {
            target_num: current_num,
            current_num,
            state: CrystalCountState::None,
            speed,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        let speed_per_second = self.speed * delta_time;
        let delta = self.target_num - self.current_num;
        if delta.abs() < speed_per_second {
            self.current_num = self.target_num; // Snap to target if within speed range
        } else if delta > 0.0 {
            self.current_num += speed_per_second * delta.signum(); // Increment towards target
        }

        match self.state {
            CrystalCountState::None => {
                if self.current_num != self.target_num {
                    self.state = CrystalCountState::Counting {
                        duration: 0.0, // Start counting down from 0.5 seconds
                    };
                }
            }
            CrystalCountState::Counting { duration } => {
                if self.current_num == self.target_num {
                    self.state = CrystalCountState::None; // Stop counting if we reached the target
                } else {
                    self.state = CrystalCountState::Counting {
                        duration: duration + delta_time,
                    };
                }
            }
        }
    }

    pub fn get_load(&self) -> f32 {
        // duration is the load
        match self.state {
            CrystalCountState::None => 0.0,
            CrystalCountState::Counting { duration } => duration,
        }
    }
}

pub struct Game {
    player: Player,
    camera: OrthoCamera,
    walk_audio: AudioHandle,
    rng: StdRng,

    windup_audio: AudioHandle,
    attack_audio: AudioHandle,
    staggered_audio: AudioHandle,
    stance_broken_audio: AudioHandle,

    manager: RoomManager,

    ui_sheet_32: GizmoSpriteSheet,
    ui_sheet_16: GizmoSpriteSheet,
    num_flasks_text: FeaturedTextBuffer,

    num_crystals_text: FeaturedTextBuffer,
    crystal_count_buffer: CrystalCountBuffer,
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
        let ui_sheet_32 = rendering_system.gizmo_sprite_sheet_from_encoded_image(
            include_bytes!("assets/ui.png"),
            [0.0, 0.0],
            [1.0, 1.0],
            [2, 5],
        );
        let ui_sheet_16 = rendering_system.gizmo_sprite_sheet_from_encoded_image(
            include_bytes!("assets/ui.png"),
            [0.0, 0.0],
            [1.0, 1.0],
            [4, 10],
        );

        rendering_system.load_font(include_bytes!("assets/leko majuna.ttf"));

        let num_flasks_text = rendering_system.create_text_buffer(
            16.0,
            17.0,
            200.0,
            16.0,
            "ala",
            Attrs::new().family(glyphon::Family::SansSerif),
            Align::Left,
        );

        let num_crystals_text = rendering_system.create_text_buffer(
            8.0,
            9.0,
            128.0,
            8.0,
            "ala",
            Attrs::new().family(glyphon::Family::SansSerif),
            Align::Right,
        );

        let rng = StdRng::from_seed([0; 32]); // Seed with zeros for reproducibility
        Self {
            player: Player::new(
                Vec2::new(8.0, 8.0),
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
            windup_audio: audio_system.load_buffer(include_bytes!("assets/windup_2.wav")),
            attack_audio: audio_system.load_buffer(include_bytes!("assets/attack_1.wav")),
            staggered_audio: audio_system.load_buffer(include_bytes!("assets/staggered_1.wav")),
            stance_broken_audio: audio_system
                .load_buffer(include_bytes!("assets/stance_broken_1.wav")),

            manager: RoomManager::new(
                GameLevelSpec::load(
                    GameLevelLoadData {
                        background_bytes: include_bytes!("assets/level_generated/spawn_floor.png"),
                        decoration_bytes: include_bytes!(
                            "assets/level_generated/spawn_with_walls.png"
                        ),
                        collision_csv: include_str!("assets/level_generated/spawn_collision.csv"),
                        enemies_csv: include_str!("assets/level_generated/spawn_enemies.csv"),
                    },
                    rendering_system,
                )
                .expect("Failed to load spawn level"),
                rendering_system.gizmo_sprite_sheet_from_encoded_image(
                    include_bytes!("assets/char_template.png"),
                    [0.0, 0.0],
                    [1.0, 1.0],
                    [3, 4],
                ),
            )
            .add_room_spec(
                GameLevelSpec::load(
                    GameLevelLoadData {
                        background_bytes: include_bytes!("assets/level_generated/base_0_floor.png"),
                        decoration_bytes: include_bytes!(
                            "assets/level_generated/base_0_with_walls.png"
                        ),
                        collision_csv: include_str!("assets/level_generated/base_0_collision.csv"),
                        enemies_csv: include_str!("assets/level_generated/base_0_enemies.csv"),
                    },
                    rendering_system,
                )
                .expect("Failed to load level"),
            ),
            ui_sheet_16,
            ui_sheet_32,
            num_flasks_text,
            num_crystals_text,
            crystal_count_buffer: CrystalCountBuffer::new(0.0, 10.0),
        }
    }

    pub fn update(
        &mut self,
        input: &mut InputSystem,
        audio_system: &mut AudioSystem,
        rendering_system: &mut RenderingSystem,
        delta_time: f32,
    ) {
        self.num_flasks_text.set_text(
            rendering_system,
            &convert_latin_to_ucsur(&number_to_toki_pona(self.player.healing_flasks)),
        );

        self.crystal_count_buffer.target_num = self.player.num_crystals as f32;
        self.crystal_count_buffer.update(delta_time);
        self.num_crystals_text.set_text(
            rendering_system,
            &convert_latin_to_ucsur(&number_to_toki_pona(
                self.crystal_count_buffer.current_num as u32,
            )),
        );

        let level_origin =
            Transform::new().set_origin(&Transform::new().translate(Vec3::new(0.0, 0.0, 0.0)));

        let room = self.manager.get_current_room_mut();

        for enemy in room.enemies.iter_mut() {
            if enemy.health > 0.0 {
                let enemy_event = enemy.update(
                    delta_time,
                    |enemy_space| {
                        let mut collision_result = None;
                        room.spec.collides_with(
                            &level_origin,
                            enemy_space,
                            &mut |collision, id| {
                                if id == 1 {
                                    collision_result = Some(collision);
                                }
                            },
                        );
                        collision_result
                    },
                    &self.player.controller,
                    &room.spec,
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

                if let Some((attack_space, windup_duration)) = enemy.get_attack_space(&level_origin)
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
                            audio_system
                                .play(&self.staggered_audio, self.rng.random_range(0.8..1.2));
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
        }

        if self.player.health > 0.0 {
            for enemy in room.enemies.iter_mut() {
                if enemy.health <= 0.0 {
                    continue; // Skip dead enemies
                }
                if let Some((attack_space, windup_duration)) =
                    self.player.get_attack_space(&level_origin)
                {
                    let attacking_enemy = Collision::do_spaces_collide(
                        &attack_space,
                        &enemy.controller.collider(&level_origin),
                    )
                    .is_some();
                    if attacking_enemy {
                        enemy.health -= 100.0 * delta_time * windup_duration; // Deal damage to the enemy
                        enemy.poise -= 100.0 * delta_time * windup_duration; // Deal poise damage to the enemy
                        if enemy
                            .attack_controller
                            .make_staggered(windup_duration * 0.25)
                        {
                            audio_system
                                .play(&self.staggered_audio, self.rng.random_range(0.6..1.0));
                        }
                        if enemy.poise <= 0.0 {
                            enemy.poise = 50.0; // Prevent negative poise
                            enemy.attack_controller.make_staggered(1.0);
                            audio_system
                                .play(&self.stance_broken_audio, self.rng.random_range(0.6..1.0));
                        }
                        if enemy.health <= 0.0 {
                            enemy.health = 0.0; // Prevent negative health
                            info!("Enemy defeated!");
                            self.player.num_crystals += self.rng.random_range(10..=50);
                        }
                    }
                }
            }

            let player_event = self.player.update(input, delta_time, |player_space| {
                let mut collision_result = None;
                self.manager.get_current_room().spec.collides_with(
                    &level_origin,
                    player_space,
                    &mut |collision, id| {
                        if id == 1 {
                            collision_result = Some(collision);
                        }
                    },
                );
                collision_result
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

            // Level advancing:
            // collides with:
            // 2 -> move down
            // 3 -> move right
            // 4 -> move up
            // 5 -> move left
            let player_space = self.player.controller.collider(&level_origin);
            let mut collision_result = None;
            self.manager.get_current_room().spec.collides_with(
                &level_origin,
                &player_space,
                &mut |collision, id| {
                    if id == 2 || id == 3 || id == 4 || id == 5 {
                        collision_result = Some((collision, id));
                    }
                },
            );
            if let Some((collision, id)) = collision_result {
                let current_position = self.manager.current_room;
                let new_position = match id {
                    2 => (
                        current_position.0,
                        current_position.1 - 1,
                        current_position.2,
                    ), // Move down
                    3 => (
                        current_position.0 + 1,
                        current_position.1,
                        current_position.2,
                    ), // Move right
                    4 => (
                        current_position.0,
                        current_position.1 + 1,
                        current_position.2,
                    ), // Move up
                    5 => (
                        current_position.0 - 1,
                        current_position.1,
                        current_position.2,
                    ), // Move left
                    _ => current_position,
                };
                self.manager.change_room(new_position);
                info!("Changed room to: {:?}", new_position);
                // Move player position accordingly
                match id {
                    2 => self.player.controller.position.y = 1.0, // Move down
                    3 => self.player.controller.position.x = 1.25, // Move right
                    4 => self.player.controller.position.y = 14.5, // Move up
                    5 => self.player.controller.position.x = 14.75, // Move left
                    _ => {}
                }
            }
        }
    }

    pub fn render(&self, drawer: &mut Drawer) {
        drawer.clear_slow(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 255.0,
        });

        let view_transform = self.camera.get_transform().set_origin(
            &self
                .player
                .controller
                .local_space(&Transform::new().translate(Vec3::new(0.5, 0.5, 0.0))),
        );

        let current_level = self.manager.get_current_room();
        let level_transform = current_level.spec.get_local_space(
            &view_transform.set_origin(&Transform::new().translate(Vec3::new(0.0, 0.0, 0.0))),
        );
        drawer.draw_square_slow(
            Some(&level_transform),
            Some(&EngineColor::WHITE),
            current_level.spec.background.get_sprite([0, 0]).unwrap(),
        );
        drawer.draw_square_slow(
            Some(&level_transform),
            Some(&EngineColor::WHITE),
            current_level.spec.decoration.get_sprite([0, 0]).unwrap(),
        );

        // Draw enemies
        for enemy in &current_level.enemies {
            if enemy.health > 0.0 {
                let color = if let EnemyAIState::Chasing(_) = enemy.state {
                    EngineColor::RED
                } else {
                    EngineColor::BLUE
                };

                drawer.draw_square_slow(
                    Some(&enemy.controller.local_space(&view_transform)),
                    Some(&color),
                    enemy.animation.get_current_sprite(),
                );

                let white_sprite = drawer.white_sprite();

                if let Some((attack_space, _)) = enemy.get_attack_space(&view_transform) {
                    drawer.draw_square_slow(
                        Some(&attack_space),
                        Some(&EngineColor::GREEN),
                        white_sprite,
                    );
                }

                // Draw enemy health bar
                drawer.draw_square_slow(
                    Some(&enemy.health_bar_space(&view_transform, true)),
                    Some(&EngineColor::RED.additive_darken(0.7)),
                    white_sprite,
                );
                drawer.draw_square_slow(
                    Some(&enemy.health_bar_space(&view_transform, false)),
                    Some(&EngineColor::RED),
                    white_sprite,
                );

                // Draw enemy poise bar
                drawer.draw_square_slow(
                    Some(&enemy.poise_bar_space(&view_transform, true)),
                    Some(&EngineColor::YELLOW.additive_darken(0.7)),
                    white_sprite,
                );
                drawer.draw_square_slow(
                    Some(&enemy.poise_bar_space(&view_transform, false)),
                    Some(&EngineColor::YELLOW),
                    white_sprite,
                );
            }
        }

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

        if let Some((attack_space, _)) = self.player.get_attack_space(&view_transform) {
            drawer.draw_square_slow(Some(&attack_space), Some(&EngineColor::GREEN), white_sprite);
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

        // Render healing flasks
        let flask_index = ((self.player.max_healing_flasks - self.player.healing_flasks) * 4)
            / self.player.max_healing_flasks;
        let flask_sprite = self.ui_sheet_32.get_sprite([0, flask_index]).unwrap();
        drawer.draw_square_slow(
            Some(
                &ui_transform
                    .translate(Vec3::new(8.0, 240.0 - 32.0 - 8.0, 0.0))
                    .scale(Vec3::new(32.0, 32.0, 1.0)),
            ),
            Some(&EngineColor::WHITE),
            flask_sprite,
        );

        drawer.draw_text_slow(
            &self.num_flasks_text,
            8.0 + 32.0,
            240.0 - 16.0 - 8.0,
            1.0,
            GlyphonColor::rgba(255, 255, 255, 255),
        );

        // Render crystals
        let crystal_load = self.crystal_count_buffer.get_load();
        let crystal_index = (crystal_load as u32).min(4);
        let crystal_sprite = self.ui_sheet_16.get_sprite([2, crystal_index]).unwrap();
        drawer.draw_square_slow(
            Some(
                &ui_transform
                    .translate(Vec3::new(320.0 - 8.0 - 16.0, 8.0, 0.0))
                    .scale(Vec3::new(16.0, 16.0, 1.0)),
            ),
            Some(&EngineColor::WHITE),
            crystal_sprite,
        );
        drawer.draw_text_slow(
            &self.num_crystals_text,
            320.0 - 8.0 - 16.0 - 128.0,
            8.0 + 4.0,
            1.0,
            GlyphonColor::rgba(255, 255, 255, 255),
        );
    }
}
