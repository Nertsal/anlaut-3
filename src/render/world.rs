use super::*;

pub struct WorldRender {
    geng: Geng,
    assets: Rc<Assets>,
}

impl WorldRender {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
        }
    }

    pub fn draw_world(
        &self,
        world: &World,
        draw_hitboxes: bool,
        framebuffer: &mut ugli::Framebuffer,
        _normal_framebuffer: Option<&mut ugli::Framebuffer>,
    ) {
        // TODO: normals
        self.draw_background(world, framebuffer);
        self.draw_level(
            &world.level,
            &world.geometry.0,
            &world.geometry.1,
            draw_hitboxes,
            &world.camera,
            framebuffer,
        );
        self.draw_player(&world.player, draw_hitboxes, &world.camera, framebuffer);
        self.draw_particles(&world.particles, &world.camera, framebuffer);
    }

    pub fn draw_background(&self, world: &World, framebuffer: &mut ugli::Framebuffer) {
        let texture = &self.assets.sprites.background;
        let size = texture.size().map(|x| x as f32 / PIXELS_PER_UNIT) / vec2(1.0, 4.0);
        let bounds = world.level.bounds().map(Coord::as_f32);
        let texture_bounds = bounds.extend_positive(vec2(0.5, 0.0) - size);
        let camera_bounds = world.camera_bounds().map(Coord::as_f32);

        // Parallax background
        for i in (0..4).rev() {
            let geometry = get_tile_uv(i, vec2(1, 4));
            let vertices = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
            let vertices = [0, 1, 2, 3].map(|i| Vertex {
                a_pos: vec2(vertices[i].0, vertices[i].1),
                a_uv: geometry[i],
            });
            let geometry = [
                vertices[0],
                vertices[1],
                vertices[2],
                vertices[0],
                vertices[2],
                vertices[3],
            ];

            let move_speed = 1.0 + (i as f32 / 3.0) * 0.1;
            let mut pos =
                (world.camera.center - camera_bounds.bottom_left()) / camera_bounds.size();
            if camera_bounds.width().approx_eq(&0.0) {
                pos.x = 0.0;
            }
            if camera_bounds.height().approx_eq(&0.0) {
                pos.y = 0.0;
            }
            let pos = (texture_bounds.size() * pos - vec2(0.5, 0.5)) * move_speed;
            let mut pos = texture_bounds.bottom_left() + pos;
            let move_speed = (1.0 - i as f32 / 3.0) * 0.1;
            pos.x = world.camera.center.x * move_speed;
            pos.x = world.camera.center.x + pos.x - (pos.x / size.x + 0.5).floor() * size.x;
            let pos = pixel_perfect_pos(pos.map(Coord::new));

            let geometry = itertools::chain![
                geometry.iter().map(|&(mut v)| {
                    v.a_pos -= vec2(1.0, 0.0);
                    v
                }),
                geometry.iter().map(|&(mut v)| {
                    v.a_pos += vec2(1.0, 0.0);
                    v
                }),
                geometry,
            ]
            .collect();

            let matrix = mat3::translate(pos) * mat3::scale(size);
            let geometry = ugli::VertexBuffer::new_dynamic(self.geng.ugli(), geometry);
            ugli::draw(
                framebuffer,
                &self.assets.shaders.texture,
                ugli::DrawMode::Triangles,
                &geometry,
                (
                    ugli::uniforms! {
                        u_model_matrix: matrix,
                        u_texture: texture,
                    },
                    geng::camera2d_uniforms(&world.camera, framebuffer.size().map(|x| x as f32)),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::default()),
                    ..Default::default()
                },
            );
        }

        let texture = &self.assets.sprites.sun;
        let size = texture.size().map(|x| x as f32 / PIXELS_PER_UNIT);
        let move_speed = 0.9;
        let mut pos = bounds.bottom_left();
        pos += (world.camera.center - camera_bounds.bottom_left()) * move_speed;
        let matrix = mat3::translate(pos) * mat3::scale(size);
        let vertices = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let vertices = vertices.map(|(x, y)| Vertex {
            a_pos: vec2(x, y),
            a_uv: vec2(x, y),
        });
        let geometry = ugli::VertexBuffer::new_dynamic(self.geng.ugli(), vertices.to_vec());
        ugli::draw(
            framebuffer,
            &self.assets.shaders.texture,
            ugli::DrawMode::TriangleFan,
            &geometry,
            (
                ugli::uniforms! {
                    u_model_matrix: matrix,
                    u_texture: texture,
                },
                geng::camera2d_uniforms(&world.camera, framebuffer.size().map(|x| x as f32)),
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::default()),
                ..Default::default()
            },
        );
    }

    pub fn draw_level(
        &self,
        level: &Level,
        tiles_geometry: &HashMap<Tile, ugli::VertexBuffer<Vertex>>,
        masked_geometry: &HashMap<Tile, ugli::VertexBuffer<MaskedVertex>>,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.draw_props(&level.props, camera, framebuffer);
        self.draw_tiles(tiles_geometry, masked_geometry, camera, framebuffer);
        self.draw_hazards(&level.hazards, draw_hitboxes, camera, framebuffer);
        self.draw_coins(&level.coins, draw_hitboxes, camera, framebuffer);

        // Finish
        let finish = level.finish().raw().map(Coord::as_f32);
        self.geng.draw_2d(
            framebuffer,
            camera,
            &draw_2d::TexturedQuad::new(
                Aabb2::ZERO
                    .extend_symmetric(finish.size() / 2.0 * vec2(-1.0, 1.0))
                    .translate(finish.center()),
                &self.assets.sprites.partner,
            ),
        );
        if draw_hitboxes {
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::Quad::new(finish, Rgba::new(0.0, 0.0, 1.0, 0.9)),
            );
        }
    }

    pub fn draw_level_editor(
        &self,
        level: &Level,
        tiles_geometry: &HashMap<Tile, ugli::VertexBuffer<Vertex>>,
        masked_geometry: &HashMap<Tile, ugli::VertexBuffer<MaskedVertex>>,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.draw_level(
            level,
            tiles_geometry,
            masked_geometry,
            draw_hitboxes,
            camera,
            framebuffer,
        );

        // Spawnpoint
        self.geng.draw_2d(
            framebuffer,
            camera,
            &draw_2d::Quad::new(
                Player::new(level.spawn_point)
                    .collider
                    .raw()
                    .map(Coord::as_f32),
                Rgba::new(0.0, 1.0, 0.0, 0.5),
            ),
        );

        // Spotlights
        for spotlight in &level.spotlights {
            let pos = pixel_perfect_pos(spotlight.position);
            let size = vec2(1.0, 1.0);
            let aabb = Aabb2::point(pos).extend_symmetric(size / 2.0);
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(aabb, &self.assets.sprites.spotlight),
            );
        }
    }

    pub fn draw_tiles(
        &self,
        tiles_geometry: &HashMap<Tile, ugli::VertexBuffer<Vertex>>,
        masked_geometry: &HashMap<Tile, ugli::VertexBuffer<MaskedVertex>>,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let mask = self.assets.sprites.tiles.mask.texture();
        for (tile, geometry) in masked_geometry {
            let set = self.assets.sprites.tiles.get_tile_set(tile);
            let texture = set.texture();
            ugli::draw(
                framebuffer,
                &self.assets.shaders.texture_mask,
                ugli::DrawMode::Triangles,
                geometry,
                (
                    ugli::uniforms! {
                        u_model_matrix: mat3::identity(),
                        u_texture: texture,
                        u_mask: mask,
                    },
                    geng::camera2d_uniforms(camera, framebuffer.size().map(|x| x as f32)),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::default()),
                    ..Default::default()
                },
            );
        }
        for (tile, geometry) in tiles_geometry {
            let set = self.assets.sprites.tiles.get_tile_set(tile);
            let texture = set.texture();
            ugli::draw(
                framebuffer,
                &self.assets.shaders.texture,
                ugli::DrawMode::Triangles,
                geometry,
                (
                    ugli::uniforms! {
                        u_model_matrix: mat3::identity(),
                        u_texture: texture,
                    },
                    geng::camera2d_uniforms(camera, framebuffer.size().map(|x| x as f32)),
                ),
                ugli::DrawParameters {
                    blend_mode: Some(ugli::BlendMode::default()),
                    ..Default::default()
                },
            );
        }
    }

    pub fn draw_props(
        &self,
        props: &[Prop],
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for prop in props {
            let texture = self.assets.sprites.props.get_texture(&prop.prop_type);
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(prop.sprite.map(Coord::as_f32), texture),
            );
        }
    }

    pub fn draw_hazards(
        &self,
        hazards: &[Hazard],
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for hazard in hazards {
            let texture = self.assets.sprites.hazards.get_texture(&hazard.hazard_type);
            let transform = (mat3::translate(hazard.sprite.center())
                * mat3::rotate(
                    hazard
                        .direction
                        .map_or(Coord::ZERO, |dir| dir.arg() - Coord::PI / Coord::new(2.0)),
                ))
            .map(Coord::as_f32);
            self.geng.draw_2d_transformed(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(
                    Aabb2::ZERO.extend_symmetric(hazard.sprite.size().map(Coord::as_f32) / 2.0),
                    texture,
                ),
                transform,
            );
            if draw_hitboxes {
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        hazard.collider.raw().map(Coord::as_f32),
                        Rgba::new(1.0, 0.0, 0.0, 0.5),
                    ),
                );
            }
        }
    }

    pub fn draw_coins(
        &self,
        coins: &[Coin],
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for coin in coins {
            let texture = &self.assets.sprites.coin;
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(coin.collider.raw().map(Coord::as_f32), texture),
            );
            if draw_hitboxes {
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        coin.collider.raw().map(Coord::as_f32),
                        Rgba::new(1.0, 1.0, 0.0, 0.5),
                    ),
                );
            }
        }
    }

    pub fn draw_player(
        &self,
        player: &Player,
        draw_hitboxes: bool,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        if let PlayerState::Respawning { .. } = player.state {
        } else {
            let sprites = &self.assets.sprites.player;
            let mut flip = player.facing_left;
            let (texture, transform) = match player.state {
                PlayerState::Drilling | PlayerState::AirDrill { .. } => {
                    let mut velocity = player.velocity.map(|x| {
                        if x.as_f32().abs() < 1.0 {
                            0.00
                        } else {
                            x.as_f32()
                        }
                    });
                    if velocity == vec2::ZERO {
                        velocity.y = 1.0;
                    }
                    flip = false;
                    let mut angle = (velocity.arg() / f32::PI * 4.0 + 2.0).round();
                    let drill = if angle as i32 % 2 == 0 {
                        // Vertical/horizontal
                        &sprites.drill.drill_v0
                    } else {
                        // Diagonal
                        angle -= 1.0;
                        &sprites.drill.drill_d0
                    };
                    (drill, mat3::rotate(angle * f32::PI / 4.0))
                }
                PlayerState::WallSliding { wall_normal, .. } => {
                    flip = wall_normal.x < Coord::ZERO;
                    (&sprites.slide0, mat3::identity())
                }
                _ => (&sprites.idle0, mat3::identity()),
            };

            let pos = player.collider.feet();
            let size = texture.size().map(|x| x as f32) / PIXELS_PER_UNIT;
            let pos = pixel_perfect_pos(pos) + vec2(0.0, size.y / 2.0);
            let transform = mat3::translate(pos) * transform;
            self.geng.draw_2d_transformed(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(
                    Aabb2::ZERO
                        .extend_symmetric(size / 2.0 * vec2(if flip { -1.0 } else { 1.0 }, 1.0)),
                    texture,
                ),
                transform,
            );
            if draw_hitboxes {
                self.geng.draw_2d(
                    framebuffer,
                    camera,
                    &draw_2d::Quad::new(
                        player.collider.raw().map(Coord::as_f32),
                        Rgba::new(0.0, 1.0, 0.0, 0.7),
                    ),
                );
            }
        }
    }

    pub fn draw_particles(
        &self,
        particles: &[Particle],
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        for particle in particles {
            let texture = match particle.particle_type {
                ParticleType::Heart4 => &self.assets.sprites.heart4,
                ParticleType::Heart8 => &self.assets.sprites.heart8,
                ParticleType::Circle { radius, color } => {
                    let t =
                        particle.lifetime.min(Time::ONE) / particle.initial_lifetime.min(Time::ONE);
                    let radius = (radius * t).as_f32();
                    self.geng.draw_2d(
                        framebuffer,
                        camera,
                        &draw_2d::Ellipse::circle(
                            particle.position.map(Coord::as_f32),
                            radius,
                            color,
                        ),
                    );
                    continue;
                }
            };
            let size = texture.size().map(|x| x as f32 / PIXELS_PER_UNIT);
            self.geng.draw_2d(
                framebuffer,
                camera,
                &draw_2d::TexturedQuad::new(
                    Aabb2::point(particle.position.map(Coord::as_f32)).extend_symmetric(size / 2.0),
                    texture,
                ),
            );
        }
    }
}
