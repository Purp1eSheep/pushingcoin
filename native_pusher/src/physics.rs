use glam::{Mat4, Vec3};
use rapier3d::prelude::*;

use crate::geometry::{COIN_HEIGHT, COIN_RADIUS};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CoinType {
    Gold,
    Silver,
    Ruby,
    Diamond,
}

impl CoinType {
    pub fn value(&self) -> u64 {
        match self {
            CoinType::Gold => 1,
            CoinType::Silver => 2,
            CoinType::Ruby => 5,
            CoinType::Diamond => 10,
        }
    }

    pub fn color(&self) -> [f32; 4] {
        match self {
            CoinType::Gold => [1.0, 0.82, 0.16, 1.0],      // 24K Polished Gold
            CoinType::Silver => [0.92, 0.94, 0.98, 1.0],    // Chrome Silver
            CoinType::Ruby => [1.0, 0.15, 0.32, 1.0],       // Glowing Crimson Ruby
            CoinType::Diamond => [0.15, 0.92, 1.0, 1.0],    // Electric Neon Cyan
        }
    }

    /// [metallic, roughness, emission, scale]
    pub fn extra(&self) -> [f32; 4] {
        match self {
            CoinType::Gold => [0.95, 0.22, 0.05, 1.0],
            CoinType::Silver => [1.0, 0.12, 0.05, 1.0],
            CoinType::Ruby => [0.2, 0.15, 0.75, 1.0],
            CoinType::Diamond => [0.3, 0.1, 0.85, 1.0],
        }
    }
}

pub struct CoinInstance {
    pub handle: RigidBodyHandle,
    pub coin_type: CoinType,
    pub color: [f32; 4],
    pub extra: [f32; 4],
}

pub struct ScoredCoinEvent {
    pub coin_type: CoinType,
    pub value: u64,
    pub position: Vec3,
    pub color: [f32; 4],
    pub is_side: bool,
}

pub struct PhysicsStepResult {
    pub scored_events: Vec<ScoredCoinEvent>,
    pub fever_started: bool,
}

pub struct PhysicsWorld {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub soft_body_set: SoftBodySet,
    pub ccd_solver: CCDSolver,
    pub physics_hooks: (),
    pub event_handler: (),

    pub pusher_handle: RigidBodyHandle,
    pub left_wall_handle: RigidBodyHandle,
    pub right_wall_handle: RigidBodyHandle,
    pub side_barriers_enabled: bool,
    pub coins: Vec<CoinInstance>,

    pub score: u64,
    pub balance: u64,
    pub combo: u32,
    pub last_score_time: f32,

    pub fever_meter: f32,
    pub is_fever: bool,
    pub fever_time_remaining: f32,

    pub time: f32,
    pub pusher_phase: f32,
    pub drop_stagger_idx: usize,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();

        // 1. Lower Deck (Z: -1.2 to 4.2, centered at Z = 1.5, Y = -0.2, depth 5.4)
        let deck_body = RigidBodyBuilder::fixed()
            .translation(Vec3::new(0.0, -0.2, 1.5))
            .build();
        let deck_handle = rigid_body_set.insert(deck_body);
        let deck_collider = ColliderBuilder::cuboid(3.2, 0.2, 2.7)
            .friction(0.40) // Smooth arcade metal friction: coins glide fluidly off the edge!
            .friction_combine_rule(CoefficientCombineRule::Average)
            .restitution(0.0)
            .build();
        collider_set.insert_with_parent(deck_collider, deck_handle, &mut rigid_body_set);

        // (Exit ledge/lip completely removed - exit is 100% flush so coins tumble freely!)


        // 2. Left Side Barrier (X = -3.24, Z = 1.5, Y = 0.22, height 0.44 matching visual barrier plate)
        let left_wall_body = RigidBodyBuilder::fixed()
            .translation(Vec3::new(-3.24, 0.22, 1.5))
            .build();
        let left_wall_handle = rigid_body_set.insert(left_wall_body);
        let left_wall_collider = ColliderBuilder::cuboid(0.03, 0.22, 2.7)
            .friction(0.05)
            .restitution(0.1)
            .build();
        collider_set.insert_with_parent(left_wall_collider, left_wall_handle, &mut rigid_body_set);

        // 3. Right Side Barrier (X = 3.24, Z = 1.5, Y = 0.22, height 0.44 matching visual barrier plate)
        let right_wall_body = RigidBodyBuilder::fixed()
            .translation(Vec3::new(3.24, 0.22, 1.5))
            .build();
        let right_wall_handle = rigid_body_set.insert(right_wall_body);
        let right_wall_collider = ColliderBuilder::cuboid(0.03, 0.22, 2.7)
            .friction(0.05)
            .restitution(0.1)
            .build();
        collider_set.insert_with_parent(right_wall_collider, right_wall_handle, &mut rigid_body_set);

        // 4. Back Wall (Z = -3.8)
        let back_wall_body = RigidBodyBuilder::fixed()
            .translation(Vec3::new(0.0, 2.6, -3.8))
            .build();
        let back_wall_handle = rigid_body_set.insert(back_wall_body);
        let back_wall_collider = ColliderBuilder::cuboid(3.8, 2.6, 0.2)
            .friction(0.05)
            .restitution(0.1)
            .build();
        collider_set.insert_with_parent(back_wall_collider, back_wall_handle, &mut rigid_body_set);

        // 5. Stationary Scraper Wall (Z = -1.1, zero-gap with raised pusher top Y=0.865)
        let scraper_body = RigidBodyBuilder::fixed()
            .translation(Vec3::new(0.0, 1.48, -1.1))
            .build();
        let scraper_handle = rigid_body_set.insert(scraper_body);
        let scraper_collider = ColliderBuilder::cuboid(2.75, 0.65, 0.08)
            .friction(0.05)
            .restitution(0.0)
            .build();
        collider_set.insert_with_parent(scraper_collider, scraper_handle, &mut rigid_body_set);

        // 6. Rear Containment Shelf (Z: -1.1 to -3.8, Y = 0.83)
        // Solid safety floor behind the scraper wall so nothing can ever fall into the machinery void!
        let rear_shelf_body = RigidBodyBuilder::fixed()
            .translation(Vec3::new(0.0, 0.83, -2.45))
            .build();
        let rear_shelf_handle = rigid_body_set.insert(rear_shelf_body);
        let rear_shelf_collider = ColliderBuilder::cuboid(2.75, 0.04, 1.35)
            .friction(0.05)
            .restitution(0.0)
            .build();
        collider_set.insert_with_parent(rear_shelf_collider, rear_shelf_handle, &mut rigid_body_set);

        // 7. Rear Machinery Roof (Z = -2.45, Y = 2.20)
        let roof_body = RigidBodyBuilder::fixed()
            .translation(Vec3::new(0.0, 2.20, -2.45))
            .build();
        let roof_handle = rigid_body_set.insert(roof_body);
        let roof_collider = ColliderBuilder::cuboid(2.75, 0.06, 1.35)
            .friction(0.05)
            .restitution(0.0)
            .build();
        collider_set.insert_with_parent(roof_collider, roof_handle, &mut rigid_body_set);

        // 8. Upper Side Guards (Zero-gap lateral containment behind scraper wall, Z: -3.8 to -1.1)
        let upper_left = RigidBodyBuilder::fixed()
            .translation(Vec3::new(-2.74, 1.48, -2.45))
            .build();
        let upper_left_h = rigid_body_set.insert(upper_left);
        let upper_left_col = ColliderBuilder::cuboid(0.08, 0.65, 1.35).build();
        collider_set.insert_with_parent(upper_left_col, upper_left_h, &mut rigid_body_set);

        let upper_right = RigidBodyBuilder::fixed()
            .translation(Vec3::new(2.74, 1.48, -2.45))
            .build();
        let upper_right_h = rigid_body_set.insert(upper_right);
        let upper_right_col = ColliderBuilder::cuboid(0.08, 0.65, 1.35).build();
        collider_set.insert_with_parent(upper_right_col, upper_right_h, &mut rigid_body_set);

        // 9. Front exit is completely open (no air wall, coins tumble freely off the cliff!)

        // 10. Kinematic Pusher (Balanced arcade reach & zero-gap design against lower deck + taller pusher height Y=0.865)
        let pusher_body = RigidBodyBuilder::kinematic_position_based()
            .translation(Vec3::new(0.0, 0.43, -0.45))
            .ccd_enabled(true)
            .build();
        let pusher_handle = rigid_body_set.insert(pusher_body);
        let pusher_collider = ColliderBuilder::cuboid(2.70, 0.44, 1.7)
            .translation(Vec3::new(0.0, -0.005, -0.6))
            .friction(0.35)
            .friction_combine_rule(CoefficientCombineRule::Max)
            .restitution(0.0)
            .build();
        collider_set.insert_with_parent(pusher_collider, pusher_handle, &mut rigid_body_set);

        let mut integration_parameters = IntegrationParameters::default();
        integration_parameters.dt = 1.0 / 60.0;

        let mut world = Self {
            rigid_body_set,
            collider_set,
            integration_parameters,
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            soft_body_set: SoftBodySet::new(),
            ccd_solver: CCDSolver::new(),
            physics_hooks: (),
            event_handler: (),
            pusher_handle,
            left_wall_handle,
            right_wall_handle,
            side_barriers_enabled: true,
            coins: Vec::with_capacity(5000),
            score: 0,
            balance: 50,
            combo: 0,
            last_score_time: 0.0,
            fever_meter: 0.0,
            is_fever: false,
            fever_time_remaining: 0.0,
            time: 0.0,
            pusher_phase: 0.0,
            drop_stagger_idx: 0,
        };

        world.prepopulate();
        world
    }

    pub fn prepopulate(&mut self) {
        // Pre-populate upper deck (flat, spaced out with sleek tokens)
        for i in 0..32 {
            let col = (i % 8) as f32 - 3.5;
            let row = (i / 8) as f32;
            let x = col * 0.62 + (rand::random::<f32>() - 0.5) * 0.06;
            let z = -0.92 + row * 0.35 + (rand::random::<f32>() - 0.5) * 0.05;
            let y = 0.90 + (i / 16) as f32 * 0.04;

            let ctype = if i % 10 == 0 {
                CoinType::Diamond
            } else if i % 7 == 0 {
                CoinType::Ruby
            } else if i % 3 == 0 {
                CoinType::Silver
            } else {
                CoinType::Gold
            };

            self.spawn_coin(x, y, z, ctype);
        }

        // Pre-populate lower deck runway (clean blanket spreading smoothly towards the cliff edge!)
        for i in 0..64 {
            let col = (i % 8) as f32 - 3.5;
            let row = (i / 8) as f32;
            let x = col * 0.72 + (rand::random::<f32>() - 0.5) * 0.05;
            let z = 0.80 + row * 0.44 + (rand::random::<f32>() - 0.5) * 0.04;
            let y = 0.03 + (i / 20) as f32 * 0.03;

            let ctype = if i % 12 == 0 {
                CoinType::Diamond
            } else if i % 8 == 0 {
                CoinType::Ruby
            } else if i % 4 == 0 {
                CoinType::Silver
            } else {
                CoinType::Gold
            };

            self.spawn_coin(x, y, z, ctype);
        }
    }

    pub fn spawn_coin(&mut self, x: f32, y: f32, z: f32, coin_type: CoinType) {
        self.spawn_coin_with_vel(x, y, z, Vec3::ZERO, coin_type);
    }

    fn spawn_coin_with_vel(&mut self, x: f32, y: f32, z: f32, vel: Vec3, coin_type: CoinType) {
        let radius = COIN_RADIUS;
        let height = COIN_HEIGHT;
        let mass = 2.2;

        let body = RigidBodyBuilder::dynamic()
            .translation(Vec3::new(x, y, z))
            .linvel(vel)
            .linear_damping(0.40)
            .angular_damping(0.55) // High angular damping keeps coins lying strictly flat without fluttering
            .can_sleep(true)
            .build();
        let handle = self.rigid_body_set.insert(body);

        let border_radius = 0.012;
        let half_height = (height * 0.5 - border_radius).max(0.003);
        let eff_radius = radius - border_radius;

        let collider = ColliderBuilder::round_cylinder(half_height, eff_radius, border_radius)
            .mass(mass) // Direct real solid metal mass & rotational inertia tensor!
            .friction(0.45) // High coin friction! Coins lock together, push cleanly, and hang over the cliff!
            .friction_combine_rule(CoefficientCombineRule::Max)
            .restitution(0.0) // Solid non-bouncy metal impact
            .build();
        self.collider_set.insert_with_parent(collider, handle, &mut self.rigid_body_set);

        self.coins.push(CoinInstance {
            handle,
            coin_type,
            color: coin_type.color(),
            extra: coin_type.extra(),
        });
    }

    pub fn drop_coin(&mut self, spout_x: f32, forced_type: Option<CoinType>) -> Option<CoinType> {
        if self.balance == 0 && !self.is_fever {
            return None;
        }

        if !self.is_fever {
            self.balance -= 1;
        }

        self.drop_stagger_idx = (self.drop_stagger_idx + 1) % 6;
        let x = spout_x + ((self.drop_stagger_idx % 3) as f32 - 1.0) * 0.10;
        let y = 2.6 + self.drop_stagger_idx as f32 * 0.08;
        let z = -0.68 + (rand::random::<f32>() - 0.5) * 0.15;

        let ctype = if let Some(t) = forced_type {
            t
        } else {
            let roll = rand::random::<f32>();
            if roll < 0.06 {
                CoinType::Diamond
            } else if roll < 0.22 {
                CoinType::Ruby
            } else if roll < 0.50 {
                CoinType::Silver
            } else {
                CoinType::Gold
            }
        };

        // Give dropped coin a smooth entry velocity from the chute!
        let initial_vel = Vec3::new(0.0, -5.0, 0.3);
        self.spawn_coin_with_vel(x, y, z, initial_vel, ctype);
        Some(ctype)
    }

    pub fn trigger_shake(&mut self) {
        let mass = 2.2;
        for coin in &self.coins {
            if let Some(body) = self.rigid_body_set.get_mut(coin.handle) {
                let jolt_y = (1.2 + rand::random::<f32>() * 0.8) * mass * 0.85;
                let jolt_z = (0.5 + rand::random::<f32>() * 0.5) * mass * 0.85;
                let jolt_x = (rand::random::<f32>() - 0.5) * 0.8 * mass;
                body.apply_impulse(Vec3::new(jolt_x, jolt_y, jolt_z), true);
            }
        }
    }

    pub fn refill(&mut self) {
        self.balance += 50;
    }

    /// Sets whether the side barriers are raised (ON) or lowered into slots (OFF)
    pub fn set_side_barriers(&mut self, enabled: bool) {
        self.side_barriers_enabled = enabled;
        let y_pos = if enabled { 0.22 } else { -20.0 };
        if let Some(body) = self.rigid_body_set.get_mut(self.left_wall_handle) {
            body.set_translation(Vec3::new(-3.24, y_pos, 1.5), true);
        }
        if let Some(body) = self.rigid_body_set.get_mut(self.right_wall_handle) {
            body.set_translation(Vec3::new(3.24, y_pos, 1.5), true);
        }
        // Wake up sleeping coins near borders so they immediately react to the raised/lowered walls
        for coin in &self.coins {
            if let Some(body) = self.rigid_body_set.get_mut(coin.handle) {
                body.wake_up(true);
            }
        }
    }

    /// Toggles the side barriers between ON and OFF
    pub fn toggle_side_barriers(&mut self) -> bool {
        let new_state = !self.side_barriers_enabled;
        self.set_side_barriers(new_state);
        new_state
    }

    pub fn step(&mut self, dt: f32) -> PhysicsStepResult {
        self.time += dt;

        // Fever countdown
        let mut fever_started = false;
        if self.is_fever {
            self.fever_time_remaining -= dt;
            if self.fever_time_remaining <= 0.0 {
                self.is_fever = false;
                self.fever_meter = 0.0;
            }
        }

        // Kinematic pusher oscillation along Z (Moderate balanced arcade stroke)
        let speed = if self.is_fever { 1.7 } else { 1.15 };
        let amplitude = 0.95;
        let base_z = -0.45;

        // Smooth continuous phase accumulation
        self.pusher_phase += speed * dt;
        let current_z = base_z + self.pusher_phase.sin() * amplitude;

        if let Some(pusher) = self.rigid_body_set.get_mut(self.pusher_handle) {
            pusher.set_next_kinematic_translation(Vec3::new(0.0, 0.43, current_z));
        }

        self.integration_parameters.dt = dt;
        let gravity = Vec3::new(0.0, -48.0, 0.0);

        self.physics_pipeline.step(
            gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.soft_body_set,
            &mut self.ccd_solver,
            &self.physics_hooks,
            &self.event_handler,
        );

        // Anti-standup stabilizer: ensures coins always lie flat on the deck and never stand upright on their rims
        // (Excluded when coins pass the edge or start tumbling down the cliff)
        for coin in &self.coins {
            if let Some(body) = self.rigid_body_set.get_mut(coin.handle) {
                if !body.is_sleeping() {
                    let pos = body.translation();
                    if pos.y >= -0.02 && pos.z < 4.18 {
                        let rot = *body.rotation();
                        let local_up = rot * Vec3::Y;
                        // If tilted more than 40 degrees from horizontal (i.e. trying to stand on edge)
                        if local_up.y.abs() < 0.76 {
                            let perp = Vec3::new(-local_up.z, 0.0, local_up.x);
                            let topple_axis = if perp.length_squared() > 1e-4 {
                                perp.normalize()
                            } else {
                                Vec3::X
                            };
                            body.apply_torque_impulse(topple_axis * 0.18, true);
                        }
                    }
                }
            }
        }

        // Check scoring coins:
        // Only score & remove when coin has physically fallen down into the lower collection chute (Y < -2.2)!
        // Coins will NOT disappear when pushed over the cliff — players watch them tumble all the way down!
        let mut scored_events = Vec::new();
        let is_fever = self.is_fever;
        let mut coins_to_remove = Vec::new();

        for (idx, coin) in self.coins.iter().enumerate() {
            if let Some(body) = self.rigid_body_set.get(coin.handle) {
                let pos = body.translation();
                if pos.y < -2.2 {
                    let base_val = coin.coin_type.value();
                    let mult = if is_fever { 5 } else { 1 };
                    let is_front = pos.z >= 4.0;
                    let is_side = pos.x.abs() > 3.0 && pos.z >= -1.0 && pos.z < 4.0;
                    if is_front || is_side {
                        let val = if is_side {
                            base_val // Side drop: consolation score
                        } else {
                            base_val * mult // Front drop: full winner payout
                        };
                        scored_events.push(ScoredCoinEvent {
                            coin_type: coin.coin_type,
                            value: val,
                            position: pos,
                            color: coin.color,
                            is_side,
                        });
                    }
                    coins_to_remove.push((idx, coin.handle));
                }
            } else {
                coins_to_remove.push((idx, coin.handle));
            }
        }

        // Remove from physics world and coin list
        if !coins_to_remove.is_empty() {
            let remove_set: std::collections::HashSet<RigidBodyHandle> =
                coins_to_remove.iter().map(|(_, h)| *h).collect();

            for (_, handle) in &coins_to_remove {
                self.rigid_body_set.remove(
                    *handle,
                    &mut self.island_manager,
                    &mut self.collider_set,
                    &mut self.impulse_joint_set,
                    &mut self.multibody_joint_set,
                    &mut self.soft_body_set,
                    true,
                );
            }

            self.coins.retain(|c| !remove_set.contains(&c.handle));
        }

        if !scored_events.is_empty() {
            let scored_total: u64 = scored_events.iter().map(|e| e.value).sum();
            self.score += scored_total;
            self.balance += scored_total; // Award free coins directly back to balance!

            if self.time - self.last_score_time < 1.6 {
                self.combo += scored_events.len() as u32;
            } else {
                self.combo = scored_events.len() as u32;
            }
            self.last_score_time = self.time;

            // Charge Fever meter (+4.5 per scored coin from front)
            if !self.is_fever {
                let front_coins = scored_events.iter().filter(|e| !e.is_side).count();
                self.fever_meter = (self.fever_meter + front_coins as f32 * 4.5).min(100.0);
                if self.fever_meter >= 100.0 {
                    self.is_fever = true;
                    self.fever_time_remaining = 10.0;
                    fever_started = true;
                }
            }
        } else if self.time - self.last_score_time > 1.6 {
            self.combo = 0;
        }

        PhysicsStepResult {
            scored_events,
            fever_started,
        }
    }

    /// Extract model matrices, colors, and material attributes for GPU Instanced rendering in 1 draw call!
    pub fn get_coin_transforms(&self) -> Vec<(Mat4, [f32; 4], [f32; 4])> {
        let mut transforms = Vec::with_capacity(self.coins.len());
        for coin in &self.coins {
            if let Some(body) = self.rigid_body_set.get(coin.handle) {
                let p = body.translation();
                let r = body.rotation();
                let translation = Vec3::new(p.x, p.y, p.z);
                let rotation = *r;
                let mat = Mat4::from_scale_rotation_translation(Vec3::ONE, rotation, translation);
                transforms.push((mat, coin.color, coin.extra));
            }
        }
        transforms
    }

    pub fn get_pusher_transform(&self) -> Mat4 {
        if let Some(pusher) = self.rigid_body_set.get(self.pusher_handle) {
            let p = pusher.translation();
            Mat4::from_translation(Vec3::new(p.x, p.y, p.z))
        } else {
            Mat4::IDENTITY
        }
    }
}
