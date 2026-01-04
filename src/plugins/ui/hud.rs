//! HUD setup and update systems.

use bevy::prelude::*;
use bevy::ui::Node as UiNode;
use std::path::Path;

use crate::compat::{NodeBundle, TextBundle, TextStyle};
use crate::plugins::core::ViewMode;
use crate::plugins::player::PlayerControl;
use crate::plugins::render2d::{FocusMarker, IntelRefreshCooldown};
use crate::plugins::sim::SimTickCount;
use crate::ships::Cargo;
use crate::ships::Ship;
use crate::world::ZoneId;

use super::components::{
    ContactsListContainer, CooldownText, FleetDetailDivider, FleetDetailText, FleetListContainer,
    FleetPanelMarker, FocusText, HudText, IntelContentText, IntelPanelText, LogContentText,
    LogPanelMarker, MapUi, PlayerPanelText, TacticalPanelText, WorldUi,
};

// =============================================================================
// Setup Systems
// =============================================================================

pub fn setup_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        info!("HUD font not found at {}", font_on_disk.display());
        return;
    }

    let font = asset_server.load(font_path);

    // Main HUD text (top-left)
    commands.spawn((
        HudText,
        TextBundle::from_section(
            "Seed: -- | Tick: --",
            TextStyle {
                font: font.clone(),
                font_size: 18.0,
                color: Color::srgb(0.9, 0.9, 0.95),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(10.0),
            ..default()
        }),
    ));

    // Player panel text (world view)
    commands.spawn((
        PlayerPanelText,
        WorldUi,
        TextBundle::from_section(
            "Player: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::srgb(0.82, 0.88, 0.95),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(36.0),
            ..default()
        }),
    ));

    // Map view help texts
    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "Icons: R Rad | N Neb | O Ore | C Clear | . None",
            TextStyle {
                font: font.clone(),
                font_size: 12.0,
                color: Color::srgb(0.6, 0.65, 0.72),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(118.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "Map: G grid | R routes | T route labels | Y node labels | V reveal adj | A reveal all",
            TextStyle {
                font: font.clone(),
                font_size: 12.0,
                color: Color::srgb(0.6, 0.65, 0.72),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(136.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "Route label: distance + risk",
            TextStyle {
                font: font.clone(),
                font_size: 12.0,
                color: Color::srgb(0.6, 0.65, 0.72),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(154.0),
            ..default()
        }),
    ));

    // Log panel (bottom-left)
    spawn_log_panel(&mut commands, &font);

    // Cooldown text
    commands.spawn((
        CooldownText,
        TextBundle::from_section(
            "Intel refresh: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::srgb(0.7, 0.75, 0.82),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(300.0),
            ..default()
        }),
    ));

    // Focus text (world view)
    commands.spawn((
        FocusText,
        WorldUi,
        TextBundle::from_section(
            "Focus: --",
            TextStyle {
                font: font.clone(),
                font_size: 13.0,
                color: Color::srgb(0.7, 0.8, 0.9),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(460.0),
            ..default()
        }),
    ));

    // Fleet panel (top-right, world view)
    spawn_fleet_panel(&mut commands, &font);

    // Intel + Contacts wrapper (bottom-right, world view)
    spawn_intel_contacts_panels(&mut commands, &font);

    // Map compass markers
    spawn_compass_markers(&mut commands, &font);
}

fn spawn_log_panel(commands: &mut Commands, font: &Handle<Font>) {
    commands
        .spawn((
            LogPanelMarker,
            NodeBundle {
                node: UiNode {
                    position_type: PositionType::Absolute,
                    left: Val::Px(14.0),
                    bottom: Val::Px(14.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    min_width: Val::Px(280.0),
                    max_height: Val::Px(160.0),
                    overflow: Overflow {
                        y: OverflowAxis::Scroll,
                        ..default()
                    },
                    ..default()
                },
                background_color: Color::srgba(0.02, 0.05, 0.08, 0.85).into(),
                border_color: Color::srgb(0.6, 0.4, 0.8).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "Subspace Transmissions",
                TextStyle {
                    font: font.clone(),
                    font_size: 13.0,
                    color: Color::srgb(0.8, 0.6, 1.0),
                },
            ));

            // Divider
            parent.spawn(NodeBundle {
                node: UiNode {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(4.0)),
                    ..default()
                },
                background_color: Color::srgb(0.4, 0.25, 0.5).into(),
                ..default()
            });

            // Content
            parent.spawn((
                LogContentText,
                TextBundle::from_section(
                    "Awaiting signal...",
                    TextStyle {
                        font: font.clone(),
                        font_size: 12.0,
                        color: Color::srgb(0.7, 0.75, 0.82),
                    },
                ),
            ));
        });
}

fn spawn_fleet_panel(commands: &mut Commands, font: &Handle<Font>) {
    commands
        .spawn((
            FleetPanelMarker,
            WorldUi,
            Interaction::None,
            NodeBundle {
                node: UiNode {
                    position_type: PositionType::Absolute,
                    right: Val::Px(14.0),
                    top: Val::Px(14.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    min_width: Val::Px(200.0),
                    max_height: Val::Px(180.0),
                    overflow: Overflow {
                        y: OverflowAxis::Scroll,
                        ..default()
                    },
                    ..default()
                },
                background_color: Color::srgba(0.02, 0.05, 0.08, 0.85).into(),
                border_color: Color::srgb(0.0, 0.8, 0.8).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "Fleet",
                TextStyle {
                    font: font.clone(),
                    font_size: 13.0,
                    color: Color::srgb(0.0, 1.0, 1.0),
                },
            ));

            // Divider
            parent.spawn(NodeBundle {
                node: UiNode {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(4.0)),
                    ..default()
                },
                background_color: Color::srgb(0.0, 0.5, 0.5).into(),
                ..default()
            });

            // List container
            parent.spawn((
                FleetListContainer,
                NodeBundle {
                    node: UiNode {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                },
            ));

            // Detail divider (hidden by default)
            parent.spawn((
                FleetDetailDivider,
                NodeBundle {
                    node: UiNode {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(6.0)),
                        ..default()
                    },
                    background_color: Color::srgb(0.0, 0.4, 0.4).into(),
                    visibility: Visibility::Hidden,
                    ..default()
                },
            ));

            // Detail section
            parent.spawn((
                FleetDetailText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: font.clone(),
                        font_size: 11.0,
                        color: Color::srgb(0.5, 0.7, 0.7),
                    },
                ),
            ));
        });
}

fn spawn_intel_contacts_panels(commands: &mut Commands, font: &Handle<Font>) {
    commands
        .spawn((
            WorldUi,
            NodeBundle {
                node: UiNode {
                    position_type: PositionType::Absolute,
                    right: Val::Px(14.0),
                    bottom: Val::Px(14.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            // Intel panel
            parent
                .spawn((
                    IntelPanelText,
                    NodeBundle {
                        node: UiNode {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(8.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            min_width: Val::Px(140.0),
                            max_height: Val::Px(120.0),
                            overflow: Overflow {
                                y: OverflowAxis::Scroll,
                                ..default()
                            },
                            ..default()
                        },
                        background_color: Color::srgba(0.02, 0.05, 0.08, 0.85).into(),
                        border_color: Color::srgb(0.0, 0.8, 0.8).into(),
                        ..default()
                    },
                ))
                .with_children(|intel| {
                    intel.spawn(TextBundle::from_section(
                        "Intel",
                        TextStyle {
                            font: font.clone(),
                            font_size: 13.0,
                            color: Color::srgb(0.0, 1.0, 1.0),
                        },
                    ));

                    intel.spawn(NodeBundle {
                        node: UiNode {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.0, 0.5, 0.5).into(),
                        ..default()
                    });

                    intel.spawn((
                        IntelContentText,
                        TextBundle::from_section(
                            "No target selected",
                            TextStyle {
                                font: font.clone(),
                                font_size: 13.0,
                                color: Color::srgb(0.6, 0.8, 0.8),
                            },
                        ),
                    ));
                });

            // Contacts panel
            parent
                .spawn((
                    TacticalPanelText,
                    Interaction::default(),
                    ScrollPosition::default(),
                    NodeBundle {
                        node: UiNode {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(8.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            min_width: Val::Px(140.0),
                            max_height: Val::Px(150.0),
                            overflow: Overflow {
                                y: OverflowAxis::Scroll,
                                ..default()
                            },
                            ..default()
                        },
                        background_color: Color::srgba(0.02, 0.05, 0.08, 0.85).into(),
                        border_color: Color::srgb(0.0, 0.8, 0.8).into(),
                        ..default()
                    },
                ))
                .with_children(|contacts| {
                    contacts.spawn(TextBundle::from_section(
                        "Contacts",
                        TextStyle {
                            font: font.clone(),
                            font_size: 13.0,
                            color: Color::srgb(0.0, 1.0, 1.0),
                        },
                    ));

                    contacts.spawn(NodeBundle {
                        node: UiNode {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.0, 0.5, 0.5).into(),
                        ..default()
                    });

                    contacts.spawn((
                        ContactsListContainer,
                        NodeBundle {
                            node: UiNode {
                                flex_direction: FlexDirection::Column,
                                ..default()
                            },
                            ..default()
                        },
                    ));
                });
        });
}

fn spawn_compass_markers(commands: &mut Commands, font: &Handle<Font>) {
    // N
    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "N",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::srgb(0.65, 0.7, 0.78),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Px(8.0),
            ..default()
        }),
    ));

    // S
    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "S",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::srgb(0.65, 0.7, 0.78),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            bottom: Val::Px(8.0),
            ..default()
        }),
    ));

    // W
    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "W",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::srgb(0.65, 0.7, 0.78),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            top: Val::Percent(50.0),
            ..default()
        }),
    ));

    // E
    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "E",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::srgb(0.65, 0.7, 0.78),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            top: Val::Percent(50.0),
            ..default()
        }),
    ));
}

// =============================================================================
// Update Systems
// =============================================================================

pub fn update_hud(
    view: Res<ViewMode>,
    ticks: Res<SimTickCount>,
    mut hud_text: Query<&mut Text, With<HudText>>,
) {
    if let Some(mut text) = hud_text.iter_mut().next() {
        text.0 = format!("View: {:?} | t{} | F3: Debug", *view, ticks.tick);
    }
}

pub fn update_cooldown_panel(
    ticks: Res<SimTickCount>,
    cooldown: Res<IntelRefreshCooldown>,
    mut panel: Query<&mut Text, With<CooldownText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        let remaining = cooldown.remaining_ticks(ticks.tick);
        if remaining == 0 {
            text.0 = "Intel refresh: ready".to_string();
        } else {
            text.0 = format!("Intel refresh: {}t", remaining);
        }
    }
}

pub fn update_player_panel(
    player: Query<(&Ship, &Cargo, &ZoneId), With<PlayerControl>>,
    mut panel: Query<&mut Text, With<PlayerPanelText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        match player.single() {
            Ok((ship, cargo, zone_id)) => {
                let fuel_pct = if ship.fuel_capacity > 0.0 {
                    (ship.fuel / ship.fuel_capacity) * 100.0
                } else {
                    0.0
                };
                let ore_pct = if cargo.capacity > 0.0 {
                    (cargo.common_ore / cargo.capacity) * 100.0
                } else {
                    0.0
                };
                text.0 = format!(
                    "Player: Zone {} | Fuel {:.0}% | Ore {:.0}% ({:.0}/{:.0})",
                    zone_id.0, fuel_pct, ore_pct, cargo.common_ore, cargo.capacity
                );
            }
            Err(_) => {
                text.0 = "Player: --".to_string();
            }
        }
    }
}

pub fn update_focus_panel(marker: Res<FocusMarker>, mut panel: Query<&mut Text, With<FocusText>>) {
    if let Some(mut text) = panel.iter_mut().next() {
        match marker.node_id() {
            Some(node_id) => {
                text.0 = format!("Focus: node {}", node_id);
            }
            None => {
                text.0 = "Focus: --".to_string();
            }
        };
    }
}
