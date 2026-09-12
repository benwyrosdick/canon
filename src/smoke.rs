//! Opt-in rendering check: fires a shot, captures both views, then exits.
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

use crate::game::{Game, Phase};

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
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("target/smoke-aim.png"));
            game.fire();
            info!("Render smoke test: fired a shot");
            *stage = 2;
        }
        2 if *elapsed > 10.0 => {
            assert!(game.ball.is_none(), "Smoke-test shot should have finished");
            assert!(
                matches!(game.phase, Phase::Handoff | Phase::Finished(_)),
                "Shot should have resolved"
            );
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("target/smoke-impact.png"));
            info!("Render smoke test: shot resolved, capturing battlefield");
            *stage = 3;
        }
        3 if *elapsed > 12.0 => {
            info!("Render smoke test complete");
            exit.write(AppExit::Success);
            *stage = 4;
        }
        _ => {}
    }
}
