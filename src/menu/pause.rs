use super::*;

use geng::ui::*;

pub struct PauseMenu {
    geng: Geng,
    assets: Rc<Assets>,
    state: MenuState,
    resume: bool,
    quit: bool,
}

enum MenuState {
    Paused,
    Settings,
    Rules,
}

impl PauseMenu {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            state: MenuState::Paused,
            resume: false,
            quit: false,
        }
    }

    pub fn pause(&mut self) {
        self.state = MenuState::Paused;
    }

    pub fn resume(&mut self) -> bool {
        std::mem::take(&mut self.resume)
    }

    pub fn quit(&mut self) -> bool {
        std::mem::take(&mut self.quit)
    }

    pub fn ui<'a>(
        &'a mut self,
        world: &'a mut World,
        cx: &'a geng::ui::Controller,
    ) -> Box<dyn geng::ui::Widget + 'a> {
        match self.state {
            MenuState::Paused => Box::new(self.paused_ui(world, cx)),
            MenuState::Settings => Box::new(self.settings_ui(world, cx)),
            MenuState::Rules => Box::new(self.rules_ui(world, cx)),
        }
    }

    fn paused_ui<'a>(
        &'a mut self,
        world: &'a mut World,
        cx: &'a geng::ui::Controller,
    ) -> impl geng::ui::Widget + 'a {
        let resume = {
            let button = geng::ui::Button::new(cx, "Resume");
            if button.was_clicked() {
                self.resume = true;
            }
            button
        };

        let retry = {
            let button = geng::ui::Button::new(cx, "Retry");
            if button.was_clicked() {
                world.kill_player();
                self.resume = true;
            }
            button
        };

        let settings = {
            let button = geng::ui::Button::new(cx, "Settings");
            if button.was_clicked() {
                self.state = MenuState::Settings;
            }
            button
        };

        let rules = {
            let button = geng::ui::Button::new(cx, "Rules");
            if button.was_clicked() {
                self.state = MenuState::Rules;
            }
            button
        };

        let quit = {
            let button = geng::ui::Button::new(cx, "Quit");
            if button.was_clicked() {
                self.quit = true;
            }
            button
        };

        geng::ui::column![resume, retry, settings, rules, quit,].align(vec2(0.5, 0.5))
    }

    fn rules_ui<'a>(
        &'a mut self,
        world: &'a mut World,
        cx: &'a geng::ui::Controller,
    ) -> impl geng::ui::Widget + 'a {
        let font = self.geng.default_font();
        let text_size = cx.theme().text_size;

        let back = {
            let button = geng::ui::Button::new(cx, "Back");
            if button.was_clicked() {
                self.state = MenuState::Paused;
            }
            button
        };

        let restore = {
            let button = geng::ui::Button::new(cx, "Restore default");
            if button.was_clicked() {
                world.rules = self.assets.rules.clone();
            }
            button
        };

        let slider = |name, range, value: &mut R32| {
            ui::slider(cx, name, value, range, font.clone(), text_size)
        };

        #[rustfmt::skip]
        let rules = geng::ui::column![geng::ui::row![
            geng::ui::column![
                // slider(gravity: "vec2", 0.0..=1.0, &mut world.rules.vec2),
                slider("move_speed", 5.0..=20.0, &mut world.rules.move_speed),
                slider("acceleration", 0.0..=5.0, &mut world.rules.acceleration),

                slider("jump_buffer_time", 0.0..=0.5, &mut world.rules.jump_buffer_time),
                slider("coyote_time", 0.0..=0.5, &mut world.rules.coyote_time),
                slider("edge_correction_max", 0.0..=0.5, &mut world.rules.edge_correction_max),

                slider("free_fall_speed", 5.0..=30.0, &mut world.rules.free_fall_speed),
                slider("wall_slide_speed", 1.0..=20.0, &mut world.rules.wall_slide_speed),
            ].padding_right(text_size as f64 * 2.0),
            geng::ui::column![
                slider("normal_jump_push", 0.0..=5.0, &mut world.rules.jump.normal_push),
                slider("normal_jump_strength", 5.0..=20.0, &mut world.rules.jump.normal_strength),
                slider("wall_jump_strength", 5.0..=20.0, &mut world.rules.jump.wall_strength),
                slider("wall_jump_angle", 0.0..=f64::PI / 2.0, &mut world.rules.jump.wall_angle),
                slider("wall_jump_timeout", 0.0..=0.5, &mut world.rules.jump.wall_timeout),
                slider("fall_multiplier", 0.0..=10.0, &mut world.rules.jump.fall_multiplier),
                slider("low_jump_multiplier", 0.0..=10.0, &mut world.rules.jump.low_multiplier),

                // &mut world.rules.can_drill_dash
                slider("drill_release_time", 0.0..=0.5, &mut world.rules.drill.release_time),
                slider("drill_speed_min", 5.0..=20.0, &mut world.rules.drill.speed_min),
                slider("drill_speed_inc", 0.0..=5.0, &mut world.rules.drill.speed_inc),
                slider("drill_dash_time", 0.0..=0.5, &mut world.rules.drill.dash_time),
                slider("drill_dash_speed_min", 5.0..=20.0, &mut world.rules.drill.dash_speed_min),
                slider("drill_dash_speed_inc", 0.0..=10.0, &mut world.rules.drill.dash_speed_inc),
                slider("drill_jump_speed_min", 5.0..=30.0, &mut world.rules.drill.jump_speed_min),
                slider("drill_jump_speed_inc", 0.0..=10.0, &mut world.rules.drill.jump_speed_inc),
            ]],
            restore,
        ];

        geng::ui::column![back, rules,].align(vec2(0.5, 0.5))
    }

    fn settings_ui<'a>(
        &'a mut self,
        world: &'a mut World,
        cx: &'a geng::ui::Controller,
    ) -> impl geng::ui::Widget + 'a {
        let font = self.geng.default_font();
        let text_size = cx.theme().text_size;

        let back = {
            let button = geng::ui::Button::new(cx, "Back");
            if button.was_clicked() {
                self.state = MenuState::Paused;
            }
            button
        };

        let screen_size = {
            let text = geng::ui::Text::new(
                format!("Screen size: {}", world.screen_resolution),
                font,
                text_size,
                Rgba::WHITE,
            );

            let mut current = world.screen_resolution.x / PIXELS_PER_UNIT;

            let inc = Button::new(cx, "+");
            if inc.was_clicked() {
                current += 1;
            }
            let dec = Button::new(cx, "-");
            if dec.was_clicked() {
                current -= 1;
            }

            current = current.clamp(10, 50);
            let target = current * PIXELS_PER_UNIT;
            world.update_screen_size(target);

            geng::ui::row![
                text.padding_right(text_size.into()),
                inc.padding_right(text_size.into()),
                dec.padding_right(text_size.into()),
            ]
        };

        geng::ui::column![back, screen_size,].align(vec2(0.5, 0.5))
    }
}