use bevy::prelude::*;

use crate::{
    game::{Action, Game, Phase},
    net::{Menu, MenuAction, Net, PlayMode},
    terrain::MapSize,
    visuals::{BLUE, GOLD, RED},
};

const INK: Color = Color::srgb(0.045, 0.075, 0.10);
const PAPER: Color = Color::srgb(0.91, 0.94, 0.90);
const MUTED: Color = Color::srgb(0.62, 0.73, 0.75);

#[derive(Component)]
pub enum Label {
    Status,
    Aim,
    Wind,
    Round,
    Health(usize),
    Message,
    Primary,
    Pause,
    Map,
    NextMap,
    MenuStatus,
    JoinCode,
}
#[derive(Component)]
pub struct HealthFill(usize);
#[derive(Component)]
pub struct MenuRoot;

fn text(value: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}

pub fn setup(mut commands: Commands) {
    commands.spawn(Node {
        width: percent(100), height: percent(100), flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::SpaceBetween, padding: UiRect::all(px(24)), ..default()
    }).with_children(|screen| {
        screen.spawn(Node { width: percent(100), justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Start, column_gap: px(18), ..default() }).with_children(|top| {
            top.spawn((Node { flex_direction: FlexDirection::Column, padding: UiRect::all(px(20)), row_gap: px(6), width: px(290), ..default() }, BackgroundColor(INK.with_alpha(0.93)), BorderRadius::all(px(14)))).with_children(|brand| {
                brand.spawn(text("3D CANON", 34.0, PAPER));
                brand.spawn(text("W I N D  &  W A R F A R E", 12.0, GOLD));
                brand.spawn((text("", 14.0, MUTED), Label::Round));
                brand.spawn((text("", 15.0, PAPER), Label::Wind));
                brand.spawn((text("", 14.0, GOLD), Label::Map));
                brand.spawn(text("NEXT MAP SIZE", 12.0, MUTED));
                brand.spawn(Node { column_gap: px(5), ..default() }).with_children(|sizes| {
                    for size in MapSize::ALL {
                        sizes.spawn((Button, Action::SelectSize(size), Node { padding: UiRect::axes(px(9), px(9)), ..default() }, BackgroundColor(INK), BorderRadius::all(px(6)))).with_children(|button| {
                            button.spawn(text(size.label(), 13.0, PAPER));
                        });
                    }
                });
                brand.spawn((text("", 12.0, MUTED), Label::NextMap));
            });
            top.spawn(Node { column_gap: px(12), ..default() }).with_children(|players| {
                for i in 0..2 {
                    let color = if i == 0 { RED } else { BLUE };
                    players.spawn((Node { width: px(180), padding: UiRect::all(px(16)), flex_direction: FlexDirection::Column, row_gap: px(10), ..default() }, BackgroundColor(INK.with_alpha(0.93)), BorderRadius::all(px(14)))).with_children(|card| {
                        card.spawn(text(if i == 0 { "01 / RED" } else { "02 / BLUE" }, 16.0, color));
                        card.spawn((text("100 HP", 25.0, PAPER), Label::Health(i)));
                        card.spawn((Node { width: percent(100), height: px(5), ..default() }, BackgroundColor(Color::srgb(0.17, 0.22, 0.25)))).with_children(|track| {
                            track.spawn((HealthFill(i), Node { width: percent(100), height: percent(100), ..default() }, BackgroundColor(color)));
                        });
                    });
                }
            });
        });
        screen.spawn((Node { align_self: AlignSelf::Center, padding: UiRect::all(px(22)), ..default() }, BackgroundColor(INK.with_alpha(0.94)), BorderRadius::all(px(14)), Visibility::Hidden, Label::Pause))
            .with_children(|panel| { panel.spawn(text("PAUSED\nEsc to resume", 28.0, PAPER)); });
        screen.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(10), ..default() }).with_children(|bottom| {
            bottom.spawn((Node { align_self: AlignSelf::Center, padding: UiRect::axes(px(20), px(10)), ..default() }, BackgroundColor(INK.with_alpha(0.87)), BorderRadius::all(px(10)))).with_children(|banner| {
                banner.spawn((text("", 16.0, PAPER), Label::Message));
            });
            bottom.spawn((Node { padding: UiRect::all(px(20)), flex_direction: FlexDirection::Column, row_gap: px(12), ..default() }, BackgroundColor(INK.with_alpha(0.95)), BorderRadius::all(px(14)))).with_children(|panel| {
                panel.spawn(Node { justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, column_gap: px(20), ..default() }).with_children(|row| {
                    row.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(8), ..default() }).with_children(|stats| {
                        stats.spawn((text("", 26.0, GOLD), Label::Status));
                        stats.spawn((text("", 18.0, PAPER), Label::Aim));
                    });
                    row.spawn(Node { column_gap: px(10), ..default() }).with_children(|buttons| {
                        buttons.spawn((Button, Action::Primary, Node { padding: UiRect::axes(px(24), px(16)), ..default() }, BackgroundColor(GOLD), BorderRadius::all(px(8)))).with_children(|button| {
                            button.spawn((text("READY / ENTER", 16.0, INK), Label::Primary));
                        });
                        buttons.spawn((Button, Action::Restart, Node { padding: UiRect::axes(px(18), px(16)), ..default() }, BackgroundColor(Color::srgb(0.18, 0.25, 0.28)), BorderRadius::all(px(8)))).with_children(|button| {
                            button.spawn(text("NEW MAP / R", 16.0, PAPER));
                        });
                    });
                });
                panel.spawn(text("A / D  Aim     W / S  Elevation     Q / E  Power     Shift  Fine tune     Space  Fire", 14.0, MUTED));
                panel.spawn(text("Right-drag  Orbit     Scroll  Zoom     C  Overview     Esc  Pause     M  Sound     Shift + R  Replay map", 13.0, MUTED));
            });
        });
    });
    commands
        .spawn((
            MenuRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(16),
                ..default()
            },
            BackgroundColor(INK.with_alpha(0.94)),
            GlobalZIndex(10),
        ))
        .with_children(|menu| {
            menu.spawn(text("3D CANON", 48.0, PAPER));
            menu.spawn(text("LOCAL OR ONLINE 1V1", 14.0, GOLD));
            menu.spawn((text("", 16.0, GOLD), Label::MenuStatus));
            menu.spawn(Node {
                column_gap: px(12),
                ..default()
            })
            .with_children(|row| {
                for (action, label) in [
                    (MenuAction::Local, "LOCAL"),
                    (MenuAction::Host, "HOST"),
                    (MenuAction::Join, "JOIN"),
                ] {
                    row.spawn((
                        Button,
                        action,
                        Node {
                            padding: UiRect::axes(px(22), px(14)),
                            ..default()
                        },
                        BackgroundColor(if action == MenuAction::Local {
                            GOLD
                        } else {
                            Color::srgb(0.18, 0.25, 0.28)
                        }),
                        BorderRadius::all(px(8)),
                    ))
                    .with_children(|button| {
                        button.spawn(text(
                            label,
                            16.0,
                            if action == MenuAction::Local {
                                INK
                            } else {
                                PAPER
                            },
                        ));
                    });
                }
            });
            menu.spawn((text("JOIN CODE: ____", 18.0, PAPER), Label::JoinCode));
            menu.spawn(text(
                "Type a 4-character code, then Join. Host shares the code after connecting.",
                14.0,
                MUTED,
            ));
            menu.spawn(text(
                "Online uses 192.241.147.149:3478   --relay=host:port to override",
                13.0,
                MUTED,
            ));
        });
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    game: Res<Game>,
    mode: Option<Res<PlayMode>>,
    menu: Option<Res<Menu>>,
    net: Option<Res<Net>>,
    mut labels: Query<(&Label, &mut Text, &mut TextColor)>,
    mut health: Query<(&HealthFill, &mut Node)>,
    mut pause: Query<(&Label, &mut Visibility), Without<MenuRoot>>,
    mut menu_root: Query<&mut Visibility, With<MenuRoot>>,
    mut buttons: Query<(&Action, &Interaction, &mut BackgroundColor)>,
) {
    let online = mode.as_deref() == Some(&PlayMode::Online);
    let waiting = net.as_ref().is_some_and(|net| net.waiting);
    let show_menu = mode.as_deref() == Some(&PlayMode::Menu) || waiting;
    for mut visibility in &mut menu_root {
        *visibility = if show_menu {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let mine = net.as_ref().is_some_and(|net| net.my_turn(&game));
    let cannon = game.cannons[game.active];
    let player = if game.active == 0 { "RED" } else { "BLUE" };
    let bearing = game
        .wind
        .x
        .atan2(-game.wind.z)
        .to_degrees()
        .rem_euclid(360.0);
    let compass =
        ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][((bearing + 22.5) / 45.0) as usize % 8];
    for (label, mut text, mut color) in &mut labels {
        let value = match label {
            Label::Status => {
                color.0 = if game.active == 0 { RED } else { BLUE };
                match game.phase {
                    Phase::Handoff if online && !mine => format!("WAITING FOR {player}"),
                    Phase::Handoff => format!("{player}, YOUR TURN"),
                    Phase::Aiming if online && !mine => format!("{player} IS AIMING"),
                    Phase::Aiming => format!("{player} / LINE UP YOUR SHOT"),
                    Phase::Flying => "CANNONBALL IN FLIGHT".into(),
                    Phase::Resolving(_) => "IMPACT / SETTLING".into(),
                    Phase::Finished(Some(i)) => {
                        color.0 = GOLD;
                        format!("{} WINS!", if i == 0 { "RED" } else { "BLUE" })
                    }
                    Phase::Finished(None) => {
                        color.0 = GOLD;
                        "MUTUAL DESTRUCTION / DRAW".into()
                    }
                }
            }
            Label::Aim => format!(
                "Bearing {:05.1} deg  /  Elevation {:04.1} deg  /  Power {:04.1} m/s",
                (cannon.yaw.to_degrees() + 90.0).rem_euclid(360.0),
                cannon.elevation.to_degrees(),
                cannon.power
            ),
            Label::Wind => format!(
                "WIND  {:.1} m/s toward {compass}\nShared for both turns this round",
                game.wind.length()
            ),
            Label::Round => format!(
                "ROUND {:02}  /  SOUND {}\nSEED {}",
                game.round,
                if game.muted { "OFF" } else { "ON" },
                game.seed
            ),
            Label::Health(i) => format!("{:.0} HP", game.cannons[*i].health.ceil()),
            Label::Map => format!(
                "{} / {:.0} x {:.0} m",
                game.size.label(),
                game.size.half() * 2.0,
                game.size.half() * 2.0
            ),
            Label::NextMap => format!("{} selected. Apply: New Map / R", game.next_size.label()),
            Label::Message => {
                if online {
                    if let Some(net) = net.as_deref() {
                        format!(
                            "{}  You are {}. Room {}.",
                            game.message,
                            if net.seat == 0 { "Red / host" } else { "Blue" },
                            net.code.as_str()
                        )
                    } else {
                        game.message.clone()
                    }
                } else if game.paused {
                    "Simulation paused. Press Esc to resume.".into()
                } else if game.phase == Phase::Handoff {
                    format!(
                        "{}  Pass to {player} and press Enter when ready.",
                        game.message
                    )
                } else {
                    game.message.clone()
                }
            }
            Label::MenuStatus => menu
                .as_ref()
                .map(|menu| {
                    if menu.status.is_empty() {
                        "Hot-seat on this computer, or host/join over canon-relay.".into()
                    } else {
                        menu.status.clone()
                    }
                })
                .unwrap_or_default(),
            Label::JoinCode => format!(
                "JOIN CODE: {:<4}",
                menu.as_ref()
                    .map(|menu| menu.join_code.as_str())
                    .unwrap_or("")
            ),
            Label::Primary => {
                if game.paused {
                    "PAUSED / ESC".into()
                } else {
                    match game.phase {
                        Phase::Handoff => "READY / ENTER",
                        Phase::Aiming => "FIRE / SPACE",
                        Phase::Flying | Phase::Resolving(_) => "SHOT IN PROGRESS",
                        Phase::Finished(_) => "MATCH COMPLETE",
                    }
                    .into()
                }
            }
            Label::Pause => continue,
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    for (fill, mut node) in &mut health {
        node.width = percent(game.cannons[fill.0].health);
    }
    for (label, mut visibility) in &mut pause {
        if matches!(label, Label::Pause) {
            *visibility = if game.paused {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
    for (action, interaction, mut background) in &mut buttons {
        background.0 = match action {
            Action::Primary
                if game.paused || !matches!(game.phase, Phase::Handoff | Phase::Aiming) =>
            {
                Color::srgb(0.35, 0.40, 0.40)
            }
            Action::Primary if *interaction == Interaction::Hovered => Color::srgb(1.0, 0.9, 0.6),
            Action::Primary => GOLD,
            Action::Restart if *interaction == Interaction::Hovered => {
                Color::srgb(0.28, 0.36, 0.39)
            }
            Action::Restart => Color::srgb(0.18, 0.25, 0.28),
            Action::SelectSize(size) if *size == game.next_size => Color::srgb(0.46, 0.32, 0.08),
            Action::SelectSize(_) if *interaction == Interaction::Hovered => {
                Color::srgb(0.28, 0.36, 0.39)
            }
            Action::SelectSize(_) => Color::srgb(0.18, 0.25, 0.28),
        };
    }
}
