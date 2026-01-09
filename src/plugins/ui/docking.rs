//! Docking menu UI systems.

use bevy::prelude::*;
use bevy::ui::Node as UiNode;
use std::path::Path;

use crate::compat::{NodeBundle, TextBundle, TextStyle};
use crate::plugins::player::{DockingState, PlayerControl};
use crate::ships::{Cargo, Credits, Ship};
use crate::stations::{
    outpost_ore_to_credits, RefineryJob, RefineryStorage, ShipyardJob, Station, StationKind,
    StationState, OUTPOST_BUY_FUEL_OPTIONS, OUTPOST_SELL_ORE_OPTIONS,
};

use super::components::{
    DockingMenuBuildScoutButton, DockingMenuBuyFuelButton, DockingMenuCancelButton,
    DockingMenuCargoText, DockingMenuCollectButton, DockingMenuCollectSection,
    DockingMenuConvertButton, DockingMenuCreditsText, DockingMenuJobSection, DockingMenuJobText,
    DockingMenuOutpostSection, DockingMenuRoot, DockingMenuSellAllOreButton,
    DockingMenuSellOreButton, DockingMenuStatus, DockingMenuTitle, DockingMenuUndockButton,
};

// =============================================================================
// Constants
// =============================================================================

/// Cost in ore to build a scout
pub const SCOUT_BUILD_COST: u32 = 15;

/// Time in seconds to build a scout
pub const SCOUT_BUILD_TIME: f32 = 120.0;

/// Ore conversion options: (ore_in, fuel_out, time_seconds)
pub const REFINERY_OPTIONS: [(u32, f32, f32); 2] = [(5, 10.0, 60.0), (10, 20.0, 90.0)];

// =============================================================================
// Setup Systems
// =============================================================================

pub fn setup_docking_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        return;
    }

    let font = asset_server.load(font_path);

    // Docking menu panel (centered, initially hidden)
    commands
        .spawn((
            DockingMenuRoot,
            NodeBundle {
                node: UiNode {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect {
                        left: Val::Px(-160.0),
                        top: Val::Px(-200.0),
                        ..default()
                    },
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    min_width: Val::Px(320.0),
                    ..default()
                },
                background_color: Color::srgba(0.02, 0.05, 0.08, 0.95).into(),
                border_color: Color::srgb(0.0, 0.8, 0.8).into(),
                visibility: Visibility::Hidden,
                ..default()
            },
        ))
        .with_children(|parent| {
            // Title (station name and type)
            parent.spawn((
                DockingMenuTitle,
                TextBundle::from_section(
                    "STATION",
                    TextStyle {
                        font: font.clone(),
                        font_size: 16.0,
                        color: Color::srgb(0.0, 1.0, 1.0),
                    },
                ),
            ));

            // Status line
            parent.spawn((
                DockingMenuStatus,
                TextBundle::from_section(
                    "Status: Operational",
                    TextStyle {
                        font: font.clone(),
                        font_size: 13.0,
                        color: Color::srgb(0.6, 0.8, 0.6),
                    },
                ),
            ));

            // Divider
            parent.spawn(NodeBundle {
                node: UiNode {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
                background_color: Color::srgb(0.0, 0.4, 0.4).into(),
                ..default()
            });

            // Job section (progress + cancel button)
            parent
                .spawn((
                    DockingMenuJobSection,
                    NodeBundle {
                        node: UiNode {
                            flex_direction: FlexDirection::Column,
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|job_section| {
                    job_section.spawn((
                        DockingMenuJobText,
                        TextBundle::from_section(
                            "",
                            TextStyle {
                                font: font.clone(),
                                font_size: 12.0,
                                color: Color::srgb(0.8, 0.8, 0.5),
                            },
                        ),
                    ));

                    // Cancel button
                    job_section.spawn((
                        DockingMenuCancelButton,
                        Interaction::None,
                        TextBundle::from_section(
                            "[CANCEL - 50% refund]",
                            TextStyle {
                                font: font.clone(),
                                font_size: 13.0,
                                color: Color::srgb(0.8, 0.4, 0.4),
                            },
                        )
                        .with_node(UiNode {
                            margin: UiRect::vertical(Val::Px(2.0)),
                            ..default()
                        }),
                    ));
                });

            // Divider
            parent.spawn(NodeBundle {
                node: UiNode {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
                background_color: Color::srgb(0.0, 0.4, 0.4).into(),
                ..default()
            });

            // Actions section header
            parent.spawn(TextBundle::from_section(
                "ACTIONS",
                TextStyle {
                    font: font.clone(),
                    font_size: 13.0,
                    color: Color::srgb(0.7, 0.7, 0.7),
                },
            ));

            // Build Scout button (Shipyard)
            parent.spawn((
                DockingMenuBuildScoutButton,
                Interaction::None,
                TextBundle::from_section(
                    format!(
                        "Build Scout ({} ore, {}s)",
                        SCOUT_BUILD_COST, SCOUT_BUILD_TIME as u32
                    ),
                    TextStyle {
                        font: font.clone(),
                        font_size: 13.0,
                        color: Color::srgb(0.4, 0.8, 0.4),
                    },
                )
                .with_node(UiNode {
                    margin: UiRect::vertical(Val::Px(2.0)),
                    ..default()
                }),
            ));

            // Convert ore buttons (Refinery)
            for (ore_in, fuel_out, time) in REFINERY_OPTIONS {
                parent.spawn((
                    DockingMenuConvertButton { ore_amount: ore_in },
                    Interaction::None,
                    TextBundle::from_section(
                        format!(
                            "Convert {} ore -> {} fuel ({}s)",
                            ore_in, fuel_out as u32, time as u32
                        ),
                        TextStyle {
                            font: font.clone(),
                            font_size: 13.0,
                            color: Color::srgb(0.4, 0.8, 0.4),
                        },
                    )
                    .with_node(UiNode {
                        margin: UiRect::vertical(Val::Px(2.0)),
                        ..default()
                    }),
                ));
            }

            // Collect section (Refinery)
            parent
                .spawn((
                    DockingMenuCollectSection,
                    NodeBundle {
                        node: UiNode {
                            flex_direction: FlexDirection::Column,
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|collect| {
                    collect.spawn(TextBundle::from_section(
                        "COLLECT",
                        TextStyle {
                            font: font.clone(),
                            font_size: 13.0,
                            color: Color::srgb(0.7, 0.7, 0.7),
                        },
                    ));

                    collect.spawn((
                        DockingMenuCollectButton,
                        Interaction::None,
                        TextBundle::from_section(
                            "[TAKE ALL FUEL]",
                            TextStyle {
                                font: font.clone(),
                                font_size: 13.0,
                                color: Color::srgb(0.4, 0.6, 0.8),
                            },
                        )
                        .with_node(UiNode {
                            margin: UiRect::vertical(Val::Px(2.0)),
                            ..default()
                        }),
                    ));
                });

            // Outpost trading section (hidden by default)
            parent
                .spawn((
                    DockingMenuOutpostSection,
                    NodeBundle {
                        node: UiNode {
                            flex_direction: FlexDirection::Column,
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        },
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                ))
                .with_children(|outpost| {
                    // BUY FUEL section
                    outpost.spawn(TextBundle::from_section(
                        "BUY FUEL",
                        TextStyle {
                            font: font.clone(),
                            font_size: 13.0,
                            color: Color::srgb(0.7, 0.7, 0.7),
                        },
                    ));

                    for option in OUTPOST_BUY_FUEL_OPTIONS {
                        outpost.spawn((
                            DockingMenuBuyFuelButton {
                                fuel_amount: option.fuel_amount,
                                credit_cost: option.credit_cost,
                            },
                            Interaction::None,
                            TextBundle::from_section(
                                format!(
                                    "> {} fuel for {} cr",
                                    option.fuel_amount, option.credit_cost
                                ),
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 13.0,
                                    color: Color::srgb(0.4, 0.8, 0.4),
                                },
                            )
                            .with_node(UiNode {
                                margin: UiRect::vertical(Val::Px(2.0)),
                                ..default()
                            }),
                        ));
                    }

                    // SELL ORE section
                    outpost.spawn(
                        TextBundle::from_section(
                            "SELL ORE",
                            TextStyle {
                                font: font.clone(),
                                font_size: 13.0,
                                color: Color::srgb(0.7, 0.7, 0.7),
                            },
                        )
                        .with_node(UiNode {
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        }),
                    );

                    for option in OUTPOST_SELL_ORE_OPTIONS {
                        outpost.spawn((
                            DockingMenuSellOreButton {
                                ore_amount: option.ore_amount,
                                credit_reward: option.credit_reward,
                            },
                            Interaction::None,
                            TextBundle::from_section(
                                format!(
                                    "> {} ore -> {} cr",
                                    option.ore_amount, option.credit_reward
                                ),
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 13.0,
                                    color: Color::srgb(0.8, 0.8, 0.4),
                                },
                            )
                            .with_node(UiNode {
                                margin: UiRect::vertical(Val::Px(2.0)),
                                ..default()
                            }),
                        ));
                    }

                    // Sell ALL ore button
                    outpost.spawn((
                        DockingMenuSellAllOreButton,
                        Interaction::None,
                        TextBundle::from_section(
                            "> SELL ALL ORE",
                            TextStyle {
                                font: font.clone(),
                                font_size: 13.0,
                                color: Color::srgb(0.8, 0.6, 0.3),
                            },
                        )
                        .with_node(UiNode {
                            margin: UiRect::vertical(Val::Px(2.0)),
                            ..default()
                        }),
                    ));
                });

            // Divider
            parent.spawn(NodeBundle {
                node: UiNode {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
                background_color: Color::srgb(0.0, 0.4, 0.4).into(),
                ..default()
            });

            // Cargo and credits display
            parent.spawn((
                DockingMenuCargoText,
                TextBundle::from_section(
                    "YOUR CARGO\nOre: 0/50  Fuel: 0/100",
                    TextStyle {
                        font: font.clone(),
                        font_size: 12.0,
                        color: Color::srgb(0.6, 0.7, 0.8),
                    },
                ),
            ));

            parent.spawn((
                DockingMenuCreditsText,
                TextBundle::from_section(
                    "Credits: 50",
                    TextStyle {
                        font: font.clone(),
                        font_size: 12.0,
                        color: Color::srgb(0.8, 0.7, 0.4),
                    },
                )
                .with_node(UiNode {
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                }),
            ));

            // Divider
            parent.spawn(NodeBundle {
                node: UiNode {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
                background_color: Color::srgb(0.0, 0.4, 0.4).into(),
                ..default()
            });

            // Undock button
            parent.spawn((
                DockingMenuUndockButton,
                Interaction::None,
                TextBundle::from_section(
                    "[UNDOCK]",
                    TextStyle {
                        font: font.clone(),
                        font_size: 13.0,
                        color: Color::srgb(0.8, 0.6, 0.4),
                    },
                )
                .with_node(UiNode {
                    margin: UiRect::vertical(Val::Px(2.0)),
                    ..default()
                }),
            ));
        });
}

// =============================================================================
// Update Systems
// =============================================================================

/// Show/hide the docking menu based on docking state
#[allow(clippy::type_complexity)]
pub fn update_docking_menu_visibility(
    docking: Res<DockingState>,
    mut menu: Query<&mut Visibility, With<DockingMenuRoot>>,
) {
    let Ok(mut visibility) = menu.single_mut() else {
        return;
    };

    if docking.is_docked() {
        *visibility = Visibility::Visible;
    } else {
        *visibility = Visibility::Hidden;
    }
}

/// Update the docking menu content based on docked station
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn update_docking_menu_content(
    docking: Res<DockingState>,
    player_query: Query<(&Cargo, &Credits, &Ship), With<PlayerControl>>,
    stations: Query<(
        &Station,
        &Name,
        Option<&ShipyardJob>,
        Option<&RefineryJob>,
        Option<&RefineryStorage>,
    )>,
    mut title_text: Query<&mut Text, With<DockingMenuTitle>>,
    mut status_text: Query<&mut Text, (With<DockingMenuStatus>, Without<DockingMenuTitle>)>,
    mut job_text: Query<
        &mut Text,
        (
            With<DockingMenuJobText>,
            Without<DockingMenuTitle>,
            Without<DockingMenuStatus>,
        ),
    >,
    mut cargo_text: Query<
        &mut Text,
        (
            With<DockingMenuCargoText>,
            Without<DockingMenuTitle>,
            Without<DockingMenuStatus>,
            Without<DockingMenuJobText>,
        ),
    >,
    mut credits_text: Query<
        &mut Text,
        (
            With<DockingMenuCreditsText>,
            Without<DockingMenuTitle>,
            Without<DockingMenuStatus>,
            Without<DockingMenuJobText>,
            Without<DockingMenuCargoText>,
        ),
    >,
    mut job_section: Query<&mut Visibility, With<DockingMenuJobSection>>,
    mut cancel_btn: Query<
        &mut Visibility,
        (
            With<DockingMenuCancelButton>,
            Without<DockingMenuJobSection>,
        ),
    >,
    mut build_btn: Query<
        &mut Visibility,
        (
            With<DockingMenuBuildScoutButton>,
            Without<DockingMenuJobSection>,
            Without<DockingMenuCancelButton>,
        ),
    >,
    mut convert_btns: Query<
        &mut Visibility,
        (
            With<DockingMenuConvertButton>,
            Without<DockingMenuJobSection>,
            Without<DockingMenuCancelButton>,
            Without<DockingMenuBuildScoutButton>,
        ),
    >,
    mut collect_section: Query<
        &mut Visibility,
        (
            With<DockingMenuCollectSection>,
            Without<DockingMenuJobSection>,
            Without<DockingMenuCancelButton>,
            Without<DockingMenuBuildScoutButton>,
            Without<DockingMenuConvertButton>,
        ),
    >,
    mut outpost_section: Query<
        &mut Visibility,
        (
            With<DockingMenuOutpostSection>,
            Without<DockingMenuJobSection>,
            Without<DockingMenuCancelButton>,
            Without<DockingMenuBuildScoutButton>,
            Without<DockingMenuConvertButton>,
            Without<DockingMenuCollectSection>,
        ),
    >,
) {
    let Some(station_entity) = docking.docked_at else {
        return;
    };

    let Ok((station, name, shipyard_job, refinery_job, refinery_storage)) =
        stations.get(station_entity)
    else {
        return;
    };

    let player_data = player_query.single().ok();

    // Update title
    if let Ok(mut text) = title_text.single_mut() {
        let kind_str = match station.kind {
            StationKind::Shipyard => "SHIPYARD",
            StationKind::Refinery => "REFINERY",
            StationKind::Outpost => "FRONTIER OUTPOST",
            _ => "STATION",
        };
        text.0 = format!("{} - {}", kind_str, name.as_str());
    }

    // Update status
    if let Ok(mut text) = status_text.single_mut() {
        if matches!(station.kind, StationKind::Outpost) {
            text.0 = "Status: Open for Trade".to_string();
        } else {
            let status_str = match station.state {
                StationState::Deploying => "Deploying...",
                StationState::Operational => "Operational",
                StationState::Strained => "Strained (job paused)",
                StationState::Failing => "Failing (job paused)",
                StationState::Failed => "Failed",
            };
            text.0 = format!(
                "Status: {} | Fuel: {:.0}/{:.0}",
                status_str, station.fuel, station.fuel_capacity
            );
        }
    }

    let is_outpost = matches!(station.kind, StationKind::Outpost);

    // Update job progress (not for Outpost)
    let has_job = shipyard_job.is_some() || refinery_job.is_some();

    if let Ok(mut vis) = job_section.single_mut() {
        *vis = if has_job && !is_outpost {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut vis) = cancel_btn.single_mut() {
        *vis = if has_job && !is_outpost {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut text) = job_text.single_mut() {
        if let Some(job) = shipyard_job {
            let progress = 1.0 - (job.remaining_seconds / SCOUT_BUILD_TIME);
            let bar = progress_bar(progress, 20);
            text.0 = format!(
                "BUILDING: Scout ({:.0}s remaining)\n[{}]",
                job.remaining_seconds, bar
            );
        } else if let Some(job) = refinery_job {
            let total_time = 60.0; // Approximate
            let progress = 1.0 - (job.remaining_seconds / total_time).min(1.0);
            let bar = progress_bar(progress, 20);
            text.0 = format!(
                "CONVERTING: {} ore -> {} fuel ({:.0}s)\n[{}]",
                job.ore_in, job.fuel_out as u32, job.remaining_seconds, bar
            );
        } else {
            text.0 = String::new();
        }
    }

    // Show/hide build scout button (Shipyard only, no active job)
    if let Ok(mut vis) = build_btn.single_mut() {
        *vis = if matches!(station.kind, StationKind::Shipyard) && shipyard_job.is_none() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Show/hide convert buttons (Refinery only, no active job)
    for mut vis in convert_btns.iter_mut() {
        *vis = if matches!(station.kind, StationKind::Refinery) && refinery_job.is_none() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Show/hide collect section (Refinery only)
    if let Ok(mut vis) = collect_section.single_mut() {
        let has_fuel = refinery_storage.is_some_and(|s| s.fuel > 0.0);
        *vis = if matches!(station.kind, StationKind::Refinery) && has_fuel {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Show/hide outpost section (Outpost only)
    if let Ok(mut vis) = outpost_section.single_mut() {
        *vis = if is_outpost {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Update cargo and credits display
    if let Some((cargo, credits, ship)) = player_data {
        if let Ok(mut text) = cargo_text.single_mut() {
            text.0 = format!(
                "YOUR CARGO\nOre: {}/{}  Fuel: {:.0}/{:.0}",
                cargo.ore, cargo.ore_capacity, ship.fuel, ship.fuel_capacity
            );
        }
        if let Ok(mut text) = credits_text.single_mut() {
            text.0 = format!("Credits: {}", credits.amount);
        }
    }
}

/// Handle button clicks in the docking menu
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn handle_docking_menu_clicks(
    mut commands: Commands,
    mut docking: ResMut<DockingState>,
    mut player_query: Query<(&mut Cargo, &mut Credits, &mut Ship), With<PlayerControl>>,
    mut stations: Query<(
        Entity,
        &mut Station,
        Option<&ShipyardJob>,
        Option<&RefineryJob>,
        Option<&mut RefineryStorage>,
    )>,
    undock_btn: Query<&Interaction, (With<DockingMenuUndockButton>, Changed<Interaction>)>,
    build_btn: Query<&Interaction, (With<DockingMenuBuildScoutButton>, Changed<Interaction>)>,
    convert_btns: Query<(&Interaction, &DockingMenuConvertButton), Changed<Interaction>>,
    cancel_btn: Query<&Interaction, (With<DockingMenuCancelButton>, Changed<Interaction>)>,
    collect_btn: Query<&Interaction, (With<DockingMenuCollectButton>, Changed<Interaction>)>,
    buy_fuel_btns: Query<(&Interaction, &DockingMenuBuyFuelButton), Changed<Interaction>>,
    sell_ore_btns: Query<(&Interaction, &DockingMenuSellOreButton), Changed<Interaction>>,
    sell_all_ore_btn: Query<
        &Interaction,
        (With<DockingMenuSellAllOreButton>, Changed<Interaction>),
    >,
) {
    let Some(station_entity) = docking.docked_at else {
        return;
    };

    // Handle undock
    for interaction in undock_btn.iter() {
        if matches!(interaction, Interaction::Pressed) {
            docking.undock();
            return;
        }
    }

    // Handle cancel job
    for interaction in cancel_btn.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((entity, _, shipyard_job, refinery_job, _)) = stations.get(station_entity) {
                let mut refund = 0u32;

                if let Some(job) = shipyard_job {
                    refund = job.ore_in / 2; // 50% refund
                    commands.entity(entity).remove::<ShipyardJob>();
                } else if let Some(job) = refinery_job {
                    refund = job.ore_in / 2; // 50% refund
                    commands.entity(entity).remove::<RefineryJob>();
                }

                // Refund ore to player
                if refund > 0 {
                    if let Ok((mut cargo, _, _)) = player_query.single_mut() {
                        cargo.add_ore(refund);
                    }
                }
            }
            return;
        }
    }

    let Ok((mut cargo, mut credits, mut ship)) = player_query.single_mut() else {
        return;
    };

    // Handle build scout
    for interaction in build_btn.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if cargo.ore >= SCOUT_BUILD_COST {
                if let Ok((entity, station, existing_job, _, _)) = stations.get(station_entity) {
                    if matches!(station.kind, StationKind::Shipyard) && existing_job.is_none() {
                        cargo.remove_ore(SCOUT_BUILD_COST);
                        commands.entity(entity).insert(ShipyardJob {
                            ore_in: SCOUT_BUILD_COST,
                            fuel_in: 0.0,
                            remaining_seconds: SCOUT_BUILD_TIME,
                        });
                    }
                }
            }
            return;
        }
    }

    // Handle convert ore
    for (interaction, convert_btn) in convert_btns.iter() {
        if matches!(interaction, Interaction::Pressed) {
            let ore_amount = convert_btn.ore_amount;
            if cargo.ore >= ore_amount {
                if let Ok((entity, station, _, existing_job, _)) = stations.get(station_entity) {
                    if matches!(station.kind, StationKind::Refinery) && existing_job.is_none() {
                        // Find the matching conversion option
                        for (ore_in, fuel_out, time) in REFINERY_OPTIONS {
                            if ore_in == ore_amount {
                                cargo.remove_ore(ore_amount);
                                commands.entity(entity).insert(RefineryJob {
                                    ore_in: ore_amount,
                                    fuel_out,
                                    remaining_seconds: time,
                                });
                                break;
                            }
                        }
                    }
                }
            }
            return;
        }
    }

    // Handle collect fuel
    for interaction in collect_btn.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, _, _, _, Some(mut storage))) = stations.get_mut(station_entity) {
                let free_space = ship.fuel_capacity - ship.fuel;
                let to_take = storage.fuel.min(free_space);
                if to_take > 0.0 {
                    storage.fuel -= to_take;
                    ship.fuel += to_take;
                }
            }
            return;
        }
    }

    // Handle buy fuel (Outpost)
    for (interaction, buy_btn) in buy_fuel_btns.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, station, _, _, _)) = stations.get(station_entity) {
                if matches!(station.kind, StationKind::Outpost) {
                    // Check if player can afford and has cargo space
                    let free_fuel_space = ship.fuel_capacity - ship.fuel;
                    if credits.can_afford(buy_btn.credit_cost)
                        && free_fuel_space >= buy_btn.fuel_amount as f32
                    {
                        credits.try_spend(buy_btn.credit_cost);
                        ship.fuel += buy_btn.fuel_amount as f32;
                    }
                }
            }
            return;
        }
    }

    // Handle sell ore (Outpost)
    for (interaction, sell_btn) in sell_ore_btns.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, station, _, _, _)) = stations.get(station_entity) {
                if matches!(station.kind, StationKind::Outpost) && cargo.ore >= sell_btn.ore_amount
                {
                    cargo.remove_ore(sell_btn.ore_amount);
                    credits.add(sell_btn.credit_reward);
                }
            }
            return;
        }
    }

    // Handle sell all ore (Outpost)
    for interaction in sell_all_ore_btn.iter() {
        if matches!(interaction, Interaction::Pressed) {
            if let Ok((_, station, _, _, _)) = stations.get(station_entity) {
                if matches!(station.kind, StationKind::Outpost) && cargo.ore > 0 {
                    let credit_reward = outpost_ore_to_credits(cargo.ore);
                    cargo.ore = 0;
                    credits.add(credit_reward);
                }
            }
            return;
        }
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

fn progress_bar(progress: f32, width: usize) -> String {
    let filled = ((progress * width as f32).round() as usize).min(width);
    let empty = width - filled;
    format!("{}{}", "=".repeat(filled), "-".repeat(empty))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_empty() {
        assert_eq!(progress_bar(0.0, 10), "----------");
    }

    #[test]
    fn progress_bar_half() {
        assert_eq!(progress_bar(0.5, 10), "=====-----");
    }

    #[test]
    fn progress_bar_full() {
        assert_eq!(progress_bar(1.0, 10), "==========");
    }

    #[test]
    fn scout_build_cost_is_15_ore() {
        assert_eq!(SCOUT_BUILD_COST, 15);
    }

    #[test]
    fn scout_build_time_is_120_seconds() {
        assert!((SCOUT_BUILD_TIME - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn refinery_has_two_options() {
        assert_eq!(REFINERY_OPTIONS.len(), 2);
    }

    #[test]
    fn refinery_first_option_is_5_ore() {
        assert_eq!(REFINERY_OPTIONS[0].0, 5);
        assert!((REFINERY_OPTIONS[0].1 - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn refinery_second_option_is_10_ore() {
        assert_eq!(REFINERY_OPTIONS[1].0, 10);
        assert!((REFINERY_OPTIONS[1].1 - 20.0).abs() < f32::EPSILON);
    }
}
