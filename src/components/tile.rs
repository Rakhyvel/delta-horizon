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
    pub directions: Vec<(f64, f64, f64)>,
}

impl TileMap {
    pub fn new(directions: Vec<(f64, f64, f64)>) -> Self {
        let n = directions.len();
        Self {
            occupants: vec![None; n],
            directions,
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
        let (x, y, z) = self.directions[index as usize];
        nalgebra_glm::vec3(x * radius, y * radius, z * radius)
    }
}
