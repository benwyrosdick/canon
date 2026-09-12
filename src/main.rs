mod camera;
mod game;
mod physics;
mod smoke;
mod sound;
mod terrain;
mod ui;
mod visuals;

use bevy::prelude::*;

fn main() {
    let seed = std::env::args()
        .skip(1)
        .find_map(|arg| {
            arg.strip_prefix("--seed=")
                .and_then(|s| s.parse::<u64>().ok())
        })
        .unwrap_or_else(game::fresh_seed);
    let size = std::env::args()
        .find_map(|arg| {
            arg.strip_prefix("--size=")
                .map(|value| value.parse::<terrain::MapSize>())
        })
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2);
        })
        .unwrap_or_default();
    let terrain = terrain::Terrain::new(seed, size);
    let game = game::Game::new(seed, &terrain);
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.57, 0.73, 0.80)))
        .insert_resource(AmbientLight {
            color: Color::srgb(0.80, 0.88, 1.0),
            brightness: 450.0,
            ..default()
        })
        .insert_resource(Time::<Fixed>::from_hz(120.0))
        .insert_resource(terrain)
        .insert_resource(game)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "3D CANON — Wind & Warfare".into(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(
            Startup,
            (visuals::setup, camera::setup, ui::setup, sound::setup),
        )
        .add_systems(FixedUpdate, game::simulate)
        .add_systems(
            Update,
            (
                game::input,
                visuals::sync_environment,
                visuals::sync_cannons,
                visuals::projectile,
                visuals::effects,
                visuals::animate_particles,
                camera::update,
                ui::update,
            )
                .chain(),
        );
    if std::env::args().any(|arg| arg == "--smoke-test") {
        app.add_systems(Update, smoke::run.after(ui::update));
    }
    app.run();
}
