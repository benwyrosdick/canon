use bevy::{
    asset::RenderAssetUsages,
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::hierarchy::ChildSpawnerCommands,
    mesh::PrimitiveTopology,
    prelude::*,
    render::render_resource::{TextureDimension, TextureFormat, TextureUsages},
};

use crate::{
    camera::MainCamera,
    game::Game,
    terrain::{MapSize, Terrain},
};

const POLE_HEIGHT: f32 = 12.0;
const MOUTH_RADIUS: f32 = 1.1;
const LENGTH: f32 = 6.5;
const BANDS: usize = 5;
const SECTIONS: usize = BANDS * 3;
const SIDES: usize = 16;

const HUD_LAYER: usize = 1;
const HUD_ORIGIN: Vec3 = Vec3::ZERO;
const HUD_DISTANCE: f32 = 10.0;
const HUD_HEIGHT: f32 = 26.0;
const HUD_LOOK_Y: f32 = 4.0;

#[derive(Component)]
pub struct Mast;

#[derive(Component)]
pub struct Swivel;

#[derive(Component)]
pub struct HudWind;

#[derive(Component)]
pub struct HudWindCamera;

#[derive(Resource, Clone, Copy)]
pub struct HudWindView {
    pub camera: Entity,
}

#[derive(Resource)]
pub struct Fabric(Handle<Mesh>);

struct SockParts {
    metal: Handle<StandardMaterial>,
    concrete: Handle<StandardMaterial>,
    fabric: Handle<StandardMaterial>,
    rose_paint: Handle<StandardMaterial>,
    cloth: Handle<Mesh>,
    pole: Handle<Mesh>,
    foot: Handle<Mesh>,
    ring: Handle<Mesh>,
    cap: Handle<Mesh>,
    rose: Handle<Mesh>,
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    terrain: Res<Terrain>,
    game: Res<Game>,
) {
    let parts = SockParts {
        metal: materials.add(StandardMaterial {
            base_color: Color::srgb(0.65, 0.71, 0.74),
            metallic: 0.65,
            perceptual_roughness: 0.45,
            ..default()
        }),
        concrete: materials.add(Color::srgb(0.45, 0.46, 0.42)),
        fabric: materials.add(StandardMaterial {
            // The tube is open at both ends, with visible fabric on the inside as well.
            cull_mode: None,
            double_sided: true,
            perceptual_roughness: 1.0,
            ..default()
        }),
        rose_paint: materials.add(StandardMaterial {
            perceptual_roughness: 1.0,
            ..default()
        }),
        cloth: meshes.add(fabric_mesh(game.wind.length(), 0.0)),
        pole: meshes.add(Cylinder::new(0.16, POLE_HEIGHT).mesh().resolution(12)),
        foot: meshes.add(Cylinder::new(0.85, 0.3).mesh().resolution(8)),
        ring: meshes.add(Torus::new(MOUTH_RADIUS - 0.045, MOUTH_RADIUS + 0.045)),
        cap: meshes.add(Sphere::new(0.24).mesh().ico(1).unwrap()),
        rose: meshes.add(compass_rose_mesh()),
    };

    let site = terrain.size.windsock_site();
    commands
        .spawn((
            Mast,
            Name::new("North windsock"),
            Transform::from_xyz(site.x, terrain.height(site.x, site.y).unwrap(), site.y),
            Visibility::default(),
        ))
        .with_children(|mast| {
            attach_sock(mast, RenderLayers::default(), &parts, game.wind);
        });

    let hud_layer = RenderLayers::layer(HUD_LAYER);
    commands
        .spawn((
            Mast,
            HudWind,
            Name::new("HUD windsock"),
            Transform::from_translation(HUD_ORIGIN),
            Visibility::default(),
            hud_layer.clone(),
        ))
        .with_children(|mast| {
            attach_sock(mast, hud_layer.clone(), &parts, game.wind);
        });
    commands.spawn((
        DirectionalLight {
            illuminance: 16000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-40.0, 80.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        hud_layer.clone(),
    ));
    commands.spawn((
        PointLight {
            intensity: 1_200_000.0,
            range: 50.0,
            shadows_enabled: false,
            color: Color::srgb(1.0, 0.96, 0.9),
            ..default()
        },
        Transform::from_xyz(6.0, 18.0, 8.0),
        hud_layer.clone(),
    ));

    let mut image = Image::new_uninit(
        default(),
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image_handle = images.add(image);
    let camera = commands
        .spawn((
            HudWindCamera,
            Camera3d::default(),
            Projection::from(PerspectiveProjection {
                fov: 0.7,
                ..default()
            }),
            Camera {
                order: -1,
                target: RenderTarget::Image(image_handle.into()),
                clear_color: ClearColorConfig::Custom(Color::srgb(0.07, 0.10, 0.11)),
                ..default()
            },
            bevy::core_pipeline::tonemapping::Tonemapping::Reinhard,
            hud_view(Vec3::NEG_Z),
            hud_layer,
        ))
        .id();
    commands.insert_resource(HudWindView { camera });
    commands.insert_resource(Fabric(parts.cloth));
}

fn attach_sock(
    mast: &mut ChildSpawnerCommands,
    layer: RenderLayers,
    parts: &SockParts,
    wind: Vec3,
) {
    mast.spawn((
        Mesh3d(parts.rose.clone()),
        MeshMaterial3d(parts.rose_paint.clone()),
        Transform::from_xyz(0.0, 0.04, 0.0),
        layer.clone(),
    ));
    mast.spawn((
        Mesh3d(parts.foot.clone()),
        MeshMaterial3d(parts.concrete.clone()),
        Transform::from_xyz(0.0, 0.18, 0.0),
        layer.clone(),
    ));
    mast.spawn((
        Mesh3d(parts.pole.clone()),
        MeshMaterial3d(parts.metal.clone()),
        Transform::from_xyz(0.0, POLE_HEIGHT / 2.0, 0.0),
        layer.clone(),
    ));
    mast.spawn((
        Mesh3d(parts.cap.clone()),
        MeshMaterial3d(parts.metal.clone()),
        Transform::from_xyz(0.0, POLE_HEIGHT, 0.0),
        layer.clone(),
    ));
    mast.spawn((
        Swivel,
        Transform::from_xyz(0.0, POLE_HEIGHT + MOUTH_RADIUS, 0.0).with_rotation(heading(wind)),
        Visibility::default(),
        layer.clone(),
    ))
    .with_children(|sock| {
        sock.spawn((
            Mesh3d(parts.ring.clone()),
            MeshMaterial3d(parts.metal.clone()),
            Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            layer.clone(),
        ));
        sock.spawn((
            Mesh3d(parts.cloth.clone()),
            MeshMaterial3d(parts.fabric.clone()),
            Transform::default(),
            // The fabric changes shape every frame; its initial bounds are not permanent.
            bevy::camera::visibility::NoFrustumCulling,
            layer,
        ));
    });
}

fn heading(wind: Vec3) -> Quat {
    let horizontal = Vec3::new(wind.x, 0.0, wind.z);
    let direction = horizontal.try_normalize().unwrap_or(Vec3::X);
    // The mouth is fixed at the pole and the narrow tail points downwind.
    Quat::from_rotation_arc(Vec3::X, direction)
}

/// Ground-plane forward of a camera. Falls back to world north when looking straight down.
fn camera_heading(transform: &Transform) -> Vec3 {
    let forward = *transform.forward();
    Vec3::new(forward.x, 0.0, forward.z)
        .try_normalize()
        .unwrap_or(Vec3::NEG_Z)
}

/// Orbit the HUD sock so the top of the pane is the main camera's heading.
fn hud_view(heading: Vec3) -> Transform {
    let eye = HUD_ORIGIN - heading * HUD_DISTANCE + Vec3::Y * HUD_HEIGHT;
    Transform::from_translation(eye).looking_at(HUD_ORIGIN + Vec3::Y * HUD_LOOK_Y, Vec3::Y)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update(
    game: Res<Game>,
    terrain: Res<Terrain>,
    time: Res<Time>,
    fabric: Res<Fabric>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut masts: Query<&mut Transform, (With<Mast>, Without<HudWind>, Without<HudWindCamera>)>,
    mut swivels: Query<
        &mut Transform,
        (
            With<Swivel>,
            Without<Mast>,
            Without<HudWind>,
            Without<HudWindCamera>,
        ),
    >,
    mut hud_cameras: Query<
        &mut Transform,
        (
            With<HudWindCamera>,
            Without<Mast>,
            Without<HudWind>,
            Without<Swivel>,
        ),
    >,
    main_cameras: Query<
        &Transform,
        (
            With<MainCamera>,
            Without<HudWindCamera>,
            Without<Mast>,
            Without<HudWind>,
            Without<Swivel>,
        ),
    >,
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
    if let Ok(main) = main_cameras.single() {
        let view = hud_view(camera_heading(main));
        for mut transform in &mut hud_cameras {
            *transform = view;
        }
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
    let inflation = (speed / 25.0).clamp(0.0, 1.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game::Game, terrain::Terrain};

    #[test]
    fn hud_view_puts_the_camera_heading_at_the_top() {
        let north = hud_view(Vec3::NEG_Z);
        assert!(north.translation.z > 0.0);
        assert!(north.translation.x.abs() < 0.001);

        let east = hud_view(Vec3::X);
        assert!(east.translation.x < 0.0);
        assert!(east.translation.z.abs() < 0.001);
    }

    #[test]
    fn camera_heading_ignores_pitch() {
        let transform = Transform::from_xyz(10.0, 8.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y);
        let heading = camera_heading(&transform);
        let expected = Vec3::new(-1.0, 0.0, -1.0).normalize();
        assert!((heading - expected).length() < 0.001);
        assert!(heading.y.abs() < 0.001);
    }

    fn hud_app(main: Transform) -> (App, Entity) {
        let terrain = Terrain::new(1, MapSize::Small);
        let game = Game::new(1, &terrain);
        let mut app = App::new();
        app.insert_resource(game)
            .insert_resource(terrain)
            .insert_resource(Time::<()>::default())
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Fabric(Handle::default()))
            .add_systems(Update, update);
        app.world_mut()
            .spawn((MainCamera, Camera3d::default(), main));
        let hud = app
            .world_mut()
            .spawn((HudWindCamera, Camera3d::default(), Transform::default()))
            .id();
        (app, hud)
    }

    #[test]
    fn hud_camera_orbits_with_the_main_view() {
        let (mut app, hud) =
            hud_app(Transform::from_xyz(0.0, 10.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y));
        app.update();
        let transform = app.world().get::<Transform>(hud).unwrap();
        assert!((transform.translation.z - HUD_DISTANCE).abs() < 0.5);
        assert!(transform.translation.x.abs() < 0.5);
        assert!((transform.translation.y - HUD_HEIGHT).abs() < 0.5);

        let (mut app, hud) =
            hud_app(Transform::from_xyz(-40.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y));
        app.update();
        let transform = app.world().get::<Transform>(hud).unwrap();
        assert!((transform.translation.x + HUD_DISTANCE).abs() < 0.5);
        assert!(transform.translation.z.abs() < 0.5);
    }
}
