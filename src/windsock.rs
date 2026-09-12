use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*};

use crate::{
    game::Game,
    terrain::{MapSize, Terrain},
};

const POLE_HEIGHT: f32 = 12.0;
const MOUTH_RADIUS: f32 = 1.1;
const LENGTH: f32 = 6.5;
const BANDS: usize = 5;
const SECTIONS: usize = BANDS * 3;
const SIDES: usize = 16;

#[derive(Component)]
pub struct Mast;

#[derive(Component)]
pub struct Swivel;

#[derive(Resource)]
pub struct Fabric(Handle<Mesh>);

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain: Res<Terrain>,
    game: Res<Game>,
) {
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.71, 0.74),
        metallic: 0.65,
        perceptual_roughness: 0.45,
        ..default()
    });
    let concrete = materials.add(Color::srgb(0.45, 0.46, 0.42));
    let fabric = materials.add(StandardMaterial {
        // The tube is open at both ends, with visible fabric on the inside as well.
        cull_mode: None,
        double_sided: true,
        perceptual_roughness: 1.0,
        ..default()
    });
    let cloth = meshes.add(fabric_mesh(game.wind.length(), 0.0));
    let pole = meshes.add(Cylinder::new(0.16, POLE_HEIGHT).mesh().resolution(12));
    let foot = meshes.add(Cylinder::new(0.85, 0.3).mesh().resolution(8));
    let ring = meshes.add(Torus::new(MOUTH_RADIUS - 0.045, MOUTH_RADIUS + 0.045));
    let cap = meshes.add(Sphere::new(0.24).mesh().ico(1).unwrap());

    let site = terrain.size.windsock_site();
    commands
        .spawn((
            Mast,
            Name::new("North windsock"),
            Transform::from_xyz(site.x, terrain.height(site.x, site.y).unwrap(), site.y),
            Visibility::default(),
        ))
        .with_children(|mast| {
            mast.spawn((
                Mesh3d(meshes.add(compass_rose_mesh())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    perceptual_roughness: 1.0,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.04, 0.0),
            ));
            mast.spawn((
                Mesh3d(foot),
                MeshMaterial3d(concrete),
                Transform::from_xyz(0.0, 0.18, 0.0),
            ));
            mast.spawn((
                Mesh3d(pole),
                MeshMaterial3d(metal.clone()),
                Transform::from_xyz(0.0, POLE_HEIGHT / 2.0, 0.0),
            ));
            mast.spawn((
                Mesh3d(cap),
                MeshMaterial3d(metal.clone()),
                Transform::from_xyz(0.0, POLE_HEIGHT, 0.0),
            ));
            mast.spawn((
                Swivel,
                Transform::from_xyz(0.0, POLE_HEIGHT + MOUTH_RADIUS, 0.0)
                    .with_rotation(heading(game.wind)),
                Visibility::default(),
            ))
            .with_children(|sock| {
                sock.spawn((
                    Mesh3d(ring),
                    MeshMaterial3d(metal),
                    Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                ));
                sock.spawn((
                    Mesh3d(cloth.clone()),
                    MeshMaterial3d(fabric),
                    Transform::default(),
                    // The fabric changes shape every frame; its initial bounds are not permanent.
                    bevy::camera::visibility::NoFrustumCulling,
                ));
            });
        });
    commands.insert_resource(Fabric(cloth));
}

fn heading(wind: Vec3) -> Quat {
    let horizontal = Vec3::new(wind.x, 0.0, wind.z);
    let direction = horizontal.try_normalize().unwrap_or(Vec3::X);
    // The mouth is fixed at the pole and the narrow tail points downwind.
    Quat::from_rotation_arc(Vec3::X, direction)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update(
    game: Res<Game>,
    terrain: Res<Terrain>,
    time: Res<Time>,
    fabric: Res<Fabric>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut masts: Query<&mut Transform, With<Mast>>,
    mut swivels: Query<&mut Transform, (With<Swivel>, Without<Mast>)>,
    mut clock: Local<f32>,
    mut last_map: Local<Option<(u64, MapSize)>>,
) {
    if terrain.is_changed() {
        let site = terrain.size.windsock_site();
        for mut transform in &mut masts {
            transform.translation =
                Vec3::new(site.x, terrain.height(site.x, site.y).unwrap(), site.y);
        }
    }
    let map = (game.seed, game.size);
    if *last_map != Some(map) {
        *last_map = Some(map);
        *clock = 0.0;
    }
    if game.paused {
        return;
    }
    *clock += time.delta_secs();
    for mut transform in &mut swivels {
        transform.rotation = heading(game.wind);
    }
    if let Some(mesh) = meshes.get_mut(&fabric.0) {
        *mesh = fabric_mesh(game.wind.length(), *clock);
    }
}

/// A tapered, open tube. The mouth stays circular and pinned to its swivel ring.
/// More wind lifts the tail and inflates the fabric; flutter grows toward the tip.
fn fabric_mesh(speed: f32, time: f32) -> Mesh {
    let inflation = (speed / 12.0).clamp(0.0, 1.0);
    let vertex = |section: usize, side: usize| {
        let t = section as f32 / SECTIONS as f32;
        let angle = side as f32 / SIDES as f32 * std::f32::consts::TAU;
        let flutter = t * t * inflation.sqrt();
        let phase = time * (2.5 + speed * 0.35) - t * 7.0;
        let center = Vec3::new(
            LENGTH * t * (0.35 + 0.65 * inflation),
            -LENGTH * t * t * (0.8 - 0.77 * inflation) + phase.sin() * flutter * 0.15,
            phase.cos() * flutter * 0.25,
        );
        // Radial basis follows the sagging tube rather than slicing it vertically.
        let tangent = Vec3::new(
            0.35 + 0.65 * inflation,
            -2.0 * t * (0.8 - 0.77 * inflation),
            0.0,
        )
        .normalize();
        let up = Vec3::new(-tangent.y, tangent.x, 0.0);
        let radius = MOUTH_RADIUS * (1.0 - 0.77 * t);
        let fullness = 1.0 - 0.45 * t * (1.0 - inflation);
        center + up * (angle.cos() * radius * fullness) + Vec3::Z * (angle.sin() * radius)
    };
    let mut positions = Vec::with_capacity(SECTIONS * SIDES * 6);
    let mut normals = Vec::with_capacity(SECTIONS * SIDES * 6);
    let mut colors = Vec::with_capacity(SECTIONS * SIDES * 6);
    for section in 0..SECTIONS {
        let color = if (section / 3) % 2 == 0 {
            Color::srgb(1.0, 0.23, 0.035)
        } else {
            Color::srgb(0.96, 0.96, 0.90)
        }
        .to_linear();
        for side in 0..SIDES {
            let a = vertex(section, side);
            let b = vertex(section + 1, side);
            let c = vertex(section, side + 1);
            let d = vertex(section + 1, side + 1);
            for triangle in [[a, c, b], [b, c, d]] {
                let normal = (triangle[1] - triangle[0])
                    .cross(triangle[2] - triangle[0])
                    .normalize();
                for point in triangle {
                    positions.push(point.to_array());
                    normals.push(normal.to_array());
                    colors.push([color.red, color.green, color.blue, color.alpha]);
                }
            }
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

/// Painted airport compass on the XZ plane. −Z is north, +X is east.
fn compass_rose_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut tri = |points: [Vec3; 3], color: Color| {
        let linear = color.to_linear();
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
    let point =
        |angle: f32, radius: f32, y: f32| Vec3::new(angle.sin() * radius, y, -angle.cos() * radius);
    let stone = Color::srgb(0.42, 0.40, 0.36);
    let ink = Color::srgb(0.12, 0.13, 0.14);
    let paper = Color::srgb(0.93, 0.91, 0.84);
    let gold = Color::srgb(0.92, 0.72, 0.18);
    {
        let mut ring = |inner: f32, outer: f32, y: f32, color: Color| {
            for i in 0..64 {
                let a0 = i as f32 / 64.0 * std::f32::consts::TAU;
                let a1 = (i + 1) as f32 / 64.0 * std::f32::consts::TAU;
                tri(
                    [
                        point(a0, inner, y),
                        point(a1, outer, y),
                        point(a0, outer, y),
                    ],
                    color,
                );
                tri(
                    [
                        point(a0, inner, y),
                        point(a1, inner, y),
                        point(a1, outer, y),
                    ],
                    color,
                );
            }
        };
        ring(0.0, 8.6, 0.0, stone);
        ring(7.7, 8.4, 0.03, ink);
        ring(1.1, 7.7, 0.03, Color::srgb(0.34, 0.33, 0.30));
        ring(0.0, 1.15, 0.04, ink);
        ring(0.0, 0.55, 0.05, gold);
    }
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::FRAC_PI_4;
        let cardinal = i % 2 == 0;
        let north = i == 0;
        let tip = if cardinal { 7.35 } else { 5.4 };
        let half_width = if cardinal { 0.55 } else { 0.28 };
        let color = if north {
            gold
        } else if cardinal {
            paper
        } else {
            ink
        };
        let left = point(angle - 0.08, half_width, 0.05);
        let right = point(angle + 0.08, half_width, 0.05);
        tri([point(angle, tip, 0.05), left, right], color);
        tri(
            [
                point(angle, 1.2, 0.05),
                point(angle + 0.04, half_width * 0.4, 0.05),
                point(angle, tip * 0.42, 0.05),
            ],
            color,
        );
        tri(
            [
                point(angle, 1.2, 0.05),
                point(angle, tip * 0.42, 0.05),
                point(angle - 0.04, half_width * 0.4, 0.05),
            ],
            color,
        );
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}
