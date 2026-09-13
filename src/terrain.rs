use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*};

pub const STEP: f32 = 1.5;
pub const FLOOR: f32 = -6.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapSize {
    #[default]
    Small,
    Medium,
    Large,
}

impl MapSize {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];

    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
        }
    }

    pub fn cells(self) -> usize {
        match self {
            Self::Small => 80,
            Self::Medium => 120,
            Self::Large => 160,
        }
    }

    pub fn half(self) -> f32 {
        self.cells() as f32 * STEP / 2.0
    }

    pub fn scale(self) -> f32 {
        self.half() / Self::Small.half()
    }

    /// West and east of the island. North-south (y) is unique per seed.
    pub fn spawns(self, seed: u64) -> [Vec2; 2] {
        let east = 40.0 * self.scale();
        let mut rng = Rng::new(seed ^ 0xC0A57);
        [
            Vec2::new(-east, rng.range(-east, east)),
            Vec2::new(east, rng.range(-east, east)),
        ]
    }

    /// North shore, inset so the compass rose stays on the island.
    pub fn windsock_site(self) -> Vec2 {
        Vec2::new(0.0, -(self.half() - 12.0))
    }
}

impl std::str::FromStr for MapSize {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|size| size.label().eq_ignore_ascii_case(value))
            .ok_or_else(|| format!("Unknown map size '{value}'; use small, medium, or large"))
    }
}

/// Small deterministic PRNG: a seed reproduces terrain and round winds.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn unit(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        (((z ^ (z >> 31)) >> 40) as f32) / (1u32 << 24) as f32
    }

    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }
}

#[derive(Resource)]
pub struct Terrain {
    pub size: MapSize,
    pub spawns: [Vec2; 2],
    heights: Vec<f32>,
    original: Vec<f32>,
    pub mesh: Handle<Mesh>,
}

impl Terrain {
    pub fn new(seed: u64, size: MapSize) -> Self {
        let cells = size.cells();
        let half = size.half();
        let scale = size.scale();
        let spawns = size.spawns(seed);
        let mut rng = Rng::new(seed);
        let hills: Vec<_> = (0..24)
            .map(|_| {
                (
                    Vec2::new(rng.range(-half, half), rng.range(-half, half)),
                    rng.range(9.0, 24.0) * scale,
                    rng.range(-3.0, 8.0),
                )
            })
            .collect();
        let phases = [
            rng.range(0.0, std::f32::consts::TAU),
            rng.range(0.0, std::f32::consts::TAU),
        ];
        let height = |p: Vec2| {
            let mut h = 3.0
                + (p.x / scale * 0.075 + phases[0]).sin() * 1.8
                + (p.y / scale * 0.09 + phases[1]).cos() * 1.5;
            for (center, radius, amplitude) in &hills {
                h += amplitude * (-p.distance_squared(*center) / (radius * radius)).exp();
            }
            h.clamp(0.0, 17.0)
        };
        let mut heights = Vec::with_capacity((cells + 1).pow(2));
        for z in 0..=cells {
            for x in 0..=cells {
                let p = Vec2::new(x as f32 * STEP - half, z as f32 * STEP - half);
                let mut h = height(p);
                let pads = [
                    (spawns[0], 4.0, 9.0),
                    (spawns[1], 4.0, 9.0),
                    (size.windsock_site(), 9.0, 14.0),
                ];
                for (center, inner, outer) in pads {
                    let blend = ((p.distance(center) - inner) / (outer - inner)).clamp(0.0, 1.0);
                    let blend = blend * blend * (3.0 - 2.0 * blend);
                    h = height(center) * (1.0 - blend) + h * blend;
                }
                heights.push(h);
            }
        }
        Self {
            size,
            spawns,
            original: heights.clone(),
            heights,
            mesh: Handle::default(),
        }
    }

    fn vertex(&self, x: usize, z: usize) -> Vec3 {
        Vec3::new(
            x as f32 * STEP - self.size.half(),
            self.heights[z * (self.size.cells() + 1) + x],
            z as f32 * STEP - self.size.half(),
        )
    }

    /// Barycentric interpolation matches the actual rendered triangles, not a bilinear surface.
    pub fn height(&self, x: f32, z: f32) -> Option<f32> {
        let half = self.size.half();
        let cells = self.size.cells();
        if !(-half..=half).contains(&x) || !(-half..=half).contains(&z) {
            return None;
        }
        let gx = (x + half) / STEP;
        let gz = (z + half) / STEP;
        let ix = (gx.floor() as usize).min(cells - 1);
        let iz = (gz.floor() as usize).min(cells - 1);
        let (u, v) = (gx - ix as f32, gz - iz as f32);
        let a = self.vertex(ix, iz).y;
        let b = self.vertex(ix + 1, iz).y;
        let c = self.vertex(ix, iz + 1).y;
        let d = self.vertex(ix + 1, iz + 1).y;
        Some(if u + v <= 1.0 {
            a + u * (b - a) + v * (c - a)
        } else {
            d + (1.0 - u) * (c - d) + (1.0 - v) * (b - d)
        })
    }

    pub fn crater(&mut self, center: Vec3, radius: f32) {
        let cells = self.size.cells();
        for z in 0..=cells {
            for x in 0..=cells {
                let p = self.vertex(x, z);
                let r2 = Vec2::new(p.x - center.x, p.z - center.z).length_squared();
                if r2 < radius * radius {
                    // A lower hemisphere cuts a bowl; never adds material or cuts below bedrock.
                    let bowl = center.y - (radius * radius - r2).sqrt() * 0.72;
                    let i = z * (cells + 1) + x;
                    self.heights[i] = self.heights[i].min(bowl).max(FLOOR);
                }
            }
        }
    }

    pub fn build_mesh(&self) -> Mesh {
        let cells = self.size.cells();
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut colors = Vec::new();
        let mut triangle = |points: [Vec3; 3], color: [f32; 4]| {
            // Vertex colors are linear, while our art palette is authored in sRGB.
            let linear = Color::srgba(color[0], color[1], color[2], color[3]).to_linear();
            let color = [linear.red, linear.green, linear.blue, linear.alpha];
            let normal = (points[1] - points[0])
                .cross(points[2] - points[0])
                .normalize();
            for point in points {
                positions.push(point.to_array());
                normals.push(normal.to_array());
                colors.push(color);
            }
        };
        for z in 0..cells {
            for x in 0..cells {
                let a = self.vertex(x, z);
                let b = self.vertex(x + 1, z);
                let c = self.vertex(x, z + 1);
                let d = self.vertex(x + 1, z + 1);
                let cut = [
                    z * (cells + 1) + x,
                    z * (cells + 1) + x + 1,
                    (z + 1) * (cells + 1) + x,
                    (z + 1) * (cells + 1) + x + 1,
                ]
                .iter()
                .any(|&i| self.original[i] - self.heights[i] > 0.25);
                let variation = ((x * 17 + z * 31) % 11) as f32 * 0.005;
                let color = if cut {
                    [0.29 + variation, 0.17 + variation, 0.075, 1.0]
                } else {
                    let altitude = (a.y / 18.0).clamp(0.0, 1.0);
                    [
                        0.13 + altitude * 0.16 + variation,
                        0.29 + altitude * 0.12 + variation,
                        0.13,
                        1.0,
                    ]
                };
                triangle([a, c, b], color);
                triangle([b, c, d], color);
            }
        }
        // Close the island sides, so craters near the edge never expose a paper-thin landscape.
        for i in 0..cells {
            for (a, b) in [
                (self.vertex(i + 1, 0), self.vertex(i, 0)),
                (self.vertex(i, cells), self.vertex(i + 1, cells)),
                (self.vertex(0, i), self.vertex(0, i + 1)),
                (self.vertex(cells, i + 1), self.vertex(cells, i)),
            ] {
                let low_a = Vec3::new(a.x, FLOOR - 3.0, a.z);
                let low_b = Vec3::new(b.x, FLOOR - 3.0, b.z);
                triangle([a, low_a, b], [0.20, 0.13, 0.08, 1.0]);
                triangle([b, low_a, low_b], [0.20, 0.13, 0.08, 1.0]);
            }
        }
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    }

    /// Exact segment/triangle intersection in the cells covered by the swept shot.
    pub fn sweep(&self, from: Vec3, to: Vec3) -> Option<f32> {
        let half = self.size.half();
        let cells = self.size.cells();
        if self.height(from.x, from.z).is_some_and(|h| from.y <= h) {
            return Some(0.0);
        }
        let lo = from.min(to);
        let hi = from.max(to);
        if hi.x < -half || lo.x > half || hi.z < -half || lo.z > half {
            return None;
        }
        let index =
            |p: f32| (((p + half) / STEP).floor() as isize).clamp(0, cells as isize - 1) as usize;
        let mut best: Option<f32> = None;
        for z in index(lo.z)..=index(hi.z) {
            for x in index(lo.x)..=index(hi.x) {
                let (a, b, c, d) = (
                    self.vertex(x, z),
                    self.vertex(x + 1, z),
                    self.vertex(x, z + 1),
                    self.vertex(x + 1, z + 1),
                );
                for tri in [[a, c, b], [b, c, d]] {
                    if let Some(t) = segment_triangle(from, to, tri) {
                        best = Some(best.map_or(t, |old| old.min(t)));
                    }
                }
            }
        }
        best
    }
}

fn segment_triangle(from: Vec3, to: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
    let direction = to - from;
    let e1 = b - a;
    let e2 = c - a;
    let p = direction.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-7 {
        return None;
    }
    let s = from - a;
    let u = s.dot(p) / det;
    let q = s.cross(e1);
    let v = direction.dot(q) / det;
    let t = e2.dot(q) / det;
    (u >= -1e-5 && v >= -1e-5 && u + v <= 1.00001 && (0.0..=1.0).contains(&t)).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_sizes_keep_cell_resolution_and_collide_at_their_edges() {
        assert_eq!(MapSize::default(), MapSize::Small);
        for (size, width) in MapSize::ALL.into_iter().zip([120.0, 180.0, 240.0]) {
            assert_eq!(size.half() * 2.0, width);
            assert_eq!(size.label().parse::<MapSize>().unwrap(), size);
            let mut terrain = Terrain::new(42, size);
            assert_eq!(terrain.heights.len(), (size.cells() + 1).pow(2));
            assert_eq!(
                terrain.build_mesh().count_vertices(),
                size.cells().pow(2) * 6 + size.cells() * 24
            );
            for sign in [-1.0, 1.0] {
                let edge = sign * size.half();
                assert!(terrain.height(edge, edge).is_some());
                assert!(terrain.height(edge + sign * 0.01, edge).is_none());
                let from = Vec3::new(edge, 40.0, edge);
                let to = Vec3::new(edge, -20.0, edge);
                let hit = from.lerp(to, terrain.sweep(from, to).unwrap());
                assert!((hit.y - terrain.height(edge, edge).unwrap()).abs() < 0.001);
            }
            // Destruction and collision also work beyond the original Small boundary.
            let x = size.half() - 6.0;
            let height = terrain.height(x, 0.0).unwrap();
            terrain.crater(Vec3::new(x, height, 0.0), 8.0);
            let from = Vec3::new(x, 40.0, 0.0);
            let to = Vec3::new(x, -20.0, 0.0);
            let hit = from.lerp(to, terrain.sweep(from, to).unwrap());
            assert!(hit.y < height - 5.0);
            assert!((hit.y - terrain.height(x, 0.0).unwrap()).abs() < 0.001);
        }
        assert!("huge".parse::<MapSize>().is_err());
    }

    #[test]
    fn seeds_reproduce_terrain_and_spawn_pads_are_flat() {
        for size in MapSize::ALL {
            let mut north_south = Vec::new();
            for seed in 0..100 {
                let t = Terrain::new(seed, size);
                assert_eq!(t.heights, Terrain::new(seed, size).heights);
                assert_eq!(t.spawns, size.spawns(seed));
                let reach = 40.0 * size.scale();
                assert!((t.spawns[0].x + reach).abs() < 0.001);
                assert!((t.spawns[1].x - reach).abs() < 0.001);
                for p in t.spawns {
                    assert!(p.y.abs() <= reach + 0.001);
                }
                north_south.push(t.spawns.map(|p| p.y));
                for p in t.spawns.into_iter().chain([size.windsock_site()]) {
                    let h = t.height(p.x, p.y).unwrap();
                    assert!((0.0..=17.0).contains(&h));
                    for offset in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                        assert!(
                            (t.height(p.x + offset.x, p.y + offset.y).unwrap() - h).abs() < 0.001
                        );
                    }
                }
                let sock = size.windsock_site();
                assert!(sock.y < -10.0);
                assert!(sock.y.abs() < size.half());
            }
            assert!(north_south.windows(2).any(|pair| pair[0] != pair[1]));
        }
        assert_ne!(
            Terrain::new(1, MapSize::Small).heights,
            Terrain::new(2, MapSize::Small).heights
        );
    }

    #[test]
    fn crater_lowers_only_nearby_ground_and_respects_bedrock() {
        let mut t = Terrain::new(4, MapSize::Small);
        let far = t.height(40.0, 40.0);
        let h = t.height(0.0, 0.0).unwrap();
        t.crater(Vec3::new(0.0, h, 0.0), 8.0);
        assert!(t.height(0.0, 0.0).unwrap() < h - 5.0);
        assert_eq!(far, t.height(40.0, 40.0));
        for _ in 0..100 {
            t.crater(Vec3::new(0.0, t.height(0.0, 0.0).unwrap(), 0.0), 8.0);
        }
        assert!(t.heights.iter().all(|h| *h >= FLOOR));
    }

    #[test]
    fn fast_shots_hit_the_rendered_surface_before_and_after_destruction() {
        let mut t = Terrain::new(42, MapSize::Small);
        for crater in [false, true] {
            if crater {
                t.crater(Vec3::new(0.0, t.height(0.0, 0.0).unwrap(), 0.0), 8.0);
            }
            for (x, z) in [
                (0.0, 0.0),
                (0.3, 0.7),
                (-12.1, 24.8),
                (t.size.half(), t.size.half()),
            ] {
                let from = Vec3::new(x, 80.0, z);
                let to = Vec3::new(x, -30.0, z);
                let hit = from.lerp(to, t.sweep(from, to).unwrap());
                assert!((hit.y - t.height(x, z).unwrap()).abs() < 0.001);
            }
        }
        assert!(t.sweep(Vec3::splat(200.0), Vec3::splat(300.0)).is_none());
        assert!(
            t.sweep(Vec3::new(-50.0, 80.0, 0.0), Vec3::new(50.0, -40.0, 0.0))
                .is_some()
        );
    }
}
