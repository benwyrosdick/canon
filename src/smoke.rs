//! Opt-in rendering check: fires a shot, captures gameplay views, then exits.
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

use crate::game::{Game, Phase};

fn snap(commands: &mut Commands, path: &'static str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

pub fn run(
    mut commands: Commands,
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut stage: Local<u8>,
    mut elapsed: Local<f32>,
    mut exit: MessageWriter<AppExit>,
) {
    *elapsed += time.delta_secs();
    match *stage {
        0 if *elapsed > 1.0 => {
            game.phase = Phase::Aiming;
            *stage = 1;
        }
        1 if *elapsed > 3.0 => {
            snap(&mut commands, "screenshots/aim.png");
            game.fire();
            info!("Render smoke test: fired a shot");
            *stage = 2;
        }
        2 if *elapsed > 4.4 && game.ball.is_some() => {
            snap(&mut commands, "screenshots/flight.png");
            info!("Render smoke test: captured flight");
            *stage = 3;
        }
        2 if *elapsed > 4.4 => {
            *stage = 3;
        }
        3 if *elapsed > 10.0
            && game.ball.is_none()
            && !matches!(game.phase, Phase::Flying | Phase::Resolving(_)) =>
        {
            assert!(game.ball.is_none(), "Smoke-test shot should have finished");
            info!("Render smoke test: shot resolved");
            *stage = 4;
            *elapsed = 0.0;
        }
        3 if *elapsed > 25.0 => panic!("Smoke-test shot failed to resolve within 25 seconds"),
        4 if *elapsed > 1.2 => {
            snap(&mut commands, "screenshots/impact.png");
            *stage = 5;
        }
        5 if *elapsed > 3.0 => {
            info!("Render smoke test complete");
            exit.write(AppExit::Success);
            *stage = 6;
        }
        _ => {}
    }
}
