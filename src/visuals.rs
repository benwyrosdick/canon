use bevy::{audio::PlaybackSettings, prelude::*};

use crate::{
    camera::CameraRig,
    game::{Effect, Game, Phase},
    sound::Sounds,
    terrain::{FLOOR, Rng, Terrain},
};

pub const RED: Color = Color::srgb(0.96, 0.28, 0.23);
pub const BLUE: Color = Color::srgb(0.22, 0.64, 1.0);
pub const GOLD: Color = Color::srgb(1.0, 0.79, 0.32);

#[derive(Component)]
pub struct CannonRoot(usize);
#[derive(Component)]
pub struct Barrel(usize);
#[derive(Component)]
pub struct Projectile;
#[derive(Component)]
pub struct Battlefield;
#[derive(Component)]
pub struct Water;

#[derive(Component)]
pub struct Particle {
    velocity: Vec3,
    remaining: f32,
    total: f32,
    size: f32,
    seed: u64,
}

#[derive(Resource)]
pub struct Art {
    particle: Handle<Mesh>,
    fire: Handle<StandardMaterial>,
    smoke: Handle<StandardMaterial>,
    earth: Handle<StandardMaterial>,
    rng: Rng,
    smoke_clock: f32,
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain: ResMut<Terrain>,
    game: Res<Game>,
) {
    terrain.mesh = meshes.add(terrain.build_mesh());
    commands.spawn((
        Mesh3d(terrain.mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::default(),
        Name::new("Destructible island"),
        Battlefield,
    ));
    commands.spawn((
        Water,
        Mesh3d(meshes.add(Cuboid::new(450.0, 1.0, 450.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.21, 0.43, 0.51),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_xyz(0.0, FLOOR - 3.2, 0.0),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-40.0, 80.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.045, 0.06, 0.075),
        metallic: 0.4,
        perceptual_roughness: 0.55,
        ..default()
    });
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.29, 0.32),
        metallic: 0.75,
        perceptual_roughness: 0.35,
        ..default()
    });
    let brass = materials.add(StandardMaterial {
        base_color: GOLD,
        metallic: 0.65,
        perceptual_roughness: 0.4,
        ..default()
    });
    let wheel_mesh = meshes.add(Cylinder::new(0.9, 0.45).mesh().resolution(12));
    let hub_mesh = meshes.add(Cylinder::new(0.32, 0.5).mesh().resolution(12));
    let body_mesh = meshes.add(Cuboid::new(2.4, 0.8, 2.4));
    let dome_mesh = meshes.add(Sphere::new(1.05).mesh().ico(1).unwrap());
    let barrel_mesh = meshes.add(Cylinder::new(0.38, 3.0).mesh().resolution(12));
    let ring_mesh = meshes.add(Cylinder::new(0.46, 0.3).mesh().resolution(12));
    let hole_mesh = meshes.add(Cylinder::new(0.30, 0.025).mesh().resolution(12));

    for i in 0..2 {
        let color = materials.add(StandardMaterial {
            base_color: if i == 0 { RED } else { BLUE },
            metallic: 0.15,
            perceptual_roughness: 0.6,
            ..default()
        });
        commands
            .spawn((
                CannonRoot(i),
                Transform::from_translation(game.cannons[i].position),
                Visibility::default(),
                Name::new(format!("Player {}", i + 1)),
            ))
            .with_children(|root| {
                root.spawn((
                    Mesh3d(body_mesh.clone()),
                    MeshMaterial3d(color.clone()),
                    Transform::default(),
                ));
                root.spawn((
                    Mesh3d(dome_mesh.clone()),
                    MeshMaterial3d(color),
                    Transform::from_xyz(0.0, 0.35, 0.0).with_scale(Vec3::new(1.0, 0.7, 1.0)),
                ));
                for x in [-1.35, 1.35] {
                    for z in [-0.75, 0.75] {
                        let transform = Transform::from_xyz(x, -0.35, z)
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
                        root.spawn((
                            Mesh3d(wheel_mesh.clone()),
                            MeshMaterial3d(dark.clone()),
                            transform,
                        ));
                        root.spawn((
                            Mesh3d(hub_mesh.clone()),
                            MeshMaterial3d(brass.clone()),
                            transform,
                        ));
                    }
                }
                root.spawn((
                    Barrel(i),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    pivot.spawn((
                        Mesh3d(barrel_mesh.clone()),
                        MeshMaterial3d(metal.clone()),
                        Transform::from_xyz(0.0, 1.5, 0.0),
                    ));
                    pivot.spawn((
                        Mesh3d(ring_mesh.clone()),
                        MeshMaterial3d(brass.clone()),
                        Transform::from_xyz(0.0, 2.85, 0.0),
                    ));
                    pivot.spawn((
                        Mesh3d(hole_mesh.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::from_xyz(0.0, 3.012, 0.0),
                    ));
                });
            });
    }
    commands.spawn((
        Projectile,
        Mesh3d(meshes.add(Sphere::new(0.28).mesh().ico(2).unwrap())),
        MeshMaterial3d(dark),
        Transform::default(),
        Visibility::Hidden,
    ));
    let particle = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let fire = materials.add(StandardMaterial {
        base_color: GOLD,
        emissive: LinearRgba::new(4.0, 1.5, 0.1, 1.0),
        unlit: true,
        ..default()
    });
    let smoke = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.42, 0.39),
        perceptual_roughness: 1.0,
        ..default()
    });
    let earth = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.20, 0.09),
        ..default()
    });
    commands.insert_resource(Art {
        particle,
        fire,
        smoke,
        earth,
        rng: Rng::new(game.seed),
        smoke_clock: 0.0,
    });
}

pub fn sync_environment(
    mut commands: Commands,
    terrain: Res<Terrain>,
    islands: Query<Entity, With<Battlefield>>,
    mut water: Query<&mut Transform, With<Water>>,
) {
    if terrain.is_changed() {
        // Bevy caches mesh bounds. Recompute after resizing or cutting a crater.
        for entity in &islands {
            commands
                .entity(entity)
                .remove::<bevy::camera::primitives::Aabb>();
        }
        for mut transform in &mut water {
            transform.scale = Vec3::new(terrain.size.scale(), 1.0, terrain.size.scale());
        }
    }
}

pub fn sync_cannons(
    game: Res<Game>,
    mut roots: Query<(&CannonRoot, &mut Transform)>,
    mut barrels: Query<(&Barrel, &mut Transform), Without<CannonRoot>>,
    mut gizmos: Gizmos,
) {
    for (root, mut transform) in &mut roots {
        let cannon = game.cannons[root.0];
        transform.translation = cannon.position;
        transform.rotation = if cannon.health <= 0.0 {
            Quat::from_rotation_z(0.55)
        } else {
            Quat::IDENTITY
        };
    }
    for (barrel, mut transform) in &mut barrels {
        transform.rotation = Quat::from_rotation_arc(Vec3::Y, game.cannons[barrel.0].direction());
    }
    if game.phase == Phase::Aiming {
        let cannon = game.cannons[game.active];
        let color = if game.active == 0 { RED } else { BLUE };
        gizmos.arrow(
            cannon.muzzle(),
            cannon.muzzle() + cannon.direction() * 8.0,
            color,
        );
        gizmos.circle(
            Isometry3d::new(
                cannon.position - Vec3::Y * 1.0,
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            3.0,
            GOLD,
        );
    }
    if game.trail.len() > 1 {
        gizmos.linestrip(
            game.trail.iter().copied(),
            Color::srgba(1.0, 0.92, 0.70, 0.5),
        );
    }
}

pub fn projectile(
    game: Res<Game>,
    mut query: Query<(&mut Transform, &mut Visibility), With<Projectile>>,
) {
    for (mut transform, mut visibility) in &mut query {
        if let Some(ball) = game.ball {
            transform.translation = ball.position;
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn effects(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut art: ResMut<Art>,
    sounds: Res<Sounds>,
    time: Res<Time>,
    mut rig: ResMut<CameraRig>,
) {
    if game.paused {
        return;
    }
    let seed = game.seed;
    let muted = game.muted;
    for effect in std::mem::take(&mut game.effects) {
        let (point, blast) = match effect {
            Effect::Fire(p) => (p, false),
            Effect::Blast(p) => (p, true),
        };
        if !muted {
            commands.spawn((
                AudioPlayer::new(if blast {
                    sounds.impact.clone()
                } else {
                    sounds.fire.clone()
                }),
                PlaybackSettings::DESPAWN,
            ));
        }
        rig.shake = if blast { 0.8 } else { 0.25 };
        for i in 0..if blast { 55 } else { 12 } {
            let velocity = Vec3::new(
                art.rng.range(-1.0, 1.0),
                art.rng.range(0.2, 1.4),
                art.rng.range(-1.0, 1.0),
            ) * if blast { 11.0 } else { 3.0 };
            let total = art.rng.range(0.35, if blast { 1.4 } else { 0.6 });
            let size = art.rng.range(0.15, if blast { 0.9 } else { 0.35 });
            let material = if i % 3 == 0 {
                art.fire.clone()
            } else if i % 3 == 1 {
                art.earth.clone()
            } else {
                art.smoke.clone()
            };
            commands.spawn((
                Mesh3d(art.particle.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(point).with_scale(Vec3::splat(size)),
                Particle {
                    velocity,
                    remaining: total,
                    total,
                    size,
                    seed,
                },
            ));
        }
    }
    art.smoke_clock += time.delta_secs();
    if art.smoke_clock > 0.04 {
        art.smoke_clock = 0.0;
        if let Some(ball) = game.ball {
            commands.spawn((
                Mesh3d(art.particle.clone()),
                MeshMaterial3d(art.smoke.clone()),
                Transform::from_translation(ball.position).with_scale(Vec3::splat(0.2)),
                Particle {
                    velocity: Vec3::Y * 0.6,
                    remaining: 0.7,
                    total: 0.7,
                    size: 0.22,
                    seed,
                },
            ));
        }
    }
}

pub fn animate_particles(
    mut commands: Commands,
    game: Res<Game>,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Particle)>,
) {
    for (entity, mut transform, mut particle) in &mut query {
        if particle.seed != game.seed || game.phase == Phase::Handoff {
            commands.entity(entity).despawn();
            continue;
        }
        if game.paused {
            continue;
        }
        let dt = time.delta_secs().min(0.05);
        particle.remaining -= dt;
        if particle.remaining <= 0.0 {
            commands.entity(entity).despawn();
        } else {
            particle.velocity.y -= 5.0 * dt;
            transform.translation += particle.velocity * dt;
            transform.scale =
                Vec3::splat(particle.size * (particle.remaining / particle.total).sqrt());
        }
    }
}
