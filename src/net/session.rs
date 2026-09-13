use bevy::prelude::*;

use super::{
    client::{self, Incoming, Link},
    protocol::Msg,
    wire::{Hello, PING, RoomCode, error_text},
};
use crate::{
    game::{Effect, Game, Phase},
    physics::BLAST_RADIUS,
    terrain::Terrain,
};

pub const DEFAULT_RELAY: &str = "192.241.147.149:3478";

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub enum PlayMode {
    Menu,
    Local,
    Online,
}

impl PlayMode {
    pub fn blocks_world(&self) -> bool {
        matches!(self, Self::Menu)
    }
}

#[derive(Resource)]
pub struct Menu {
    pub join_code: String,
    pub relay: String,
    pub status: String,
}

impl Menu {
    pub fn new(relay: String) -> Self {
        Self {
            join_code: String::new(),
            relay,
            status: String::new(),
        }
    }
}

#[derive(Resource)]
pub struct Net {
    pub link: Link,
    pub seat: usize,
    pub code: RoomCode,
    pub waiting: bool,
    pub aim_timer: f32,
    ping_timer: f32,
    ball_timer: f32,
    remote_yaw: f32,
    remote_elevation: f32,
    remote_power: f32,
    remote_z: f32,
    has_remote_aim: bool,
}

impl Net {
    pub fn send(&self, msg: &Msg) {
        self.link.send_bytes(&msg.encode());
    }

    pub fn is_host(&self) -> bool {
        self.seat == 0 && !self.waiting
    }

    pub fn my_turn(&self, game: &Game) -> bool {
        !self.waiting && game.active == self.seat
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Local,
    Host,
    Join,
    Back,
    QuitToMenu,
}

pub fn is_authority(mode: Option<&PlayMode>, net: Option<&Net>) -> bool {
    match mode {
        Some(PlayMode::Menu) => false,
        Some(PlayMode::Online) => net.is_some_and(|net| net.is_host()),
        _ => true,
    }
}

pub fn my_turn(mode: Option<&PlayMode>, net: Option<&Net>, game: &Game) -> bool {
    match mode {
        Some(PlayMode::Menu) => false,
        Some(PlayMode::Online) => net.is_some_and(|net| net.my_turn(game)),
        _ => true,
    }
}

pub fn connect(commands: &mut Commands, hello: Hello, relay: String, code: RoomCode) {
    commands.insert_resource(Net {
        link: client::spawn(relay, hello),
        seat: usize::from(matches!(hello, Hello::Guest(_))),
        code,
        waiting: true,
        aim_timer: 0.0,
        ping_timer: 0.0,
        ball_timer: 0.0,
        remote_yaw: 0.0,
        remote_elevation: 0.0,
        remote_power: 32.0,
        remote_z: 0.0,
        has_remote_aim: false,
    });
}

#[allow(clippy::too_many_arguments)]
pub fn pump(
    mut commands: Commands,
    time: Res<Time>,
    mode: Option<ResMut<PlayMode>>,
    mut menu: Option<ResMut<Menu>>,
    net: Option<ResMut<Net>>,
    mut game: ResMut<Game>,
    mut terrain: ResMut<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(mut mode) = mode else {
        return;
    };
    let Some(mut net) = net else {
        return;
    };
    let mut events = Vec::new();
    while let Ok(event) = net.link.rx.lock().expect("net inbox").try_recv() {
        events.push(event);
    }
    let mut drop_net = false;
    let mut status = None;
    for event in events {
        match event {
            Incoming::Joined { seat } => {
                net.seat = seat as usize;
                if let Some(menu) = menu.as_deref_mut() {
                    menu.status = if seat == 0 {
                        format!("Room {}. Waiting for Blue...", net.code.as_str())
                    } else {
                        "Connected. Waiting for the host...".into()
                    };
                }
            }
            Incoming::PeerJoined => {
                if net.seat == 0 {
                    net.waiting = false;
                    *mode = PlayMode::Online;
                    game.paused = false;
                    net.send(&Msg::Welcome {
                        seed: game.seed,
                        size: game.size,
                    });
                    if let Some(menu) = menu.as_deref_mut() {
                        menu.status.clear();
                    }
                }
            }
            Incoming::Game(bytes) => {
                if let Some(msg) = Msg::decode(&bytes) {
                    apply(
                        msg,
                        &mut net,
                        &mut mode,
                        &mut game,
                        &mut terrain,
                        &mut meshes,
                    );
                }
            }
            Incoming::Error(code) => {
                status = Some(error_text(code).to_string());
                drop_net = true;
            }
            Incoming::PeerLeft | Incoming::Closed => {
                status = Some("The other player disconnected.".into());
                drop_net = true;
            }
        }
    }
    net.ping_timer += time.delta_secs();
    if net.ping_timer >= 10.0 {
        net.ping_timer = 0.0;
        net.link.send_bytes(&[PING]);
    }
    if *mode == PlayMode::Online && !net.waiting && net.is_host() && game.phase == Phase::Flying {
        net.ball_timer += time.delta_secs();
        if net.ball_timer >= 0.05 {
            net.ball_timer = 0.0;
            if let Some(ball) = game.ball {
                net.send(&Msg::Ball {
                    position: ball.position,
                    velocity: ball.velocity,
                });
            }
        }
    }
    if drop_net {
        game.paused = true;
        *mode = PlayMode::Menu;
        if let Some(menu) = menu.as_deref_mut() {
            menu.status = status.unwrap_or_default();
        }
        commands.remove_resource::<Net>();
    }
}

fn apply(
    msg: Msg,
    net: &mut Net,
    mode: &mut PlayMode,
    game: &mut Game,
    terrain: &mut Terrain,
    meshes: &mut Assets<Mesh>,
) {
    match msg {
        Msg::Welcome { seed, size } => {
            Game::restart_world(seed, size, game, terrain, meshes);
            net.waiting = false;
            *mode = PlayMode::Online;
            game.paused = false;
        }
        Msg::NewMatch { seed, size } => {
            Game::restart_world(seed, size, game, terrain, meshes);
        }
        Msg::Aim {
            yaw,
            elevation,
            power,
            z,
        } => {
            net.remote_yaw = yaw;
            net.remote_elevation = elevation;
            net.remote_power = power;
            net.remote_z = z;
            if !net.has_remote_aim {
                let i = 1 - net.seat;
                let reach = game.size.lane_reach();
                let cannon = &mut game.cannons[i];
                cannon.yaw = yaw;
                cannon.elevation = elevation;
                cannon.power = power;
                cannon.place_on_lane(z, reach, terrain);
                net.has_remote_aim = true;
            }
        }
        Msg::Ready if net.is_host() && game.active != net.seat && game.phase == Phase::Handoff => {
            game.phase = Phase::Aiming;
            net.send(&Msg::Ready);
        }
        Msg::Ready if !net.is_host() => {
            game.phase = Phase::Aiming;
        }
        Msg::Fire {
            yaw,
            elevation,
            power,
            z,
        } if net.is_host() && game.active != net.seat => {
            let reach = game.size.lane_reach();
            let cannon = &mut game.cannons[game.active];
            cannon.yaw = yaw;
            cannon.elevation = elevation;
            cannon.power = power;
            cannon.place_on_lane(z, reach, terrain);
            game.fire();
            net.send(&Msg::ShotFired {
                yaw,
                elevation,
                power,
                z,
            });
        }
        Msg::ShotFired {
            yaw,
            elevation,
            power,
            z,
        } if !net.is_host() => {
            let reach = game.size.lane_reach();
            let cannon = &mut game.cannons[game.active];
            cannon.yaw = yaw;
            cannon.elevation = elevation;
            cannon.power = power;
            cannon.place_on_lane(z, reach, terrain);
            net.remote_yaw = yaw;
            net.remote_elevation = elevation;
            net.remote_power = power;
            net.remote_z = z;
            net.has_remote_aim = true;
            game.paused = false;
            game.fire();
        }
        Msg::Ball { position, velocity } if !net.is_host() => {
            if let Some(ball) = &mut game.ball {
                ball.position = ball.position.lerp(position, 0.2);
                ball.velocity = ball.velocity.lerp(velocity, 0.2);
            }
        }
        Msg::Impact {
            point,
            damage,
            health,
            cannon_y,
            direct,
        } if !net.is_host() => {
            for (i, cannon) in game.cannons.iter_mut().enumerate() {
                cannon.health = health[i];
                cannon.position.y = cannon_y[i];
            }
            terrain.crater(point, BLAST_RADIUS);
            if let Some(mesh) = meshes.get_mut(&terrain.mesh) {
                *mesh = terrain.build_mesh();
            }
            game.message = if direct {
                "DIRECT HIT!".into()
            } else if damage.iter().any(|d| *d > 0.0) {
                format!(
                    "Splash damage - Red -{:.0} / Blue -{:.0}",
                    damage[0], damage[1]
                )
            } else {
                "Terrain hit. A new crater, a new possibility.".into()
            };
            game.trail.push(point);
            game.effects.push(Effect::Blast(point));
            game.ball = None;
            game.phase = Phase::Resolving(1.5);
        }
        Msg::Miss { cannon_y } if !net.is_host() => {
            for (i, cannon) in game.cannons.iter_mut().enumerate() {
                cannon.position.y = cannon_y[i];
            }
            game.ball = None;
            game.phase = Phase::Resolving(0.8);
            game.message = "Out of bounds. Adjust your aim and power next turn.".into();
        }
        Msg::Turn {
            active,
            round,
            wind,
            health,
            cannon_y,
            winner,
        } if !net.is_host() => {
            game.active = active;
            game.round = round;
            game.wind = wind;
            for (i, cannon) in game.cannons.iter_mut().enumerate() {
                cannon.health = health[i];
                cannon.position.y = cannon_y[i];
            }
            game.ball = None;
            game.phase = match winner {
                Some(winner) => Phase::Finished(winner),
                None => Phase::Aiming,
            };
        }
        _ => {}
    }
}

pub fn smooth(
    time: Res<Time>,
    net: Option<Res<Net>>,
    mut game: ResMut<Game>,
    terrain: Res<Terrain>,
) {
    let Some(net) = net else {
        return;
    };
    if net.waiting || !net.has_remote_aim {
        return;
    }
    let blend = 1.0 - (-16.0 * time.delta_secs()).exp();
    let reach = game.size.lane_reach();
    let aiming = game.phase == Phase::Aiming;
    let cannon = &mut game.cannons[1 - net.seat];
    cannon.yaw = lerp_angle(cannon.yaw, net.remote_yaw, blend);
    cannon.elevation += (net.remote_elevation - cannon.elevation) * blend;
    cannon.power += (net.remote_power - cannon.power) * blend;
    if aiming {
        let z = cannon.position.z + (net.remote_z - cannon.position.z) * blend;
        cannon.place_on_lane(z, reach, &terrain);
    }
}

fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let delta =
        (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    (from + delta * t).rem_euclid(std::f32::consts::TAU)
}

pub fn menu_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<PlayMode>,
    mut menu: ResMut<Menu>,
    mut game: ResMut<Game>,
    net: Option<Res<Net>>,
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
) {
    if *mode == PlayMode::Online && !net.as_ref().is_some_and(|net| net.waiting) {
        return;
    }
    let mut action = None;
    for (interaction, pressed) in &buttons {
        if *interaction == Interaction::Pressed {
            action = Some(*pressed);
        }
    }
    if *mode == PlayMode::Local {
        if game.paused
            && (action == Some(MenuAction::QuitToMenu) || keys.just_pressed(KeyCode::Tab))
        {
            *mode = PlayMode::Menu;
            game.paused = true;
            menu.status.clear();
        }
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        action = Some(MenuAction::Back);
    }
    match action {
        Some(MenuAction::Local) => {
            commands.remove_resource::<Net>();
            *mode = PlayMode::Local;
            game.paused = false;
            menu.status.clear();
        }
        Some(MenuAction::Host) => {
            let code = RoomCode::random(crate::game::fresh_seed());
            menu.join_code = code.as_str();
            menu.status = format!("Share this code. Connecting to {}...", menu.relay);
            connect(&mut commands, Hello::Host(code), menu.relay.clone(), code);
            *mode = PlayMode::Menu;
            game.paused = true;
        }
        Some(MenuAction::Join) => {
            let Some(code) = RoomCode::parse(&menu.join_code) else {
                menu.status = "Enter a 4-character room code.".into();
                return;
            };
            menu.status = format!("Joining {}...", code.as_str());
            connect(&mut commands, Hello::Guest(code), menu.relay.clone(), code);
            *mode = PlayMode::Menu;
            game.paused = true;
        }
        Some(MenuAction::Back) => {
            commands.remove_resource::<Net>();
            menu.status.clear();
            menu.join_code.clear();
            *mode = PlayMode::Menu;
            game.paused = true;
        }
        Some(MenuAction::QuitToMenu) | None => {}
    }
    if net.is_some() {
        return;
    }
    for key in keys.get_just_pressed() {
        if *key == KeyCode::Backspace {
            menu.join_code.pop();
        } else if let Some(ch) = key_char(*key)
            && menu.join_code.len() < 4
        {
            menu.join_code.push(ch);
        }
    }
}

fn key_char(key: KeyCode) -> Option<char> {
    Some(match key {
        KeyCode::Digit2 => '2',
        KeyCode::Digit3 => '3',
        KeyCode::Digit4 => '4',
        KeyCode::Digit5 => '5',
        KeyCode::Digit6 => '6',
        KeyCode::Digit7 => '7',
        KeyCode::Digit8 => '8',
        KeyCode::Digit9 => '9',
        KeyCode::KeyA => 'A',
        KeyCode::KeyB => 'B',
        KeyCode::KeyC => 'C',
        KeyCode::KeyD => 'D',
        KeyCode::KeyE => 'E',
        KeyCode::KeyF => 'F',
        KeyCode::KeyG => 'G',
        KeyCode::KeyH => 'H',
        KeyCode::KeyJ => 'J',
        KeyCode::KeyK => 'K',
        KeyCode::KeyL => 'L',
        KeyCode::KeyM => 'M',
        KeyCode::KeyN => 'N',
        KeyCode::KeyP => 'P',
        KeyCode::KeyQ => 'Q',
        KeyCode::KeyR => 'R',
        KeyCode::KeyS => 'S',
        KeyCode::KeyT => 'T',
        KeyCode::KeyU => 'U',
        KeyCode::KeyV => 'V',
        KeyCode::KeyW => 'W',
        KeyCode::KeyX => 'X',
        KeyCode::KeyY => 'Y',
        KeyCode::KeyZ => 'Z',
        _ => return None,
    })
}
