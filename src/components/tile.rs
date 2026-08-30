use apricot::tri::Tri;
use hecs::Entity;
use nalgebra_glm::DVec3;

/// Tags a building entity with the tile index it occupies on its parent body
#[allow(dead_code)]
pub struct SurfaceTile {
    pub index: u32,
}

/// Per-body component tracking tile occupancy and the face-centroid directions
pub struct TileMap {
    pub occupants: Vec<Option<Entity>>,
    pub tris: Vec<Tri>,
}

impl TileMap {
    pub fn new(tris: Vec<Tri>) -> Self {
        let n = tris.len();
        Self {
            occupants: vec![None; n],
            tris,
        }
    }

    #[allow(dead_code)]
    pub fn is_free(&self, index: u32) -> bool {
        self.occupants[index as usize].is_none()
    }

    pub fn occupy(&mut self, index: u32, entity: Entity) {
        self.occupants[index as usize] = Some(entity);
    }

    #[allow(dead_code)]
    pub fn free(&mut self, index: u32) {
        self.occupants[index as usize] = None;
    }

    /// Surface position offset for a tile, scaled to the body radius
    pub fn tile_offset(&self, index: u32, radius: f64) -> DVec3 {
        let t = self.tris[index as usize];
        let dir: DVec3 = nalgebra_glm::convert(((t.v0() + t.v1() + t.v2()) / 3.0).normalize());
        dir * radius
    }
}

pub struct TileSets {
    pub dwarf: Vec<Tri>,
    pub sub: Vec<Tri>,
    pub large: Vec<Tri>,
}
