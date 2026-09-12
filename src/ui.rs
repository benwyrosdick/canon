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
    Bearing,
    Elevation,
    Power,
    Wind,
    Round,
    Health(usize),
    Message,
    Primary,
    Pause,
    Map,
    NextMap,
    MenuStatus,
}
#[derive(Component)]
pub struct HealthFill(usize);
#[derive(Component)]
pub struct MenuRoot;
#[derive(Component)]
pub struct HostOnly;
#[derive(Component)]
pub struct CodeSlot(usize);
#[derive(Component)]
pub struct CodeGlyph(usize);
#[derive(Component)]
pub enum AimWidget {
    Vertical,
    Horizontal,
    Pip,
}
#[derive(Component)]
pub struct PowerFill;

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
                brand.spawn((text("NEXT MAP SIZE", 12.0, MUTED), HostOnly));
                brand.spawn((HostOnly, Node { column_gap: px(5), ..default() })).with_children(|sizes| {
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
        screen.spawn((Node { align_self: AlignSelf::Center, padding: UiRect::all(px(28)), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(14), border: UiRect::all(px(2)), ..default() }, BackgroundColor(INK), BorderColor::all(GOLD), BorderRadius::all(px(16)), Visibility::Hidden, Label::Pause))
            .with_children(|panel| {
                panel.spawn(text("PAUSED", 28.0, PAPER));
                panel.spawn(text("Esc to resume", 14.0, MUTED));
                panel.spawn((Button, MenuAction::QuitToMenu, Node { padding: UiRect::axes(px(22), px(12)), ..default() }, BackgroundColor(Color::srgb(0.18, 0.25, 0.28)), BorderRadius::all(px(8)))).with_children(|button| {
                    button.spawn(text("MAIN MENU / TAB", 16.0, PAPER));
                });
            });
        screen.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(10), ..default() }).with_children(|bottom| {
            bottom.spawn((Node { align_self: AlignSelf::Center, padding: UiRect::axes(px(20), px(10)), ..default() }, BackgroundColor(INK.with_alpha(0.87)), BorderRadius::all(px(10)))).with_children(|banner| {
                banner.spawn((text("", 16.0, PAPER), Label::Message));
            });
            bottom.spawn((Node { padding: UiRect::all(px(20)), flex_direction: FlexDirection::Column, row_gap: px(12), ..default() }, BackgroundColor(INK.with_alpha(0.95)), BorderRadius::all(px(14)))).with_children(|panel| {
                panel.spawn(Node { justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, column_gap: px(20), ..default() }).with_children(|row| {
                    row.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(8), flex_grow: 1.0, ..default() }).with_children(|stats| {
                        stats.spawn((text("", 26.0, GOLD), Label::Status));
                    });
                    row.spawn(Node { column_gap: px(14), align_items: AlignItems::Center, ..default() }).with_children(|gauges| {
                        gauges.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(4), ..default() }).with_children(|scope_wrap| {
                            scope_wrap.spawn((text("", 13.0, GOLD), Label::Elevation));
                            scope_wrap.spawn(Node {
                                width: px(168),
                                height: px(112),
                                position_type: PositionType::Relative,
                                border: UiRect::all(px(1)),
                                overflow: Overflow::visible(),
                                ..default()
                            }).with_children(|scope| {
                                scope.spawn((
                                    Node { width: percent(100), height: percent(100), ..default() },
                                    BackgroundColor(Color::srgb(0.07, 0.10, 0.11)),
                                ));
                                scope.spawn((
                                    AimWidget::Horizontal,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: px(0),
                                        width: percent(100),
                                        height: px(2),
                                        bottom: percent(50),
                                        ..default()
                                    },
                                    BackgroundColor(GOLD.with_alpha(0.85)),
                                ));
                                scope.spawn((
                                    AimWidget::Vertical,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: px(0),
                                        height: percent(100),
                                        width: px(2),
                                        left: percent(50),
                                        ..default()
                                    },
                                    BackgroundColor(GOLD.with_alpha(0.85)),
                                ));
                                scope.spawn((
                                    AimWidget::Pip,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: px(8),
                                        height: px(8),
                                        left: percent(50),
                                        bottom: percent(50),
                                        margin: UiRect::axes(px(-3), px(-3)),
                                        border: UiRect::all(px(1)),
                                        ..default()
                                    },
                                    BackgroundColor(PAPER),
                                    BorderColor::all(GOLD),
                                    BorderRadius::all(px(8)),
                                ));
                            });
                            scope_wrap.spawn((text("", 13.0, GOLD), Label::Bearing));
                        });
                        gauges.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(6), ..default() }).with_children(|power| {
                            power.spawn(text("PWR", 11.0, MUTED));
                            power.spawn((Node { width: px(16), height: px(112), padding: UiRect::all(px(2)), flex_direction: FlexDirection::Column, justify_content: JustifyContent::End, ..default() }, BackgroundColor(Color::srgb(0.17, 0.22, 0.25)), BorderRadius::all(px(4)))).with_children(|track| {
                                track.spawn((PowerFill, Node { width: percent(100), height: percent(50), ..default() }, BackgroundColor(GOLD), BorderRadius::all(px(3))));
                            });
                            power.spawn((text("", 12.0, PAPER), Label::Power));
                        });
                    });
                    row.spawn(Node { column_gap: px(10), ..default() }).with_children(|buttons| {
                        buttons.spawn((Button, Action::Primary, Node { padding: UiRect::axes(px(24), px(16)), ..default() }, BackgroundColor(GOLD), BorderRadius::all(px(8)))).with_children(|button| {
                            button.spawn((text("READY / ENTER", 16.0, INK), Label::Primary));
                        });
                        buttons.spawn((Button, Action::Restart, HostOnly, Node { padding: UiRect::axes(px(18), px(16)), ..default() }, BackgroundColor(Color::srgb(0.18, 0.25, 0.28)), BorderRadius::all(px(8)))).with_children(|button| {
                            button.spawn(text("NEW MAP / R", 16.0, PAPER));
                        });
                    });
                });
                panel.spawn(text("A / D  Aim     W / S  Elevation     Q / E  Power     Shift  Fine tune     Space  Fire", 14.0, MUTED));
                panel.spawn(text("Right-drag  Orbit     Scroll  Zoom     C  Overview     Esc  Pause     Tab  Menu     M  Sound     Shift + R  Replay map", 13.0, MUTED));
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
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.05, 0.78)),
            GlobalZIndex(10),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(560),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(px(36)),
                        row_gap: px(16),
                        border: UiRect::all(px(2)),
                        ..default()
                    },
                    BackgroundColor(INK),
                    BorderColor::all(GOLD),
                    BorderRadius::all(px(18)),
                ))
                .with_children(|menu| {
                    menu.spawn(text("3D CANON", 48.0, PAPER));
                    menu.spawn(text("LOCAL OR ONLINE 1V1", 14.0, GOLD));
                    menu.spawn((text("", 16.0, GOLD), Label::MenuStatus));
                    menu.spawn(Node {
                        column_gap: px(12),
                        margin: UiRect::vertical(px(8)),
                        ..default()
                    })
                    .with_children(|row| {
                        for i in 0..4 {
                            row.spawn((
                                CodeSlot(i),
                                Node {
                                    width: px(72),
                                    height: px(88),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(px(2)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.08, 0.12, 0.14)),
                                BorderColor::all(Color::srgb(0.35, 0.42, 0.40)),
                                BorderRadius::all(px(10)),
                            ))
                            .with_children(|cell| {
                                cell.spawn((CodeGlyph(i), text("", 42.0, GOLD)));
                            });
                        }
                    });
                    menu.spawn(text(
                        "Type a room code, then Join. Hosting fills these boxes for the other player.",
                        14.0,
                        MUTED,
                    ));
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
                    menu.spawn(text(
                        "Online uses 192.241.147.149:3478   --relay=host:port to override",
                        13.0,
                        MUTED,
                    ));
                });
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update(
    game: Res<Game>,
    mode: Option<Res<PlayMode>>,
    menu: Option<Res<Menu>>,
    net: Option<Res<Net>>,
    mut labels: Query<(&Label, &mut Text, &mut TextColor)>,
    mut health: Query<(&HealthFill, &mut Node), (Without<PowerFill>, Without<AimWidget>)>,
    mut power_fill: Query<&mut Node, (With<PowerFill>, Without<HealthFill>, Without<AimWidget>)>,
    mut aim: Query<(&AimWidget, &mut Node), (Without<HealthFill>, Without<PowerFill>)>,
    mut pause: Query<(&Label, &mut Visibility), (Without<MenuRoot>, Without<HostOnly>)>,
    mut menu_root: Query<&mut Visibility, (With<MenuRoot>, Without<HostOnly>)>,
    mut host_only: Query<&mut Visibility, (With<HostOnly>, Without<MenuRoot>, Without<Label>)>,
    mut buttons: Query<(&Action, &Interaction, &mut BackgroundColor), Without<CodeSlot>>,
    mut glyphs: Query<(&CodeGlyph, &mut Text), Without<Label>>,
    mut slots: Query<(&CodeSlot, &mut BackgroundColor, &mut BorderColor), Without<Action>>,
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
    let guest = online && net.as_ref().is_some_and(|net| net.seat != 0);
    for mut visibility in &mut host_only {
        *visibility = if guest {
            Visibility::Hidden
        } else {
            Visibility::Visible
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
            Label::Bearing => format!(
                "BRG  {:05.1}",
                (cannon.yaw.to_degrees() + 90.0).rem_euclid(360.0)
            ),
            Label::Elevation => format!("ELV  {:04.1}", cannon.elevation.to_degrees()),
            Label::Power => format!("{:.0} m/s", cannon.power),
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
            Label::NextMap => {
                if guest {
                    format!(
                        "Host selected {} / {:.0} x {:.0} m",
                        game.size.label(),
                        game.size.half() * 2.0,
                        game.size.half() * 2.0
                    )
                } else {
                    format!("{} selected. Apply: New Map / R", game.next_size.label())
                }
            }
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
                    "Paused. Esc resumes. Tab or Main Menu returns to the title screen.".into()
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
    let bearing_pct =
        ((cannon.yaw.to_degrees() + 90.0).rem_euclid(360.0) / 360.0 * 100.0).clamp(0.0, 100.0);
    let elevation_pct = ((cannon.elevation.to_degrees() - 5.0) / 80.0 * 100.0).clamp(0.0, 100.0);
    let power_pct = ((cannon.power - 15.0) / 37.0 * 100.0).clamp(0.0, 100.0);
    for (widget, mut node) in &mut aim {
        match widget {
            AimWidget::Vertical | AimWidget::Pip => node.left = percent(bearing_pct),
            AimWidget::Horizontal => {}
        }
        match widget {
            AimWidget::Horizontal | AimWidget::Pip => node.bottom = percent(elevation_pct),
            AimWidget::Vertical => {}
        }
    }
    for mut node in &mut power_fill {
        node.height = percent(power_pct);
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
    let displayed = net
        .as_ref()
        .map(|net| net.code.as_str())
        .or_else(|| menu.as_ref().map(|menu| menu.join_code.clone()))
        .unwrap_or_default();
    let chars: Vec<char> = displayed.chars().collect();
    let cursor = chars.len().min(4);
    for (glyph, mut text) in &mut glyphs {
        let value = chars
            .get(glyph.0)
            .map(|ch| ch.to_string())
            .unwrap_or_default();
        if text.0 != value {
            text.0 = value;
        }
    }
    for (slot, mut background, mut border) in &mut slots {
        let filled = slot.0 < chars.len();
        let active = slot.0 == cursor && cursor < 4 && net.is_none();
        background.0 = if filled {
            Color::srgb(0.18, 0.14, 0.06)
        } else {
            Color::srgb(0.08, 0.12, 0.14)
        };
        *border = BorderColor::all(if filled || active {
            GOLD
        } else {
            Color::srgb(0.35, 0.42, 0.40)
        });
    }
}
