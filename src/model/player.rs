use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerControl {
    pub jump: bool,
    pub hold_jump: bool,
    pub move_dir: Vec2<Coord>,
    pub drill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub collider: Collider,
    pub velocity: Vec2<Coord>,
    pub state: PlayerState,
    pub control_timeout: Option<Time>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlayerState {
    Grounded,
    WallSliding { wall_normal: Vec2<Coord> },
    Airborn,
    Respawning { time: Time },
    Drilling,
    Finished { time: Time, next_heart: Time },
}

impl Player {
    pub fn new(feet_pos: Vec2<Coord>) -> Self {
        let height = Coord::new(2.0);
        let half_width = Coord::new(1.0 / 2.0);
        Self {
            collider: Collider::new(AABB::from_corners(
                feet_pos - vec2(half_width, Coord::ZERO),
                feet_pos + vec2(half_width, height),
            )),
            velocity: Vec2::ZERO,
            state: PlayerState::Airborn,
            control_timeout: None,
        }
    }
}

impl PlayerControl {
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl Default for PlayerControl {
    fn default() -> Self {
        Self {
            jump: false,
            hold_jump: false,
            move_dir: Vec2::ZERO,
            drill: false,
        }
    }
}
