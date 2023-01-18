use super::*;

const CAMERA_MOVE_SPEED: f32 = 20.0;

pub struct Editor {
    geng: Geng,
    assets: Rc<Assets>,
    render: Render,
    camera: Camera2d,
    framebuffer_size: Vec2<usize>,
    level_name: String,
    level: Level,
    draw_grid: bool,
    cursor_pos: Vec2<f64>,
    cursor_world_pos: Vec2<Coord>,
    dragging: Option<geng::MouseButton>,
    block_options: Vec<Block>,
    props: Vec<PropType>,
    use_prop: bool,
    selected_block: usize,
    selected_prop: usize,
}

#[derive(Debug, Clone, Copy)]
enum Block {
    Tile(Tile),
    Hazard(HazardType),
    Prop(PropType),
    Coin,
}

impl Editor {
    pub fn new(geng: &Geng, assets: &Rc<Assets>, level_name: Option<String>) -> Self {
        let level_name = level_name.unwrap_or_else(|| "new_level.json".to_string());
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            render: Render::new(geng, assets),
            camera: Camera2d {
                center: vec2(0.0, 0.25),
                rotation: 0.0,
                fov: 22.5,
            },
            framebuffer_size: vec2(1, 1),
            level: util::report_err(Level::load(&level_name), "Failed to load level")
                .unwrap_or_default(),
            level_name,
            draw_grid: true,
            cursor_pos: Vec2::ZERO,
            cursor_world_pos: Vec2::ZERO,
            dragging: None,
            block_options: itertools::chain![
                Tile::all().map(Block::Tile),
                HazardType::all().map(Block::Hazard),
                [Block::Coin],
            ]
            .collect(),
            props: itertools::chain![PropType::all()].collect(),
            use_prop: false,
            selected_block: 0,
            selected_prop: 0,
        }
    }

    fn scroll_selected_tile(&mut self, delta: isize) {
        if self.use_prop {
            let current = self.selected_block as isize;
            let target = current + delta;
            self.selected_prop = target.rem_euclid(self.props.len() as isize) as usize;
        } else {
            let current = self.selected_block as isize;
            let target = current + delta;
            self.selected_block = target.rem_euclid(self.block_options.len() as isize) as usize;
        }
    }

    fn place_block(&mut self) {
        let pos = self.level.grid.world_to_grid(self.cursor_world_pos).0;
        let block = if self.use_prop {
            Block::Prop(self.props[self.selected_prop])
        } else {
            self.block_options[self.selected_block]
        };
        match block {
            Block::Tile(tile) => {
                self.level.tiles.set_tile_isize(pos, tile);
            }
            Block::Hazard(hazard) => {
                self.level.place_hazard(pos, hazard);
            }
            Block::Coin => {
                self.level.place_coin(pos);
            }
            Block::Prop(prop) => {
                let size = self
                    .assets
                    .sprites
                    .props
                    .get_texture(&prop)
                    .size()
                    .map(|x| x as f32 / PIXELS_PER_UNIT)
                    .map(Coord::new);
                self.level.place_prop(pos, size, prop);
            }
        }
    }

    fn remove_block(&mut self) {
        self.level.remove_all_at(self.cursor_world_pos);
    }

    fn update_cursor(&mut self, cursor_pos: Vec2<f64>) {
        self.cursor_pos = cursor_pos;
        self.cursor_world_pos = self
            .camera
            .screen_to_world(
                self.framebuffer_size.map(|x| x as f32),
                cursor_pos.map(|x| x as f32),
            )
            .map(Coord::new);

        if let Some(button) = self.dragging {
            match button {
                geng::MouseButton::Left => {
                    self.place_block();
                }
                geng::MouseButton::Right => {
                    self.remove_block();
                }
                geng::MouseButton::Middle => {}
            }
        }
    }

    fn click(&mut self, position: Vec2<f64>, button: geng::MouseButton) {
        self.update_cursor(position);
        self.dragging = Some(button);

        match button {
            geng::MouseButton::Left => {
                self.place_block();
            }
            geng::MouseButton::Right => {
                self.remove_block();
            }
            _ => (),
        }
    }

    fn release(&mut self, _button: geng::MouseButton) {
        self.dragging = None;
    }

    fn save_level(&self) -> anyhow::Result<()> {
        self.level.save(&self.level_name)
    }
}

impl geng::State for Editor {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        let color = Rgba::try_from("#341a22").unwrap();
        ugli::clear(framebuffer, Some(color), None, None);

        self.render
            .draw_level_editor(&self.level, true, &self.camera, framebuffer);

        if self.draw_grid {
            self.render
                .draw_grid(&self.level.grid, self.level.size, &self.camera, framebuffer);
        }
    }

    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        let window = self.geng.window();
        let mut dir = Vec2::ZERO;
        if window.is_key_pressed(geng::Key::A) {
            dir.x -= 1.0;
        }
        if window.is_key_pressed(geng::Key::D) {
            dir.x += 1.0;
        }
        if window.is_key_pressed(geng::Key::S) {
            dir.y -= 1.0;
        }
        if window.is_key_pressed(geng::Key::W) {
            dir.y += 1.0;
        }
        self.camera.center += dir * CAMERA_MOVE_SPEED * delta_time;
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::MouseDown { position, button } => {
                self.click(position, button);
            }
            geng::Event::MouseMove { position, .. } => {
                self.update_cursor(position);
            }
            geng::Event::MouseUp { button, .. } => {
                // self.update_cursor(position);
                self.release(button);
            }
            geng::Event::Wheel { delta } => {
                self.scroll_selected_tile(delta.signum() as isize);
            }
            geng::Event::KeyDown { key } => match key {
                geng::Key::S if self.geng.window().is_key_pressed(geng::Key::LCtrl) => {
                    if let Ok(()) = util::report_err(self.save_level(), "Failed to save level") {
                        info!("Saved the level");
                    }
                }
                geng::Key::R => {
                    self.level.spawn_point = self.cursor_world_pos;
                }
                geng::Key::F => {
                    self.level.finish = self.cursor_world_pos;
                }
                geng::Key::Num1 => {
                    self.use_prop = false;
                }
                geng::Key::Num2 => {
                    self.use_prop = true;
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn ui<'a>(&'a mut self, _cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        use geng::ui::*;

        let framebuffer_size = self.framebuffer_size.map(|x| x as f32);

        let (cell_pos, cell_offset) = self.level.grid.world_to_grid(self.cursor_world_pos);
        let cell_pos = Text::new(
            format!(
                "({}, {}) + ({:.1}, {:.1})",
                cell_pos.x, cell_pos.y, cell_offset.x, cell_offset.y
            ),
            self.geng.default_font(),
            framebuffer_size.y * 0.05,
            Rgba::WHITE,
        );

        let block_ui = |block: &Block| {
            let unit = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| vec2(x, y));
            let (texture, geometry) = match block {
                Block::Tile(tile) => {
                    let set = self.assets.sprites.tiles.get_tile_set(tile);
                    (set.texture(), set.get_tile_connected([false; 8]))
                }
                Block::Hazard(hazard) => (self.assets.sprites.hazards.get_texture(hazard), unit),
                Block::Coin => (&self.assets.sprites.coin, unit),
                Block::Prop(prop) => (self.assets.sprites.props.get_texture(prop), unit),
            };
            ui::TextureBox::new(&self.geng, &self.assets, texture, geometry).fixed_size(
                vec2(framebuffer_size.y * 0.05, framebuffer_size.y * 0.05).map(|x| x as f64),
            )
        };

        let selected_tile = if self.use_prop {
            block_ui(&Block::Prop(self.props[self.selected_prop]))
        } else {
            block_ui(&self.block_options[self.selected_block])
        };

        let ui = geng::ui::stack![
            cell_pos.align(vec2(1.0, 1.0)),
            geng::ui::column![selected_tile]
                .align(vec2(1.0, 0.0))
                .uniform_padding(framebuffer_size.y as f64 * 0.05),
        ];

        Box::new(ui)
    }
}

pub fn run(geng: &Geng, level: Option<String>) -> impl geng::State {
    let future = {
        let geng = geng.clone();
        async move {
            let assets: Rc<Assets> = geng::LoadAsset::load(&geng, &run_dir().join("assets"))
                .await
                .expect("Failed to load assets");
            Editor::new(&geng, &assets, level)
        }
    };
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen, future, |state| state)
}