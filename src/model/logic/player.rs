use super::*;

impl Player {
    fn update_timers(&mut self, delta_time: Time) {
        // Coyote Time
        if let Some((_, time)) = &mut self.coyote_time {
            *time -= delta_time;
            if *time <= Time::ZERO {
                self.coyote_time = None;
            }
        }

        // Jump Buffer
        if let Some(time) = &mut self.jump_buffer {
            *time -= delta_time;
            if *time <= Time::ZERO {
                self.jump_buffer = None;
            }
        }

        // Drill Dash
        if let PlayerState::AirDrill { dash } = &mut self.state {
            if let Some(time) = dash {
                *time -= delta_time;
                if *time <= Time::ZERO {
                    *dash = None;
                }
            }
        }

        // Controll timeout
        if let Some(time) = &mut self.control_timeout {
            // No horizontal control
            *time -= delta_time;
            if *time <= Time::ZERO {
                self.control_timeout = None;
            }
        }
    }
}

impl Logic<'_> {
    pub fn process_player(&mut self) {
        if !matches!(self.world.player.state, PlayerState::Drilling) {
            if let Some(mut sound) = self.world.drill_sound.take() {
                sound.stop();
            }
        }

        self.world.player.update_timers(self.delta_time);

        // Drill Dash Cancel
        self.drill_dash_cancel();

        // Update Jump Buffer
        if self.player_control.jump {
            self.world.player.jump_buffer = Some(self.world.rules.jump_buffer_time);
        }

        // Update Jump Hold
        if self.world.player.can_hold_jump && !self.player_control.hold_jump {
            self.world.player.can_hold_jump = false;
        }

        // Pause states
        if self.pause_state() {
            return;
        }

        self.restore_drill_dash();
        self.drill_dash();

        // Update look direction
        let player = &mut self.world.player;
        if player.facing_left && player.velocity.x > Coord::ZERO
            || !player.facing_left && player.velocity.x < Coord::ZERO
        {
            player.facing_left = !player.facing_left;
        }

        // Drill or Drill Dash - no control or gravity
        if !matches!(
            self.world.player.state,
            PlayerState::Drilling | PlayerState::AirDrill { dash: Some(_) }
        ) {
            // Apply gravity
            self.world.player.velocity += self.world.rules.gravity * self.delta_time;

            self.variable_jump();
            self.horizontal_control();
            self.jump();
        } else if self.world.player.state.is_drilling()
            && self.player_control.move_dir != vec2::ZERO
        {
            if let Some((Coyote::DrillDirection { initial }, _)) = self.world.player.coyote_time {
                // Change drill direction
                if vec2::dot(self.player_control.move_dir, initial) >= Coord::ZERO {
                    self.world.player.velocity = self.player_control.move_dir.normalize_or_zero()
                        * self.world.player.velocity.len();
                    self.world.player.coyote_time = None;
                }
            }
        }

        self.world
            .player
            .collider
            .translate(self.world.player.velocity * self.delta_time);
    }

    pub fn player_collisions(&mut self) {
        if let PlayerState::Respawning { .. } = self.world.player.state {
            return;
        }

        let finished = self.world.player.state.finished_state();
        let can_drill = self.player_tiles();

        // Level bounds
        if self.level_bounds() {
            return;
        }

        // Stay in finish state
        if let Some(state) = finished {
            self.world.player.state = state;
            return;
        }

        self.update_drill_state(can_drill);

        self.player_coins();

        // Finish
        if self.check_finish() {
            return;
        }

        self.player_hazards();
    }

    fn pause_state(&mut self) -> bool {
        match &mut self.world.player.state {
            PlayerState::Respawning { time } => {
                *time -= self.delta_time;
                if *time <= Time::ZERO {
                    // Respawn
                    self.world.player.state = PlayerState::Airborn;
                    self.world.player.velocity = vec2::ZERO;
                    self.world
                        .player
                        .collider
                        .teleport(self.world.level.spawn_point);
                }
                true
            }
            PlayerState::Finished { time, next_heart } => {
                *time -= self.delta_time;
                if *time <= Time::ZERO {
                    // Level transition
                    self.next_level();
                    return true;
                }
                *next_heart -= self.delta_time;
                if *next_heart <= Time::ZERO {
                    *next_heart += Time::new(0.5);
                    self.world.particles.push(Particle {
                        initial_lifetime: Time::new(2.0),
                        lifetime: Time::new(2.0),
                        position: self.world.level.finish
                            + vec2(Coord::ZERO, self.world.player.collider.raw().height()),
                        velocity: vec2(0.0, 1.5)
                            .rotate(thread_rng().gen_range(-0.5..=0.5))
                            .map(Coord::new),
                        particle_type: ParticleType::Heart4,
                    });
                }
                self.world.player.velocity += self.world.rules.gravity * self.delta_time;
                self.world.player.velocity.x = Coord::ZERO;
                self.world
                    .player
                    .collider
                    .translate(self.world.player.velocity * self.delta_time);
                true
            }
            _ => false,
        }
    }

    fn drill_dash(&mut self) {
        // Drill Dash
        if let PlayerState::Drilling = self.world.player.state {
            self.world.player.can_drill_dash = false;
            return;
        }

        if !self.world.level.drill_allowed
            || !self.player_control.drill
            || matches!(self.world.player.state, PlayerState::AirDrill { .. })
        {
            return;
        }

        let mut dash = None;
        let dir = self.player_control.move_dir;
        if self.world.rules.can_drill_dash && self.world.player.can_drill_dash && dir != vec2::ZERO
        {
            // Dash
            let dir = dir.normalize_or_zero();
            let vel_dir = self.world.player.velocity.normalize_or_zero();
            let rules = &self.world.rules;
            // let acceleration = rules.drill_dash_speed_inc;
            // let speed = self.world.player.velocity.len();
            // let angle = Coord::new(vec2::dot(vel_dir, dir).as_f32().acos() / 2.0);
            // let current = speed * angle.cos();
            // let speed = (current + acceleration).max(rules.drill_dash_speed_min);
            let speed = rules.drill_dash_speed_min;
            let mut target = dir * speed;

            let real = self.world.player.velocity;
            if target.x != Coord::ZERO
                && target.x.signum() == real.x.signum()
                && real.x.abs() > target.x.abs()
            {
                target.x = real.x;
            }
            if target.y != Coord::ZERO
                && target.y.signum() == real.y.signum()
                && real.y.abs() > target.y.abs()
            {
                target.y = real.y;
            }

            self.world.player.velocity = target;
            self.world.player.can_drill_dash = false;
            dash = Some(self.world.rules.drill_dash_time);

            self.spawn_particles(ParticleSpawn {
                lifetime: Time::ONE,
                position: self.world.player.collider.pos(),
                velocity: -vel_dir * Coord::new(0.5),
                amount: 5,
                color: Rgba::opaque(0.8, 0.25, 0.2),
                radius: Coord::new(0.2),
                ..Default::default()
            });
        } else if !matches!(self.world.player.state, PlayerState::Drilling)
            && self.player_control.drill
            && self.world.level.drill_allowed
        {
            let dirs = itertools::chain![
                match self.world.player.state {
                    PlayerState::Grounded(tile) if tile.is_drillable() =>
                        Some(vec2(0.0, -1.0).map(Coord::new)),
                    PlayerState::WallSliding { tile, wall_normal } if tile.is_drillable() =>
                        Some(-wall_normal),
                    _ => None,
                },
                self.world
                    .player
                    .touching_wall
                    .and_then(|(tile, normal)| tile.is_drillable().then_some(-normal))
            ];
            for drill_dir in dirs {
                if vec2::dot(self.player_control.move_dir, drill_dir) > Coord::ZERO {
                    self.world.player.velocity = self.player_control.move_dir.normalize_or_zero()
                        * self.world.rules.drill_speed_min;
                }
            }
        }

        // Turn into a drill
        self.world.player.state = PlayerState::AirDrill { dash };
    }

    fn drill_dash_cancel(&mut self) {
        let PlayerState::AirDrill { dash: None } = &mut self.world.player.state else {
            // Cannot cancel yet
            return;
        };

        if self.player_control.hold_drill {
            // Input holds dash
            return;
        }

        // Turn back from drill
        let mut player = &mut self.world.player;
        player.state = PlayerState::Airborn;

        if player.drill_release.take().is_some() {
            // No slow-down
            return;
        }

        // Slow-down punishment
        let spawn = ParticleSpawn {
            lifetime: Time::new(0.3),
            position: player.collider.pos(),
            velocity: player.velocity,
            amount: 5,
            color: Rgba::opaque(0.6, 0.6, 0.6),
            radius: Coord::new(0.4),
            angle_range: Coord::new(-0.1)..=Coord::new(0.1),
            ..Default::default()
        };
        player.velocity.x = player.velocity.x.clamp_abs(self.world.rules.move_speed);
        self.spawn_particles(spawn);
    }

    fn restore_drill_dash(&mut self) {
        // Restore Drill Dash
        // Spawn particles on walk/wallslide
        match self.world.player.state {
            PlayerState::Grounded(..) => {
                self.world.player.can_drill_dash = true;
                if self.world.player.velocity.x.abs() > Coord::new(0.1)
                    && thread_rng().gen_bool(0.1)
                {
                    self.spawn_particles(ParticleSpawn {
                        lifetime: Time::ONE,
                        position: self.world.player.collider.feet(),
                        velocity: vec2(self.world.player.velocity.x.signum(), Coord::ONE)
                            * Coord::new(0.5),
                        amount: 2,
                        color: Rgba::opaque(0.8, 0.8, 0.8),
                        radius: Coord::new(0.1),
                        ..Default::default()
                    });
                }
            }
            PlayerState::WallSliding { wall_normal, .. } => {
                self.world.player.can_drill_dash = true;
                if self.world.player.velocity.y < Coord::new(-0.1) && thread_rng().gen_bool(0.1) {
                    self.spawn_particles(ParticleSpawn {
                        lifetime: Time::ONE,
                        position: self.world.player.collider.pos()
                            - wall_normal
                                * self.world.player.collider.raw().width()
                                * Coord::new(0.5),
                        velocity: vec2(wall_normal.x * Coord::new(0.2), Coord::ONE)
                            * Coord::new(0.5),
                        amount: 2,
                        color: Rgba::opaque(0.8, 0.8, 0.8),
                        radius: Coord::new(0.1),
                        ..Default::default()
                    });
                }
            }
            _ => (),
        }
    }

    fn variable_jump(&mut self) {
        if matches!(self.world.player.state, PlayerState::AirDrill { .. }) {
            return;
        }

        // Variable jump height
        if self.world.player.velocity.y < Coord::ZERO {
            // Faster drop
            self.world.player.velocity.y += self.world.rules.gravity.y
                * (self.world.rules.fall_multiplier - Coord::ONE)
                * self.delta_time;
            let cap = match self.world.player.state {
                PlayerState::WallSliding { .. } => self.world.rules.wall_slide_speed,
                _ => self.world.rules.free_fall_speed,
            };
            self.world.player.velocity.y = self.world.player.velocity.y.clamp_abs(cap);
        } else if self.world.player.velocity.y > Coord::ZERO
            && !(self.player_control.hold_jump && self.world.player.can_hold_jump)
        {
            // Low jump
            self.world.player.velocity.y += self.world.rules.gravity.y
                * (self.world.rules.low_jump_multiplier - Coord::ONE)
                * self.delta_time;
        }
    }

    fn horizontal_control(&mut self) {
        if self.world.player.control_timeout.is_some()
            || matches!(self.world.player.state, PlayerState::AirDrill { .. })
        {
            return;
        }

        // Horizontal speed control
        let target = self.player_control.move_dir.x * self.world.rules.move_speed;
        let acc = if self.world.player.velocity.x.abs() > self.world.rules.move_speed {
            self.world.rules.low_control_acc
        } else {
            self.world.rules.full_control_acc
        };
        let current = self.world.player.velocity.x;

        // If target speed is aligned with velocity, then do not slow down
        if target == Coord::ZERO
            || target.signum() != current.signum()
            || target.abs() > current.abs()
        {
            self.world.player.velocity.x += (target - current).clamp_abs(acc * self.delta_time);
        }
    }

    fn jump(&mut self) {
        if self.world.player.jump_buffer.is_none() {
            return;
        }

        // Try jump
        let rules = &self.world.rules;
        let jump = match self.world.player.state {
            PlayerState::Grounded { .. } => Some(Coyote::Ground),
            PlayerState::WallSliding { wall_normal, .. } => Some(Coyote::Wall { wall_normal }),
            PlayerState::Airborn | PlayerState::AirDrill { .. } => {
                self.world.player.coyote_time.map(|(coyote, _)| coyote)
            }
            _ => None,
        };
        let Some(jump) = jump else { return };

        // Use jump
        self.world.player.coyote_time = None;
        self.world.player.jump_buffer = None;
        self.world.player.can_hold_jump = true;
        match jump {
            Coyote::Ground => {
                let jump_vel = rules.normal_jump_strength;
                self.world.player.velocity.y = jump_vel;
                self.world.player.state = PlayerState::Airborn;
                self.world.play_sound(&self.world.assets.sounds.jump);
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: self.world.player.collider.feet(),
                    velocity: vec2(Coord::ZERO, Coord::ONE),
                    amount: 3,
                    color: Rgba::WHITE,
                    radius: Coord::new(0.1),
                    ..Default::default()
                });
            }
            Coyote::Wall { wall_normal } => {
                let angle = rules.wall_jump_angle * wall_normal.x.signum();
                let mut jump_vel = wall_normal.rotate(angle) * rules.wall_jump_strength;
                let player = &mut self.world.player;
                jump_vel.y = jump_vel.y.max(player.velocity.y);
                player.velocity = jump_vel;
                player.control_timeout = Some(self.world.rules.wall_jump_timeout);
                player.state = PlayerState::Airborn;
                self.world.play_sound(&self.world.assets.sounds.jump);
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: self.world.player.collider.feet()
                        - wall_normal * self.world.player.collider.raw().width() * Coord::new(0.5),
                    velocity: jump_vel.normalize_or_zero(),
                    amount: 3,
                    color: Rgba::WHITE,
                    radius: Coord::new(0.1),
                    ..Default::default()
                });
            }
            Coyote::DrillJump { direction } => {
                let rules = &self.world.rules;
                let acceleration = rules.drill_jump_speed_inc;
                let current = vec2::dot(self.world.player.velocity, direction);
                self.world.player.velocity =
                    direction * (current + acceleration).max(rules.drill_jump_speed_min);
                self.world.play_sound(&self.world.assets.sounds.drill_jump);
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: self.world.player.collider.pos(),
                    velocity: direction,
                    amount: 5,
                    color: Rgba::opaque(0.8, 0.25, 0.2),
                    radius: Coord::new(0.3),
                    ..Default::default()
                });
            }
            Coyote::DrillDirection { .. } => {}
        }
    }

    fn player_tiles(&mut self) -> bool {
        let player = &mut self.world.player;
        let was_grounded = player.state.is_grounded();
        let wall_sliding = player.state.is_wall_sliding();
        let has_finished = player.state.has_finished();
        let using_drill = player.state.using_drill();
        let update_state = !using_drill;

        if update_state {
            player.state = PlayerState::Airborn;
        }

        let mut particles = Vec::new();
        let mut can_drill = false;
        player.touching_wall = None;

        for _ in 0..2 {
            // Player-tiles
            let player_aabb = player.collider.grid_aabb(&self.world.level.grid);
            let collisions = (player_aabb.min.x..=player_aabb.max.x)
                .flat_map(move |x| (player_aabb.min.y..=player_aabb.max.y).map(move |y| vec2(x, y)))
                .filter_map(|pos| {
                    self.world
                        .level
                        .tiles
                        .get_tile_isize(pos)
                        .filter(|tile| {
                            let air = matches!(tile, Tile::Air);
                            let drill = using_drill && tile.is_drillable();
                            if !air && drill {
                                can_drill = true;
                            }
                            !air && !drill
                        })
                        .and_then(|tile| {
                            let collider = Collider::new(
                                Aabb2::point(self.world.level.grid.grid_to_world(pos))
                                    .extend_positive(self.world.level.grid.cell_size),
                            );
                            player.collider.check(&collider).and_then(|collision| {
                                (vec2::dot(collision.normal, player.velocity) >= Coord::ZERO)
                                    .then_some((tile, collision))
                            })
                        })
                });
            if let Some((tile, collision)) =
                collisions.max_by_key(|(_, collision)| collision.penetration)
            {
                player
                    .collider
                    .translate(-collision.normal * collision.penetration);
                let bounciness = Coord::new(if using_drill { 1.0 } else { 0.0 });
                player.velocity -= collision.normal
                    * vec2::dot(player.velocity, collision.normal)
                    * (Coord::ONE + bounciness);
                if !using_drill {
                    if collision.normal.x.approx_eq(&Coord::ZERO)
                        && collision.normal.y < Coord::ZERO
                    {
                        if !was_grounded && !has_finished {
                            particles.push(ParticleSpawn {
                                lifetime: Time::ONE,
                                position: player.collider.feet(),
                                velocity: vec2(Coord::ZERO, Coord::ONE) * Coord::new(0.5),
                                amount: 3,
                                color: Rgba::WHITE,
                                radius: Coord::new(0.1),
                                ..Default::default()
                            });
                        }
                        if update_state {
                            player.state = PlayerState::Grounded(tile);
                            player.coyote_time =
                                Some((Coyote::Ground, self.world.rules.coyote_time));
                        }
                    } else if collision.normal.y.approx_eq(&Coord::ZERO) {
                        let wall_normal = -collision.normal;
                        player.touching_wall = Some((tile, wall_normal));
                        if update_state {
                            if !wall_sliding {
                                player.velocity.y = player.velocity.y.max(Coord::ZERO);
                            }
                            player.state = PlayerState::WallSliding { tile, wall_normal };
                            player.coyote_time =
                                Some((Coyote::Wall { wall_normal }, self.world.rules.coyote_time));
                        }
                    }
                }
            }
        }

        for spawn in particles {
            self.spawn_particles(spawn);
        }

        can_drill
    }

    fn update_drill_state(&mut self, can_drill: bool) {
        if self.world.player.state.is_drilling() {
            if !can_drill {
                // Exited the ground in drill mode
                self.world.player.can_drill_dash = true;
                self.world.player.state = if self.player_control.hold_drill {
                    self.world.player.drill_release = Some(self.world.rules.drill_release_time);
                    PlayerState::AirDrill { dash: None }
                } else {
                    PlayerState::Airborn
                };

                let direction = self.world.player.velocity.normalize_or_zero();
                self.world.player.coyote_time = Some((
                    Coyote::DrillJump { direction },
                    self.world.rules.coyote_time,
                ));
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: self.world.player.collider.pos(),
                    velocity: direction * Coord::new(0.3),
                    amount: 8,
                    color: Rgba::opaque(0.7, 0.7, 0.7),
                    radius: Coord::new(0.2),
                    ..Default::default()
                });
            } else if thread_rng().gen_bool(0.2) {
                // Drilling through the ground
                self.spawn_particles(ParticleSpawn {
                    lifetime: Time::ONE,
                    position: self.world.player.collider.pos(),
                    velocity: -self.world.player.velocity.normalize_or_zero() * Coord::new(0.5),
                    amount: 2,
                    color: Rgba::opaque(0.8, 0.8, 0.8),
                    radius: Coord::new(0.1),
                    ..Default::default()
                });
            }
        } else if self.world.player.state.is_air_drilling() && can_drill {
            // Entered the ground in drill mode
            let speed = self.world.player.velocity.len();
            let dir = self.world.player.velocity.normalize_or_zero();

            self.world.player.coyote_time = Some((
                Coyote::DrillDirection { initial: dir },
                self.world.rules.coyote_time,
            ));
            self.world.player.velocity = dir * speed.max(self.world.rules.drill_speed_min);
            self.world.player.state = PlayerState::Drilling;

            self.spawn_particles(ParticleSpawn {
                lifetime: Time::ONE,
                position: self.world.player.collider.pos(),
                velocity: -dir * Coord::new(0.3),
                amount: 5,
                color: Rgba::opaque(0.7, 0.7, 0.7),
                radius: Coord::new(0.2),
                ..Default::default()
            });

            let sound = self
                .world
                .drill_sound
                .get_or_insert_with(|| self.world.assets.sounds.drill.play());
            sound.set_volume(self.world.volume);
        }
    }

    fn check_finish(&mut self) -> bool {
        if self.world.player.state.is_drilling()
            || self.world.player.state.has_finished()
            || self
                .world
                .player
                .collider
                .check(&self.world.level.finish())
                .is_none()
        {
            return false;
        }

        self.world.player.state = PlayerState::Finished {
            time: Time::new(2.0),
            next_heart: Time::new(0.5),
        };
        self.world.particles.push(Particle {
            initial_lifetime: Time::new(2.0),
            lifetime: Time::new(2.0),
            position: self.world.player.collider.head()
                + vec2(Coord::ZERO, self.world.player.collider.raw().height()),
            velocity: vec2(0.0, 1.5).map(Coord::new),
            particle_type: ParticleType::Heart8,
        });
        self.world.play_sound(&self.world.assets.sounds.charm);

        true
    }

    fn player_coins(&mut self) {
        // Collect coins
        let mut collected = None;
        for coin in &mut self.world.level.coins {
            if !coin.collected && self.world.player.collider.check(&coin.collider).is_some() {
                self.world.coins_collected += 1;
                coin.collected = true;
                collected = Some(coin.collider.pos());
            }
        }
        self.world.level.coins.retain(|coin| !coin.collected);
        if let Some(position) = collected {
            self.world.play_sound(&self.world.assets.sounds.coin);
            self.spawn_particles(ParticleSpawn {
                lifetime: Time::ONE,
                position,
                velocity: vec2(Coord::ZERO, Coord::ONE) * Coord::new(0.5),
                amount: 5,
                color: Rgba::try_from("#e3a912").unwrap(),
                radius: Coord::new(0.2),
                ..Default::default()
            });
        }
    }

    fn player_hazards(&mut self) {
        // Die from hazards
        for hazard in &self.world.level.hazards {
            if self.world.player.collider.check(&hazard.collider).is_some()
                && hazard.direction.map_or(true, |dir| {
                    vec2::dot(self.world.player.velocity, dir) <= Coord::ZERO
                })
            {
                self.world.kill_player();
                break;
            }
        }
    }

    fn level_bounds(&mut self) -> bool {
        let level = &self.world.level;
        let level_bounds = level.bounds();
        let player = &mut self.world.player;

        // Top
        if player.collider.head().y > level_bounds.max.y {
            player.collider.translate(vec2(
                Coord::ZERO,
                level_bounds.max.y - player.collider.head().y,
            ));
            player.velocity.y = if player.state.is_drilling() {
                -player.velocity.y
            } else {
                Coord::ZERO
            };
        }

        // Horizontal
        let offset = player.collider.feet().x - level_bounds.center().x;
        if offset.abs() > level_bounds.width() / Coord::new(2.0) {
            player.collider.translate(vec2(
                offset.signum() * (level_bounds.width() / Coord::new(2.0) - offset.abs()),
                Coord::ZERO,
            ));
            player.velocity.x = Coord::ZERO;
        }

        // Bottom
        let player = &mut self.world.player;
        if player.collider.feet().y < level_bounds.min.y {
            self.world.kill_player();
            return true;
        }

        false
    }
}
