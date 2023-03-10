use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Level {
    pub drill_allowed: bool,
    #[serde(default)]
    pub grid: Grid,
    pub size: vec2<usize>,
    pub spawn_point: vec2<Coord>,
    pub finish: vec2<Coord>,
    pub tiles: TileMap,
    #[serde(default)]
    pub hazards: Vec<Hazard>,
    #[serde(default)]
    pub coins: Vec<Coin>,
    #[serde(default)]
    pub props: Vec<Prop>,
    #[serde(default)]
    pub global_light: GlobalLightSource,
    #[serde(default)]
    pub spotlights: Vec<SpotlightSource>,
    pub next_level: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum BlockType {
    Tile(Tile),
    Hazard(HazardType),
    Prop(PropType),
    Spotlight(SpotlightSource),
    Coin,
}

#[derive(Debug, Clone, Copy)]
pub enum BlockId {
    Tile(vec2<isize>),
    Hazard(usize),
    Prop(usize),
    Coin(usize),
    Spotlight(usize),
}

#[derive(Debug, Clone)]
pub enum Block {
    Tile((Tile, vec2<isize>)),
    Hazard(Hazard),
    Prop(Prop),
    Coin(Coin),
    Spotlight(SpotlightSource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub collider: Collider,
    pub collected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hazard {
    pub sprite: Aabb2<Coord>,
    pub direction: Option<vec2<Coord>>,
    pub collider: Collider,
    pub hazard_type: HazardType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prop {
    pub sprite: Aabb2<Coord>,
    pub prop_type: PropType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HazardType {
    Spikes,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PropType {
    DrillUse,
    DrillJump,
}

impl HazardType {
    pub fn all() -> [Self; 1] {
        use HazardType::*;
        [Spikes]
    }
}

impl PropType {
    pub fn all() -> [Self; 2] {
        use PropType::*;
        [DrillUse, DrillJump]
    }
}

impl Level {
    pub fn new(size: vec2<usize>) -> Self {
        let mut grid = Grid::default();
        grid.offset = size.map(|x| Coord::new(x as f32 / 2.0)) * grid.cell_size;
        Self {
            spawn_point: grid.grid_to_world(size.map(|x| x as isize / 2)),
            finish: grid.grid_to_world(size.map(|x| x as isize / 2)),
            tiles: TileMap::new(size),
            hazards: Vec::new(),
            coins: Vec::new(),
            props: Vec::new(),
            next_level: None,
            drill_allowed: true,
            global_light: default(),
            spotlights: Vec::new(),
            grid,
            size,
        }
    }

    pub fn finish(&self) -> Collider {
        Collider::new(Aabb2::point(self.finish).extend_positive(self.grid.cell_size))
    }

    pub fn bounds(&self) -> Aabb2<Coord> {
        Aabb2::from_corners(
            self.grid.grid_to_world(vec2(0, 0)),
            self.grid.grid_to_world(self.size.map(|x| x as isize)),
        )
    }

    pub fn place_hazard(&mut self, pos: vec2<isize>, hazard: HazardType) {
        let connect = |pos| {
            self.tiles
                .get_tile_isize(pos)
                .map(|tile| !matches!(tile, Tile::Air))
                .unwrap_or(false)
        };
        let (direction, collider) = match hazard {
            HazardType::Spikes => {
                let size = vec2(0.8, 0.4);
                let direction = -[vec2(1, 0), vec2(-1, 0), vec2(0, 1)]
                    .into_iter()
                    .find(|&d| connect(pos + d))
                    .unwrap_or(vec2(0, -1))
                    .map(|x| x as f32);
                let pos = vec2(0.5, 0.5) - direction * 0.5;
                let aabb = Aabb2::from_corners(
                    pos + vec2(-size.x * direction.y * 0.5, -size.x * direction.x * 0.5),
                    pos + vec2(
                        size.x * direction.y * 0.5 + size.y * direction.x,
                        size.y * direction.y + size.x * direction.x * 0.5,
                    ),
                );
                let aabb = aabb.map(Coord::new);
                (
                    Some(direction.map(Coord::new)),
                    Aabb2::point(aabb.bottom_left() * self.grid.cell_size)
                        .extend_positive(aabb.size() * self.grid.cell_size),
                )
            }
        };
        let pos = self.grid.grid_to_world(pos);
        let collider = Collider::new(collider.translate(pos));
        self.hazards.push(Hazard {
            sprite: Aabb2::point(pos).extend_positive(self.grid.cell_size),
            collider,
            direction,
            hazard_type: hazard,
        });
    }

    pub fn place_prop(&mut self, pos: vec2<isize>, size: vec2<Coord>, prop: PropType) {
        let pos = self.grid.grid_to_world(pos);
        let sprite = Aabb2::point(pos).extend_symmetric(size / Coord::new(2.0));
        self.props.push(Prop {
            sprite,
            prop_type: prop,
        });
    }

    pub fn place_coin(&mut self, pos: vec2<isize>) {
        let collider = Aabb2::ZERO.extend_positive(self.grid.cell_size);
        let pos = self.grid.grid_to_world(pos);
        let collider = Collider::new(collider.translate(pos));
        self.coins.push(Coin {
            collider,
            collected: false,
        });
    }

    pub fn get_hovered(&mut self, pos: vec2<Coord>) -> Vec<BlockId> {
        let grid_pos = self.grid.world_to_grid(pos).0;
        itertools::chain![
            self.spotlights
                .iter()
                .enumerate()
                .filter(|(_, spotlight)| (spotlight.position - pos).len() < Coord::new(0.5))
                .map(|(i, _)| BlockId::Spotlight(i)),
            self.props
                .iter()
                .enumerate()
                .filter(|(_, prop)| prop.sprite.contains(pos))
                .map(|(i, _)| BlockId::Prop(i)),
            self.hazards
                .iter()
                .enumerate()
                .filter(|(_, hazard)| hazard.collider.contains(pos))
                .map(|(i, _)| BlockId::Hazard(i)),
            self.coins
                .iter()
                .enumerate()
                .filter(|(_, hazard)| hazard.collider.contains(pos))
                .map(|(i, _)| BlockId::Coin(i)),
            self.tiles
                .get_tile_isize(grid_pos)
                .map(|_| BlockId::Tile(grid_pos)),
        ]
        .collect()
    }

    pub fn get_block(&self, id: BlockId) -> Option<Block> {
        match id {
            BlockId::Tile(pos) => self
                .tiles
                .get_tile_isize(pos)
                .map(|tile| Block::Tile((tile, pos))),
            BlockId::Hazard(id) => self.hazards.get(id).cloned().map(Block::Hazard),
            BlockId::Prop(id) => self.props.get(id).cloned().map(Block::Prop),
            BlockId::Coin(id) => self.coins.get(id).cloned().map(Block::Coin),
            BlockId::Spotlight(id) => self.spotlights.get(id).cloned().map(Block::Spotlight),
        }
    }

    pub fn remove_blocks(&mut self, blocks: &[BlockId]) -> Vec<Block> {
        let mut spotlights = Vec::new();
        let mut props = Vec::new();
        let mut hazards = Vec::new();
        let mut coins = Vec::new();
        let mut tiles = Vec::new();
        for &block in blocks {
            match block {
                BlockId::Tile(pos) => tiles.push(pos),
                BlockId::Hazard(id) => hazards.push(id),
                BlockId::Prop(id) => props.push(id),
                BlockId::Coin(id) => coins.push(id),
                BlockId::Spotlight(id) => spotlights.push(id),
            }
        }

        spotlights.sort_unstable();
        props.sort_unstable();
        hazards.sort_unstable();
        coins.sort_unstable();

        let mut removed = Vec::new();
        for id in spotlights.into_iter().rev() {
            let light = self.spotlights.swap_remove(id);
            removed.push(Block::Spotlight(light));
        }
        for id in props.into_iter().rev() {
            let prop = self.props.swap_remove(id);
            removed.push(Block::Prop(prop));
        }
        for id in hazards.into_iter().rev() {
            let hazard = self.hazards.swap_remove(id);
            removed.push(Block::Hazard(hazard));
        }
        for id in coins.into_iter().rev() {
            let coin = self.coins.swap_remove(id);
            removed.push(Block::Coin(coin));
        }
        for pos in tiles {
            if let Some(tile) = self.tiles.get_tile_isize(pos) {
                removed.push(Block::Tile((tile, pos)));
            }
            self.tiles.set_tile_isize(pos, Tile::Air);
        }

        removed
    }

    pub fn change_size(&mut self, size: vec2<usize>) {
        self.tiles.change_size(size);
        self.size = size;
    }

    pub fn translate(&mut self, delta: vec2<isize>) {
        self.tiles.translate(delta);

        let delta = self.grid.grid_to_world(delta) - self.grid.grid_to_world(vec2::ZERO);
        self.spawn_point += delta;
        self.finish += delta;
        for coin in &mut self.coins {
            coin.translate(delta);
        }
        for hazard in &mut self.hazards {
            hazard.translate(delta);
        }
        for prop in &mut self.props {
            prop.translate(delta);
        }
        for light in &mut self.spotlights {
            light.position += delta;
        }
    }

    pub fn calculate_geometry(
        &self,
        geng: &Geng,
        assets: &Assets,
    ) -> (
        HashMap<Tile, ugli::VertexBuffer<Vertex>>,
        HashMap<Tile, ugli::VertexBuffer<MaskedVertex>>,
    ) {
        let mut tiles_geometry = HashMap::<Tile, Vec<Vertex>>::new();
        let mut masked_geometry = HashMap::<Tile, Vec<MaskedVertex>>::new();
        let calc_geometry = |i: usize, tile: &Tile, connections: [Connection; 8]| {
            let pos = index_to_pos(i, self.size.x);
            let pos = self.grid.grid_to_world(pos.map(|x| x as isize));
            let pos = Aabb2::point(pos)
                .extend_positive(self.grid.cell_size)
                .map(Coord::as_f32);
            let set = assets.sprites.tiles.get_tile_set(tile);
            let geometry = set.get_tile_connected(connections);
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
            let matrix = mat3::translate(pos.bottom_left()) * mat3::scale(pos.size());
            geometry.map(|vertex| {
                let pos = matrix * vertex.a_pos.extend(1.0);
                Vertex {
                    a_pos: pos.xy() / pos.z,
                    ..vertex
                }
            })
        };
        for (i, tile) in self.tiles.tiles().iter().enumerate() {
            if let Tile::Air = tile {
                continue;
            }

            let connections = self.tiles.get_tile_connections(i);
            let neighbours = self.tiles.get_tile_neighbours(i);
            if neighbours.contains(&Some(Tile::Grass)) {
                let geometry = calc_geometry(i, &Tile::Grass, connections);
                let mask = assets.sprites.tiles.mask.get_tile_connected(connections);
                let idx = [0, 1, 2, 0, 2, 3];
                let geometry = geometry.into_iter().zip(idx).map(|(v, i)| v.mask(mask[i]));
                masked_geometry
                    .entry(Tile::Grass)
                    .or_default()
                    .extend(geometry);
            }

            tiles_geometry
                .entry(*tile)
                .or_default()
                .extend(calc_geometry(i, tile, connections));
        }
        let tiles = tiles_geometry
            .into_iter()
            .map(|(tile, geom)| (tile, ugli::VertexBuffer::new_dynamic(geng.ugli(), geom)))
            .collect();
        let masked = masked_geometry
            .into_iter()
            .map(|(tile, geom)| (tile, ugli::VertexBuffer::new_dynamic(geng.ugli(), geom)))
            .collect();
        (tiles, masked)
    }

    pub fn calculate_light_geometry(&self, geng: &Geng) -> Vec<StaticPolygon> {
        itertools::chain![self
            .tiles
            .tiles()
            .iter()
            .enumerate()
            .filter_map(|(i, tile)| {
                (!matches!(tile, Tile::Air)).then(|| {
                    let pos = index_to_pos(i, self.size.x);
                    let pos = self.grid.grid_to_world(pos.map(|x| x as isize));
                    let pos = Aabb2::point(pos)
                        .extend_positive(self.grid.cell_size)
                        .map(Coord::as_f32);
                    let matrix = mat3::translate(pos.bottom_left()) * mat3::scale(pos.size());
                    StaticPolygon::new(
                        geng,
                        &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
                            .map(|(x, y)| (matrix * vec2(x, y).extend(1.0)).into_2d()),
                    )
                })
            })]
        .collect()
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let path = run_dir().join("assets").join("levels").join(path);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::new(file);
            Ok(serde_json::from_reader(reader)?)
        }
        #[cfg(target_arch = "wasm32")]
        {
            anyhow::bail!("unimplemented")
        }
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let path = run_dir().join("assets").join("levels").join(path);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::File::create(path)?;
            let writer = std::io::BufWriter::new(file);
            serde_json::to_writer_pretty(writer, self)?;
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            anyhow::bail!("unimplemented")
        }
    }
}

impl Block {
    pub fn position(&self) -> vec2<Coord> {
        match self {
            Block::Tile(_) => unimplemented!(),
            Block::Hazard(hazard) => hazard.collider.pos(),
            Block::Prop(prop) => prop.sprite.center(),
            Block::Coin(coin) => coin.collider.pos(),
            Block::Spotlight(light) => light.position,
        }
    }
}

impl Hazard {
    pub fn teleport(&mut self, pos: vec2<Coord>) {
        self.sprite.translate(pos - self.sprite.bottom_left());
        self.collider.teleport(pos);
    }

    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.sprite.translate(delta);
        self.collider.translate(delta);
    }
}

impl Coin {
    pub fn teleport(&mut self, pos: vec2<Coord>) {
        self.collider.teleport(pos);
    }

    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.collider.translate(delta);
    }
}

impl Prop {
    pub fn teleport(&mut self, pos: vec2<Coord>) {
        self.sprite.translate(pos - self.sprite.center());
    }

    pub fn translate(&mut self, delta: vec2<Coord>) {
        self.sprite.translate(delta);
    }
}

impl BlockId {
    pub fn fits_type(&self, ty: BlockType) -> bool {
        matches!(
            (self, ty),
            (BlockId::Tile(_), BlockType::Tile(_))
                | (BlockId::Hazard(_), BlockType::Hazard(_))
                | (BlockId::Prop(_), BlockType::Prop(_))
                | (BlockId::Coin(_), BlockType::Coin)
                | (BlockId::Spotlight(_), BlockType::Spotlight(_))
        )
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::new(vec2(40, 23))
    }
}

#[derive(ugli::Vertex, Debug, Clone, Copy)]
pub struct Vertex {
    pub a_pos: vec2<f32>,
    pub a_uv: vec2<f32>,
}

#[derive(ugli::Vertex, Debug, Clone, Copy)]
pub struct MaskedVertex {
    pub a_pos: vec2<f32>,
    pub a_uv: vec2<f32>,
    pub a_mask_uv: vec2<f32>,
}

impl Vertex {
    pub fn mask(self, a_mask_uv: vec2<f32>) -> MaskedVertex {
        MaskedVertex {
            a_pos: self.a_pos,
            a_uv: self.a_uv,
            a_mask_uv,
        }
    }
}
