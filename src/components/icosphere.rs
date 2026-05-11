use std::collections::HashMap;
use std::f32::consts::PI;

pub struct IcosphereMesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    pub indices: Vec<u32>,
    /// Unit-vector centroid of each face — one entry per tile
    pub tile_directions: Vec<(f64, f64, f64)>,
}

pub fn generate(subdivisions: u32) -> IcosphereMesh {
    let mut verts = base_vertices();
    let mut faces = base_faces();

    for _ in 0..subdivisions {
        faces = subdivide_once(&mut verts, faces);
    }

    // Compute per-vertex spherical UVs
    let mut uv_list: Vec<[f32; 2]> = verts
        .iter()
        .map(|v| {
            let u = v[1].atan2(v[0]) / (2.0 * PI) + 0.5;
            let vv = v[2].asin() / PI + 0.5;
            [u, vv]
        })
        .collect();

    // Fix seam: duplicate vertices where u wraps across the atan2 discontinuity
    fix_seam(&mut verts, &mut uv_list, &mut faces);

    // Tile directions are face centroids — computed after seam fix, but positions
    // are identical for duplicated vertices so the result is the same either way
    let tile_directions = faces
        .iter()
        .map(|face| {
            let a = &verts[face[0] as usize];
            let b = &verts[face[1] as usize];
            let c = &verts[face[2] as usize];
            let cx = (a[0] + b[0] + c[0]) as f64 / 3.0;
            let cy = (a[1] + b[1] + c[1]) as f64 / 3.0;
            let cz = (a[2] + b[2] + c[2]) as f64 / 3.0;
            let len = (cx * cx + cy * cy + cz * cz).sqrt();
            (cx / len, cy / len, cz / len)
        })
        .collect();

    let mut positions = Vec::with_capacity(verts.len() * 3);
    let mut normals = Vec::with_capacity(verts.len() * 3);
    let mut uvs = Vec::with_capacity(verts.len() * 2);

    for (v, uv) in verts.iter().zip(uv_list.iter()) {
        positions.extend_from_slice(v);
        normals.extend_from_slice(v); // unit sphere: normal == position
        uvs.push(uv[0]);
        uvs.push(uv[1]);
    }

    let mut indices = Vec::with_capacity(faces.len() * 3);
    for face in &faces {
        indices.push(face[0]);
        indices.push(face[1]);
        indices.push(face[2]);
    }

    IcosphereMesh {
        positions,
        normals,
        uvs,
        indices,
        tile_directions,
    }
}

/// Duplicate vertices that sit on seam-spanning faces, giving them u + 1.0 so
/// the triangle samples a continuous strip of the texture instead of wrapping.
fn fix_seam(
    verts: &mut Vec<[f32; 3]>,
    uv_list: &mut Vec<[f32; 2]>,
    faces: &mut Vec<[u32; 3]>,
) {
    // Cache: original vertex index → duplicate index with u += 1
    let mut duplicates: HashMap<u32, u32> = HashMap::new();

    for face in faces.iter_mut() {
        let ua = uv_list[face[0] as usize][0];
        let ub = uv_list[face[1] as usize][0];
        let uc = uv_list[face[2] as usize][0];

        // A face spans the seam when its u range is impossibly wide for a single
        // triangle on the sphere — caused by u wrapping from ~1 back to ~0.
        if ua.max(ub).max(uc) - ua.min(ub).min(uc) > 0.5 {
            for slot in face.iter_mut() {
                if uv_list[*slot as usize][0] < 0.5 {
                    let orig = *slot;
                    if !duplicates.contains_key(&orig) {
                        let new_idx = verts.len() as u32;
                        let pos = verts[orig as usize];
                        let old_uv = uv_list[orig as usize];
                        verts.push(pos);
                        uv_list.push([old_uv[0] + 1.0, old_uv[1]]);
                        duplicates.insert(orig, new_idx);
                    }
                    *slot = duplicates[&orig];
                }
            }
        }
    }
}

fn base_vertices() -> Vec<[f32; 3]> {
    let t = (1.0f32 + 5.0f32.sqrt()) / 2.0;
    let scale = 1.0 / (1.0 + t * t).sqrt();
    let a = scale;
    let b = t * scale;
    vec![
        [-a, b, 0.0],
        [a, b, 0.0],
        [-a, -b, 0.0],
        [a, -b, 0.0],
        [0.0, -a, b],
        [0.0, a, b],
        [0.0, -a, -b],
        [0.0, a, -b],
        [b, 0.0, -a],
        [b, 0.0, a],
        [-b, 0.0, -a],
        [-b, 0.0, a],
    ]
}

fn base_faces() -> Vec<[u32; 3]> {
    vec![
        // 5 faces around vertex 0
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        // 5 adjacent faces
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        // 5 faces around vertex 3
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        // 5 adjacent faces
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ]
}

fn subdivide_once(verts: &mut Vec<[f32; 3]>, faces: Vec<[u32; 3]>) -> Vec<[u32; 3]> {
    let mut cache: HashMap<(u32, u32), u32> = HashMap::new();
    let mut new_faces = Vec::with_capacity(faces.len() * 4);

    for face in &faces {
        let [a, b, c] = *face;
        let mab = get_midpoint(verts, &mut cache, a, b);
        let mbc = get_midpoint(verts, &mut cache, b, c);
        let mac = get_midpoint(verts, &mut cache, a, c);
        new_faces.push([a, mab, mac]);
        new_faces.push([mab, b, mbc]);
        new_faces.push([mac, mbc, c]);
        new_faces.push([mab, mbc, mac]);
    }

    new_faces
}

fn get_midpoint(
    verts: &mut Vec<[f32; 3]>,
    cache: &mut HashMap<(u32, u32), u32>,
    a: u32,
    b: u32,
) -> u32 {
    let key = (a.min(b), a.max(b));
    if let Some(&idx) = cache.get(&key) {
        return idx;
    }
    let va = verts[a as usize];
    let vb = verts[b as usize];
    let mx = (va[0] + vb[0]) * 0.5;
    let my = (va[1] + vb[1]) * 0.5;
    let mz = (va[2] + vb[2]) * 0.5;
    let len = (mx * mx + my * my + mz * mz).sqrt();
    let idx = verts.len() as u32;
    verts.push([mx / len, my / len, mz / len]);
    cache.insert(key, idx);
    idx
}
