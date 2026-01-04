//! HUD setup and update systems.

use bevy::prelude::*;
use bevy::ui::Node as UiNode;
use std::path::Path;

use crate::compat::{NodeBundle, TextBundle, TextStyle};
use crate::plugins::player::PlayerControl;
use crate::ships::Cargo;
use crate::ships::Ship;
use crate::world::ZoneId;

use super::components::{
    ContactsListContainer, FleetDetailDivider, FleetDetailText, FleetListContainer,
    FleetPanelMarker, IntelContentText, IntelPanelText, LogContentText, LogPanelMarker, MapUi,
    PlayerPanelText, TacticalPanelText, WorldUi,
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

    // Player HUD (world view, top-left)
    commands.spawn((
        PlayerPanelText,
        WorldUi,
        TextBundle::from_section(
            "",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::srgb(0.82, 0.88, 0.95),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(10.0),
            ..default()
        }),
    ));

    // Log panel (bottom-left)
    spawn_log_panel(&mut commands, &font);

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
            WorldUi,
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

pub fn update_player_panel(
    player: Query<(&Ship, &Cargo, &ZoneId), With<PlayerControl>>,
    mut panel: Query<&mut Text, With<PlayerPanelText>>,
) {
    let Some(mut text) = panel.iter_mut().next() else {
        return;
    };

    match player.single() {
        Ok((ship, cargo, zone_id)) => {
            let fuel_pct = if ship.fuel_capacity > 0.0 {
                (ship.fuel / ship.fuel_capacity) * 100.0
            } else {
                0.0
            };

            // Build fuel bar (10 chars) using ASCII
            let fuel_filled = ((fuel_pct / 10.0).round() as usize).min(10);
            let fuel_bar: String = "=".repeat(fuel_filled) + &"-".repeat(10 - fuel_filled);

            // Build ore bar (10 chars) using ASCII
            let ore_pct = if cargo.ore_capacity > 0 {
                (cargo.ore as f32 / cargo.ore_capacity as f32) * 100.0
            } else {
                0.0
            };
            let ore_filled = ((ore_pct / 10.0).round() as usize).min(10);
            let ore_bar: String = "=".repeat(ore_filled) + &"-".repeat(10 - ore_filled);

            text.0 = format!(
                "Zone {} | FUEL [{}] {:>3.0}% | ORE [{}] {:>3.0}%",
                zone_id.0, fuel_bar, fuel_pct, ore_bar, ore_pct
            );
        }
        Err(_) => {
            text.0 = String::new();
        }
    }
}
