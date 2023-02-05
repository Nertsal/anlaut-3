use super::*;

#[derive(Debug, Clone)]
pub enum Action {
    Place {
        block: PlaceableType,
        pos: vec2<Coord>,
    },
    Remove {
        ids: Vec<PlaceableId>,
    },
    Replace(Placeable),
}

impl Editor {
    pub fn action(&mut self, action: Action) {
        self.redo_actions.clear();
        let undo_action = self.action_impl(action);
        self.undo_actions.extend(undo_action);
    }

    fn action_impl(&mut self, action: Action) -> Vec<Action> {
        let actions = match action {
            Action::Place { block, pos } => self.action_place(block, pos),
            Action::Remove { ids } => self.action_remove(&ids),
            Action::Replace(block) => self.action_replace(block),
        };
        self.update_geometry();
        actions
    }

    pub fn undo(&mut self) {
        if let Some(action) = self.undo_actions.pop() {
            let redo_action = self.action_impl(action);
            self.redo_actions.extend(redo_action);
        }
    }

    pub fn redo(&mut self) {
        if let Some(action) = self.redo_actions.pop() {
            let undo_action = self.action_impl(action);
            self.undo_actions.extend(undo_action);
        }
    }

    fn action_place(&mut self, block: PlaceableType, position: vec2<Coord>) -> Vec<Action> {
        let grid_pos = self.world.level.grid.world_to_grid(position).0;
        match block {
            PlaceableType::Tile(tile) => {
                self.world
                    .level
                    .tiles
                    .set_tile_isize(grid_pos, tile, &self.assets);
            }
            PlaceableType::Hazard(hazard) => {
                self.world.level.place_hazard(position, hazard);
            }
            PlaceableType::Coin => {
                self.world.level.place_coin(grid_pos);
            }
            PlaceableType::Prop(prop) => {
                let size = self
                    .assets
                    .sprites
                    .props
                    .get_texture(&prop)
                    .size()
                    .map(|x| x as f32 / PIXELS_PER_UNIT as f32)
                    .map(Coord::new);
                self.world.level.place_prop(grid_pos, size, prop);
            }
            PlaceableType::Spotlight(light) => self
                .world
                .level
                .spotlights
                .push(SpotlightSource { position, ..light }),
        }
        vec![]
    }

    fn action_replace(&mut self, block: Placeable) -> Vec<Action> {
        match block {
            Placeable::Tile((tile, pos)) => {
                self.world
                    .level
                    .tiles
                    .set_tile_isize(pos, tile, &self.assets);
            }
            Placeable::Hazard(hazard) => {
                self.world.level.hazards.push(hazard);
            }
            Placeable::Coin(coin) => {
                self.world.level.coins.push(coin);
            }
            Placeable::Prop(prop) => {
                self.world.level.props.push(prop);
            }
            Placeable::Spotlight(spotlight) => self.world.level.spotlights.push(spotlight),
        }
        vec![]
    }

    fn action_remove(&mut self, ids: &[PlaceableId]) -> Vec<Action> {
        let actions = self
            .world
            .level
            .remove_blocks(ids, &self.assets)
            .into_iter()
            .map(Action::Replace)
            .collect();
        self.hovered.clear();
        actions
    }
}
