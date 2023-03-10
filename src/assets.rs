use super::*;

#[derive(geng::Assets)]
pub struct Assets {
    #[asset(load_with = "load_font(&geng, &base_path.join(\"pixel.ttf\"))")]
    pub font: Rc<geng::Font>,
    pub shaders: Shaders,
    pub sprites: Sprites,
    pub sounds: Sounds,
    #[asset(postprocess = "loop_sound")]
    pub music: geng::Sound,
    pub rules: Rules,
}

#[derive(geng::Assets)]
pub struct Shaders {
    pub texture: ugli::Program,
    pub texture_mask: ugli::Program,
    pub grid: ugli::Program,
    pub global_light: ugli::Program,
    pub spotlight: ugli::Program,
    pub point_light_shadow_map: ugli::Program,
    pub shadow_remove: ugli::Program,
    pub normal_map: ugli::Program,
}

#[derive(geng::Assets)]
pub struct Sounds {
    pub jump: geng::Sound,
    pub death: geng::Sound,
    pub coin: geng::Sound,
    #[asset(postprocess = "loop_sound")]
    pub drill: geng::Sound,
    pub drill_jump: geng::Sound,
    pub charm: geng::Sound,
    #[asset(path = "cutscene.mp3")]
    pub cutscene: geng::Sound,
}

#[derive(geng::Assets)]
pub struct Sprites {
    pub tiles: TileSprites,
    pub hazards: HazardSprites,
    pub player: PlayerSprites,
    pub props: PropSprites,
    #[asset(postprocess = "pixel")]
    pub partner: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub room: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub coin: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub heart4: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub heart8: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub background: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub sun: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub skull: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub drill_hover: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub cursor: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub spotlight: ugli::Texture,
}

#[derive(geng::Assets)]
pub struct TileSprites {
    pub mask: TileSet,
    pub air: TileSet,
    pub grass: TileSet,
    pub stone: TileSet,
}

#[derive(geng::Assets)]
pub struct HazardSprites {
    #[asset(postprocess = "pixel")]
    pub spikes: ugli::Texture,
}

#[derive(geng::Assets)]
pub struct PropSprites {
    #[asset(postprocess = "pixel")]
    pub tutorial_drill_use: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub tutorial_drill_jump: ugli::Texture,
}

#[derive(geng::Assets)]
pub struct PlayerSprites {
    #[asset(postprocess = "pixel")]
    pub idle0: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub slide0: ugli::Texture,
    pub drill: DrillSprites,
}

#[derive(geng::Assets)]
pub struct DrillSprites {
    #[asset(postprocess = "pixel")]
    pub drill_v0: ugli::Texture,
    #[asset(postprocess = "pixel")]
    pub drill_d0: ugli::Texture,
}

#[derive(Deref)]
pub struct Animation {
    #[deref]
    pub frames: Vec<(ugli::Texture, f32)>,
}

impl TileSprites {
    pub fn get_tile_set(&self, tile: &Tile) -> &TileSet {
        match tile {
            Tile::Air => &self.air,
            Tile::Grass => &self.grass,
            Tile::Stone => &self.stone,
        }
    }
}

impl HazardSprites {
    pub fn get_texture(&self, hazard: &HazardType) -> &ugli::Texture {
        match hazard {
            HazardType::Spikes => &self.spikes,
        }
    }
}

impl PropSprites {
    pub fn get_texture(&self, prop: &PropType) -> &ugli::Texture {
        match prop {
            PropType::DrillUse => &self.tutorial_drill_use,
            PropType::DrillJump => &self.tutorial_drill_jump,
        }
    }
}

fn pixel(texture: &mut ugli::Texture) {
    texture.set_filter(ugli::Filter::Nearest)
}

fn loop_sound(sound: &mut geng::Sound) {
    sound.looped = true;
}

// impl Animation {
//     pub fn get_frame(&self, time: Time) -> Option<&ugli::Texture> {
//         let i = (time.as_f32() * self.frames.len() as f32).floor() as usize;
//         self.frames.get(i)
//     }
// }

impl geng::LoadAsset for Animation {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let data = <Vec<u8> as geng::LoadAsset>::load(geng, path);
        let geng = geng.clone();
        async move {
            let data = data.await?;
            use image::AnimationDecoder;
            Ok(Self {
                frames: image::codecs::gif::GifDecoder::new(data.as_slice())
                    .unwrap()
                    .into_frames()
                    .map(|frame| {
                        let frame = frame.unwrap();
                        let (n, d) = frame.delay().numer_denom_ms();
                        let mut texture =
                            ugli::Texture::from_image_image(geng.ugli(), frame.into_buffer());
                        texture.set_filter(ugli::Filter::Nearest);
                        (texture, n as f32 / d as f32 / 1000.0)
                    })
                    .collect(),
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("gif");
}

fn load_font(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Rc<geng::Font>> {
    let geng = geng.clone();
    let path = path.to_owned();
    async move {
        let data = <Vec<u8> as geng::LoadAsset>::load(&geng, &path).await?;
        Ok(Rc::new(geng::Font::new(
            &geng,
            &data,
            geng::ttf::Options {
                pixel_size: 64.0,
                max_distance: 0.1,
            },
        )?))
    }
    .boxed_local()
}
