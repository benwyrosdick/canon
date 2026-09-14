use bevy::prelude::*;
use canon_3d::{
    camera, game,
    net::{self, DEFAULT_RELAY, Hello, Menu, PlayMode, RoomCode},
    smoke, sound, terrain, ui, visuals, windsock,
};

#[derive(Resource)]
struct BootNet {
    hello: Hello,
    relay: String,
    code: RoomCode,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed = args
        .iter()
        .find_map(|arg| {
            arg.strip_prefix("--seed=")
                .and_then(|s| s.parse::<u64>().ok())
        })
        .unwrap_or_else(game::fresh_seed);
    let size = args
        .iter()
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
    let relay = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--relay=").map(str::to_string))
        .unwrap_or_else(|| DEFAULT_RELAY.into());
    let smoke = args.iter().any(|arg| arg == "--smoke-test");
    let local = smoke || args.iter().any(|arg| arg == "--local");
    let host = args.iter().any(|arg| arg == "--host");
    let join = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--join=").and_then(RoomCode::parse));
    if args.iter().any(|arg| arg.starts_with("--join=")) && join.is_none() {
        eprintln!("Invalid --join code. Use 4 characters from 2-9 and A-Z, without 0/O/1/I.");
        std::process::exit(2);
    }
    let terrain = terrain::Terrain::new(seed, size);
    let mut game = game::Game::new(seed, &terrain);
    let play = if local {
        game.paused = false;
        PlayMode::Local
    } else {
        game.paused = true;
        PlayMode::Menu
    };
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
        .insert_resource(play)
        .insert_resource(Menu::new(relay.clone()))
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
            (
                boot_net,
                visuals::setup,
                camera::setup,
                sound::setup,
                (windsock::setup, ui::setup).chain(),
            ),
        )
        .add_systems(FixedUpdate, game::simulate)
        .add_systems(
            Update,
            (
                net::menu_input,
                net::pump,
                net::smooth,
                game::input,
                visuals::sync_environment,
                visuals::sync_cannons,
                visuals::wreck_cannons,
                visuals::animate_debris,
                visuals::projectile,
                visuals::effects,
                visuals::animate_particles,
                camera::update,
                windsock::update,
                ui::update,
            )
                .chain(),
        );
    if host {
        let code = RoomCode::random(seed);
        app.insert_resource(BootNet {
            hello: Hello::Host(code),
            relay,
            code,
        });
    } else if let Some(code) = join {
        app.insert_resource(BootNet {
            hello: Hello::Guest(code),
            relay,
            code,
        });
    }
    if smoke {
        app.add_systems(Update, smoke::run.after(ui::update));
    }
    app.run();
}

fn boot_net(mut commands: Commands, boot: Option<Res<BootNet>>, mut menu: ResMut<Menu>) {
    let Some(boot) = boot else {
        return;
    };
    menu.join_code = boot.code.as_str();
    menu.status = format!("Connecting to {} ({})...", boot.relay, boot.code.as_str());
    net::connect(&mut commands, boot.hello, boot.relay.clone(), boot.code);
}
