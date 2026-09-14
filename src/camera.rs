use bevy::{
    input::mouse::{AccumulatedMouseMotion, MouseWheel},
    prelude::*,
};

use crate::{
    game::{Cannon, Game, Phase},
    terrain::MapSize,
};

#[derive(Component)]
pub struct MainCamera;

#[derive(Resource)]
pub struct CameraRig {
    yaw: f32,
    pitch: f32,
    radius: f32,
    overview: bool,
    /// Aiming camera stays behind the barrel until unlocked for free orbit.
    locked: bool,
    last_turn: (u64, u32, usize, MapSize),
    pub shake: f32,
}

fn overview_view(scale: f32) -> (Vec3, Vec3) {
    (Vec3::ZERO, Vec3::new(-92.0, 115.0, 150.0) * scale)
}

fn follow_view(cannon: Cannon, yaw: f32, pitch: f32, radius: f32) -> (Vec3, Vec3) {
    let horizontal = Vec3::new(cannon.yaw.cos(), 0.0, cannon.yaw.sin());
    let offset = Vec3::new(
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    ) * radius;
    (cannon.position + horizontal * 6.0, offset)
}

pub fn setup(mut commands: Commands, game: Res<Game>) {
    let cannon = game.cannons[game.active];
    let yaw = cannon.yaw + std::f32::consts::PI;
    let pitch = 0.42;
    let radius = 39.0;
    let (target, offset) = overview_view(game.size.scale());
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        // No lookup textures required: use a tonemapper that is entirely analytic.
        bevy::core_pipeline::tonemapping::Tonemapping::Reinhard,
        Transform::from_translation(target + offset).looking_at(target, Vec3::Y),
        DistanceFog {
            color: Color::srgb(0.57, 0.73, 0.80),
            falloff: FogFalloff::Linear {
                start: 170.0,
                end: 400.0,
            },
            ..default()
        },
    ));
    commands.insert_resource(CameraRig {
        yaw,
        pitch,
        radius,
        overview: false,
        locked: true,
        last_turn: (game.seed, game.round, game.active, game.size),
        shake: 0.0,
    });
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    game: Res<Game>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut scroll: MessageReader<MouseWheel>,
    mut rig: ResMut<CameraRig>,
    mut cameras: Query<(&mut Transform, &mut DistanceFog), With<MainCamera>>,
) {
    let wheel: f32 = scroll.read().map(|event| event.y).sum();
    if game.paused {
        return;
    }
    let turn = (game.seed, game.round, game.active, game.size);
    if rig.last_turn != turn {
        rig.last_turn = turn;
        rig.yaw = game.cannons[game.active].yaw + std::f32::consts::PI;
        rig.pitch = 0.42;
        rig.radius = 39.0;
        rig.overview = false;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        rig.overview = !rig.overview;
    }
    if keys.just_pressed(KeyCode::KeyX) {
        rig.locked = !rig.locked;
    }
    let cannon = game.cannons[game.active];
    if rig.locked {
        rig.yaw = cannon.yaw + std::f32::consts::PI;
    } else if buttons.pressed(MouseButton::Right) {
        rig.yaw -= motion.delta.x * 0.005;
    }
    if buttons.pressed(MouseButton::Right) {
        rig.pitch = (rig.pitch + motion.delta.y * 0.004).clamp(0.15, 1.35);
    }
    rig.radius = (rig.radius - wheel * 2.0).clamp(18.0, 95.0);
    let horizontal = Vec3::new(cannon.yaw.cos(), 0.0, cannon.yaw.sin());
    let overview = rig.overview || matches!(game.phase, Phase::Handoff | Phase::Finished(_));
    let (target, offset) = if overview {
        overview_view(game.size.scale())
    } else if let Some(ball) = game.ball {
        let travel = Vec3::new(ball.velocity.x, 0.0, ball.velocity.z).normalize_or_zero();
        (
            ball.position,
            -travel * 24.0 + Vec3::Y * 12.0 + travel.cross(Vec3::Y) * 8.0,
        )
    } else if matches!(game.phase, Phase::Resolving(_)) {
        let target = game.trail.last().copied().unwrap_or(cannon.position);
        (target, -horizontal * 25.0 + Vec3::Y * 18.0)
    } else {
        follow_view(cannon, rig.yaw, rig.pitch, rig.radius)
    };
    let desired = target + offset;
    let blend = 1.0 - (-5.0 * time.delta_secs()).exp();
    rig.shake = (rig.shake - time.delta_secs() * 1.5).max(0.0);
    let shake = Vec3::new(
        (time.elapsed_secs() * 83.0).sin(),
        (time.elapsed_secs() * 71.0).cos(),
        0.0,
    ) * rig.shake
        * 0.25;
    for (mut transform, mut fog) in &mut cameras {
        fog.falloff = FogFalloff::Linear {
            start: 170.0 * game.size.scale(),
            end: 400.0 * game.size.scale(),
        };
        transform.translation = transform.translation.lerp(desired, blend);
        let rotation = Transform::from_translation(transform.translation)
            .looking_at(target + shake, Vec3::Y)
            .rotation;
        transform.rotation = transform.rotation.slerp(rotation, blend);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game::Game, terrain::Terrain};

    fn camera_app(yaw: f32, locked: bool) -> App {
        let terrain = Terrain::new(1, MapSize::Small);
        let mut game = Game::new(1, &terrain);
        game.phase = Phase::Aiming;
        game.cannons[0].yaw = yaw;
        let mut app = App::new();
        app.add_message::<MouseWheel>();
        app.insert_resource(game)
            .insert_resource(Time::<()>::default())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(AccumulatedMouseMotion::default())
            .insert_resource(CameraRig {
                yaw: yaw + std::f32::consts::PI,
                pitch: 0.42,
                radius: 39.0,
                overview: false,
                locked,
                last_turn: (1, 1, 0, MapSize::Small),
                shake: 0.0,
            })
            .add_systems(Update, update);
        app.world_mut().spawn((
            MainCamera,
            Camera3d::default(),
            Transform::default(),
            DistanceFog::default(),
        ));
        app
    }

    #[test]
    fn x_locks_and_unlocks_the_view_behind_the_turret() {
        let mut app = camera_app(0.4, true);
        app.world_mut().resource_mut::<Game>().cannons[0].yaw = 1.1;
        app.update();
        assert!(
            (app.world().resource::<CameraRig>().yaw - (1.1 + std::f32::consts::PI)).abs() < 0.001
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyX);
        app.update();
        assert!(!app.world().resource::<CameraRig>().locked);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut().resource_mut::<Game>().cannons[0].yaw = 2.0;
        app.update();
        assert!(
            (app.world().resource::<CameraRig>().yaw - (1.1 + std::f32::consts::PI)).abs() < 0.001
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyX);
        app.update();
        let rig = app.world().resource::<CameraRig>();
        assert!(rig.locked);
        assert!((rig.yaw - (2.0 + std::f32::consts::PI)).abs() < 0.001);
    }

    fn camera_eye(app: &mut App) -> Vec3 {
        let mut cameras = app
            .world_mut()
            .query_filtered::<&Transform, With<MainCamera>>();
        cameras.single(app.world()).unwrap().translation
    }

    #[test]
    fn opening_handoff_uses_overview_and_aiming_stays_in_the_tank() {
        let mut app = camera_app(0.4, true);
        app.world_mut().resource_mut::<Game>().phase = Phase::Handoff;
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(4));
        app.insert_resource(time);
        app.update();
        assert!(camera_eye(&mut app).y > 80.0);

        app.world_mut().resource_mut::<Game>().phase = Phase::Aiming;
        app.update();
        let cannon = app.world().resource::<Game>().cannons[0].position;
        let eye = camera_eye(&mut app);
        assert!(eye.distance(cannon) < 50.0);
        assert!(eye.y < 50.0);
    }
}
