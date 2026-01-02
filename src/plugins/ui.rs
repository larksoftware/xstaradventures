use bevy::prelude::*;
use std::path::Path;

use crate::plugins::core::EventLog;
use crate::plugins::core::GameState;
use crate::plugins::core::SimConfig;
use crate::plugins::core::ViewMode;
use crate::plugins::render2d::FocusMarker;
use crate::plugins::render2d::IntelRefreshCooldown;
use crate::plugins::render2d::MapZoomOverride;
use crate::plugins::render2d::RenderToggles;
use crate::plugins::sim::SimTickCount;
use crate::plugins::worldgen::WorldSeed;
use crate::ships::{ship_default_role, FleetRole, Ship, ShipKind, ShipState};
use crate::stations::{Station, StationKind, StationState};
use crate::world::{
    zone_modifier_effect, KnowledgeLayer, Sector, SystemIntel, SystemNode, ZoneModifier,
};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_map_grid, setup_hud))
            .add_systems(
                Update,
                (ui_root, update_hud).run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                (
                    update_state_overlay,
                    update_log_panel,
                    update_cooldown_panel,
                    update_station_panel,
                    update_ship_panel,
                    update_focus_panel,
                    sync_map_ui_visibility,
                    sync_map_grid_visibility,
                ),
            )
            .add_systems(
                Update,
                (
                    update_node_panel,
                    update_hover_panel,
                    update_risk_panel,
                    update_modifier_panel,
                )
                    .run_if(view_is_map),
            )
            .init_resource::<HoveredNode>();
    }
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct StateText;

#[derive(Component)]
struct ControlsText;

#[derive(Component)]
struct LogText;

#[derive(Component)]
struct NodeListText;

#[derive(Component)]
struct HoverText;

#[derive(Component)]
struct LegendText;

#[derive(Resource, Default)]
pub struct HoveredNode {
    pub id: Option<u32>,
    pub layer: Option<KnowledgeLayer>,
    pub confidence: f32,
    pub modifier: Option<ZoneModifier>,
    pub screen_pos: Option<Vec2>,
    pub screen_size: Vec2,
}

#[derive(Component)]
struct RiskText;

#[derive(Component)]
struct ModifierPanelText;

#[derive(Component)]
struct CooldownText;

#[derive(Component)]
struct StationPanelText;

#[derive(Component)]
struct ShipPanelText;

#[derive(Component)]
struct FocusText;

#[derive(Component)]
pub struct MapUi;

#[derive(Component)]
struct WorldUi;

#[derive(Component)]
struct MapGridRoot;

#[derive(Component)]
struct MapGridLine;
fn ui_root() {
    // Placeholder: delegation panels and problems feed will render here.
}

fn setup_map_grid(mut commands: Commands) {
    let grid_color = Color::rgba(0.2, 0.25, 0.3, 0.35);
    let line_thickness = 1.0;
    let divisions = 12;

    commands
        .spawn((
            MapGridRoot,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                background_color: Color::NONE.into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            for i in 1..divisions {
                let percent = (i as f32) * 100.0 / (divisions as f32);

                parent.spawn((
                    MapGridLine,
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(percent),
                            top: Val::Px(0.0),
                            width: Val::Px(line_thickness),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: grid_color.into(),
                        ..default()
                    },
                ));

                parent.spawn((
                    MapGridLine,
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Percent(percent),
                            width: Val::Percent(100.0),
                            height: Val::Px(line_thickness),
                            ..default()
                        },
                        background_color: grid_color.into(),
                        ..default()
                    },
                ));
            }
        });
}

fn setup_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        info!("HUD font not found at {}", font_on_disk.display());
        return;
    }

    let font = asset_server.load(font_path);

    commands.spawn((
        HudText,
        TextBundle::from_section(
            "Seed: -- | Tick: --",
            TextStyle {
                font: font.clone(),
                font_size: 18.0,
                color: Color::rgb(0.9, 0.9, 0.95),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(10.0),
            ..default()
        }),
    ));

    commands.spawn((
        StateText,
        TextBundle::from_section(
            "State: --",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::rgb(0.8, 0.85, 0.92),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(36.0),
            ..default()
        }),
    ));

    commands.spawn((
        ControlsText,
        TextBundle::from_section(
            "Keys: Space pause | [ ] rate | - = seed | M map | N nodes | R routes | F rings | G grid | T route labels | Y node labels | V/A reveal | K mods | B/J spawn | S save | L load",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::rgb(0.7, 0.75, 0.82),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(58.0),
            ..default()
        }),
    ));

    commands.spawn((
        LegendText,
        TextBundle::from_section(
            "Legend: L0 Existence | L1 Geography | L2 Resources | L3 Threats | L4 Stability",
            TextStyle {
                font: font.clone(),
                font_size: 13.0,
                color: Color::rgb(0.65, 0.7, 0.78),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(80.0),
            ..default()
        }),
    ));

    commands.spawn((
        LegendText,
        TextBundle::from_section(
            "Label: L# % MOD | Mods: RAD Radiation | NEB Nebula | ORE Rich | CLR Clear",
            TextStyle {
                font: font.clone(),
                font_size: 13.0,
                color: Color::rgb(0.65, 0.7, 0.78),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(100.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "Icons: R Rad | N Neb | O Ore | C Clear | . None",
            TextStyle {
                font: font.clone(),
                font_size: 12.0,
                color: Color::rgb(0.6, 0.65, 0.72),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(118.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "Map: G grid | R routes | T route labels | Y node labels | V reveal adj | A reveal all | C zoom",
            TextStyle {
                font: font.clone(),
                font_size: 12.0,
                color: Color::rgb(0.6, 0.65, 0.72),
            },
        )
        .with_style(Style {
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
                color: Color::rgb(0.6, 0.65, 0.72),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(154.0),
            ..default()
        }),
    ));

    commands.spawn((
        LogText,
        TextBundle::from_section(
            "Log: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::rgb(0.7, 0.75, 0.82),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            bottom: Val::Px(14.0),
            ..default()
        }),
    ));

    commands.spawn((
        NodeListText,
        MapUi,
        TextBundle::from_section(
            "Nodes: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::rgb(0.7, 0.75, 0.82),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(10.0),
            ..default()
        }),
    ));

    commands.spawn((
        HoverText,
        MapUi,
        TextBundle::from_section(
            "Hover: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::rgb(0.7, 0.75, 0.82),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(160.0),
            ..default()
        }),
    ));

    commands.spawn((
        RiskText,
        MapUi,
        TextBundle::from_section(
            "Risk: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::rgb(0.7, 0.75, 0.82),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(220.0),
            ..default()
        }),
    ));

    commands.spawn((
        ModifierPanelText,
        MapUi,
        TextBundle::from_section(
            "Modifiers: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::rgb(0.7, 0.75, 0.82),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(260.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "N",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::rgb(0.65, 0.7, 0.78),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Px(8.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "S",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::rgb(0.65, 0.7, 0.78),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            bottom: Val::Px(8.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "W",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::rgb(0.65, 0.7, 0.78),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            top: Val::Percent(50.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "E",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::rgb(0.65, 0.7, 0.78),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            top: Val::Percent(50.0),
            ..default()
        }),
    ));

    commands.spawn((
        CooldownText,
        TextBundle::from_section(
            "Intel refresh: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::rgb(0.7, 0.75, 0.82),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(300.0),
            ..default()
        }),
    ));

    commands.spawn((
        StationPanelText,
        WorldUi,
        TextBundle::from_section(
            "Stations: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::rgb(0.7, 0.75, 0.82),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(340.0),
            ..default()
        }),
    ));

    commands.spawn((
        ShipPanelText,
        WorldUi,
        TextBundle::from_section(
            "Ships: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::rgb(0.7, 0.75, 0.82),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(380.0),
            ..default()
        }),
    ));

    commands.spawn((
        FocusText,
        WorldUi,
        TextBundle::from_section(
            "Focus: --",
            TextStyle {
                font,
                font_size: 13.0,
                color: Color::rgb(0.7, 0.8, 0.9),
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(420.0),
            ..default()
        }),
    ));
}

fn update_hud(
    seed: Res<WorldSeed>,
    ticks: Res<SimTickCount>,
    config: Res<SimConfig>,
    toggles: Res<RenderToggles>,
    view: Res<ViewMode>,
    zoom: Res<MapZoomOverride>,
    mut hud_text: Query<&mut Text, With<HudText>>,
) {
    if let Some(mut text) = hud_text.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            section.value = format!(
                "Seed: {} | Tick: {} | Rate: {:.0} Hz | Paused: {} | Rings: {} | Grid: {} | RouteLbl: {} | NodeLbl: {} | Zoom: {} | View: {:?}",
                seed.value,
                ticks.tick,
                config.tick_hz,
                config.paused,
                if toggles.rings_enabled() { "On" } else { "Off" },
                if toggles.grid_enabled() { "On" } else { "Off" },
                if toggles.route_labels_enabled() { "On" } else { "Off" },
                if toggles.node_labels_enabled() { "On" } else { "Off" },
                zoom.label(),
                *view
            );
        }
    }
}

fn update_state_overlay(
    state: Res<State<GameState>>,
    mut state_text: Query<&mut Text, With<StateText>>,
) {
    if let Some(mut text) = state_text.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            section.value = format!("State: {:?}", state.get());
        }
    }
}

fn update_log_panel(log: Res<EventLog>, mut log_text: Query<&mut Text, With<LogText>>) {
    if let Some(mut text) = log_text.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            let entries = log.entries();
            if entries.is_empty() {
                section.value = "Log: --".to_string();
            } else {
                let mut body = String::from("Log:\n");
                for entry in entries {
                    body.push_str("- ");
                    body.push_str(entry);
                    body.push('\n');
                }
                section.value = body.trim_end().to_string();
            }
        }
    }
}

fn update_node_panel(
    nodes: Query<(&SystemNode, &SystemIntel)>,
    mut panel: Query<&mut Text, With<NodeListText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            let mut entries = nodes
                .iter()
                .filter(|(_, intel)| intel.revealed)
                .map(|(node, intel)| (node.id, intel.layer, intel.confidence, node.modifier))
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.0);

            if entries.is_empty() {
                section.value = "Nodes: --".to_string();
            } else {
                let mut body = String::from("Nodes:\n");
                for (id, layer, confidence, modifier) in entries {
                    body.push_str(&format!(
                        "- {} L{} {:.0}% {} ({})\n",
                        id,
                        layer_to_short(layer),
                        confidence * 100.0,
                        modifier_to_short(modifier),
                        modifier_to_long(modifier)
                    ));
                }
                section.value = body.trim_end().to_string();
            }
        }
    }
}

fn layer_to_short(layer: crate::world::KnowledgeLayer) -> &'static str {
    match layer {
        KnowledgeLayer::Existence => "0",
        KnowledgeLayer::Geography => "1",
        KnowledgeLayer::Resources => "2",
        KnowledgeLayer::Threats => "3",
        KnowledgeLayer::Stability => "4",
    }
}

fn modifier_to_short(modifier: Option<ZoneModifier>) -> &'static str {
    match modifier {
        Some(ZoneModifier::HighRadiation) => "RAD",
        Some(ZoneModifier::NebulaInterference) => "NEB",
        Some(ZoneModifier::RichOreVeins) => "ORE",
        Some(ZoneModifier::ClearSignals) => "CLR",
        None => "--",
    }
}

fn modifier_to_long(modifier: Option<ZoneModifier>) -> &'static str {
    match modifier {
        Some(ZoneModifier::HighRadiation) => "High Radiation",
        Some(ZoneModifier::NebulaInterference) => "Nebula",
        Some(ZoneModifier::RichOreVeins) => "Rich Ore",
        Some(ZoneModifier::ClearSignals) => "Clear Signals",
        None => "",
    }
}

fn update_hover_panel(
    hovered: Res<HoveredNode>,
    sector: Res<Sector>,
    mut panel: Query<(&mut Text, &mut Style), With<HoverText>>,
) {
    if let Some((mut text, mut style)) = panel.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            match (hovered.id, hovered.screen_pos) {
                (Some(id), Some(pos)) => {
                    let layer = hovered.layer.unwrap_or(KnowledgeLayer::Existence);
                    let modifier = modifier_to_short(hovered.modifier);
                    let modifier_long = modifier_to_long(hovered.modifier);
                    let (route_risk, modifier_risk) = risk_breakdown(&sector);
                    section.value = format!(
                        "Hover: {} L{} {:.0}% {} {} | Risk r{:.2} m{:.2}",
                        id,
                        layer_to_short(layer),
                        hovered.confidence * 100.0,
                        modifier,
                        modifier_long,
                        route_risk,
                        modifier_risk
                    );
                    style.display = Display::Flex;
                    style.left = Val::Px(pos.x + 16.0);
                    style.top = Val::Px((hovered.screen_size.y - pos.y) + 16.0);
                }
                _ => {
                    section.value = "Hover: --".to_string();
                    style.display = Display::None;
                }
            }
        }
    }
}

fn update_risk_panel(sector: Res<Sector>, mut panel: Query<&mut Text, With<RiskText>>) {
    if let Some(mut text) = panel.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            let (route_risk, modifier_risk) = risk_breakdown(&sector);
            section.value = format!("Risk: route {:.2} | mod {:.2}", route_risk, modifier_risk);
        }
    }
}

fn risk_breakdown(sector: &Sector) -> (f32, f32) {
    let route_risk = if sector.routes.is_empty() {
        0.0
    } else {
        let total = sector.routes.iter().map(|route| route.risk).sum::<f32>();
        total / (sector.routes.len() as f32)
    };

    let modifier_risk = if sector.nodes.is_empty() {
        0.0
    } else {
        let total = sector
            .nodes
            .iter()
            .map(|node| {
                let effect = zone_modifier_effect(node.modifier);
                effect.fuel_risk + effect.confidence_risk + effect.pirate_risk
            })
            .sum::<f32>();
        total / (sector.nodes.len() as f32)
    };

    (route_risk, modifier_risk)
}

fn update_modifier_panel(
    sector: Res<Sector>,
    mut panel: Query<&mut Text, With<ModifierPanelText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            let mut counts = std::collections::BTreeMap::new();

            for node in &sector.nodes {
                let key = match node.modifier {
                    Some(ZoneModifier::HighRadiation) => "RAD",
                    Some(ZoneModifier::NebulaInterference) => "NEB",
                    Some(ZoneModifier::RichOreVeins) => "ORE",
                    Some(ZoneModifier::ClearSignals) => "CLR",
                    None => "NONE",
                };

                let entry = counts.entry(key).or_insert(0u32);
                *entry += 1;
            }

            let summary = counts
                .iter()
                .map(|(key, count)| format!("{}:{}", key, count))
                .collect::<Vec<_>>()
                .join(" ");

            section.value = format!("Modifiers: {}", summary);
        }
    }
}

fn update_cooldown_panel(
    ticks: Res<SimTickCount>,
    cooldown: Res<IntelRefreshCooldown>,
    mut panel: Query<&mut Text, With<CooldownText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            let remaining = cooldown.remaining_ticks(ticks.tick);
            if remaining == 0 {
                section.value = "Intel refresh: ready".to_string();
            } else {
                section.value = format!("Intel refresh: {}t", remaining);
            }
        }
    }
}

fn update_station_panel(
    stations: Query<(
        &Station,
        Option<&crate::stations::StationBuild>,
        Option<&crate::stations::StationCrisis>,
    )>,
    mut panel: Query<&mut Text, With<StationPanelText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            if stations.is_empty() {
                section.value = "Stations: --".to_string();
                return;
            }

            let mut kind_counts = std::collections::BTreeMap::new();
            let mut state_counts = std::collections::BTreeMap::new();
            let mut fuel_sum = 0.0;
            let mut fuel_capacity_sum = 0.0;

            let mut build_remaining = None;

            let mut crisis_count = 0u32;
            let mut fuel_crisis = 0u32;
            let mut pirate_crisis = 0u32;

            for (station, build, crisis) in stations.iter() {
                let kind_key = match station.kind {
                    StationKind::MiningOutpost => "Mine",
                    StationKind::FuelDepot => "Fuel",
                    StationKind::SensorStation => "Sensor",
                };
                let state_key = match station.state {
                    StationState::Deploying => "Deploy",
                    StationState::Operational => "Op",
                    StationState::Strained => "Strain",
                    StationState::Failing => "Fail",
                    StationState::Failed => "Dead",
                };

                let kind_entry = kind_counts.entry(kind_key).or_insert(0u32);
                *kind_entry += 1;

                let state_entry = state_counts.entry(state_key).or_insert(0u32);
                *state_entry += 1;

                fuel_sum += station.fuel;
                fuel_capacity_sum += station.fuel_capacity;

                if let Some(build) = build {
                    if build_remaining.map_or(true, |current| build.remaining_seconds > current) {
                        build_remaining = Some(build.remaining_seconds);
                    }
                }

                if crisis.is_some() {
                    crisis_count += 1;
                    if let Some(crisis) = crisis {
                        match crisis.crisis_type {
                            crate::stations::CrisisType::FuelShortage => fuel_crisis += 1,
                            crate::stations::CrisisType::PirateHarassment => pirate_crisis += 1,
                        }
                    }
                }
            }

            let kind_summary = kind_counts
                .iter()
                .map(|(key, count)| format!("{}:{}", key, count))
                .collect::<Vec<_>>()
                .join(" ");

            let state_summary = state_counts
                .iter()
                .map(|(key, count)| format!("{}:{}", key, count))
                .collect::<Vec<_>>()
                .join(" ");

            let fuel_pct = if fuel_capacity_sum > 0.0 {
                (fuel_sum / fuel_capacity_sum) * 100.0
            } else {
                0.0
            };

            let crisis_breakdown = if crisis_count > 0 {
                format!("Fuel {} | Pirate {}", fuel_crisis, pirate_crisis)
            } else {
                "None".to_string()
            };

            if let Some(remaining) = build_remaining {
                section.value = format!(
                    "Stations: {} | {} | Fuel {:.0}% | Build {:.0}s | Crisis {}",
                    kind_summary, state_summary, fuel_pct, remaining, crisis_breakdown
                );
            } else {
                section.value = format!(
                    "Stations: {} | {} | Fuel {:.0}% | Crisis {}",
                    kind_summary, state_summary, fuel_pct, crisis_breakdown
                );
            }
        }
    }
}

fn update_ship_panel(ships: Query<&Ship>, mut panel: Query<&mut Text, With<ShipPanelText>>) {
    if let Some(mut text) = panel.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            if ships.is_empty() {
                section.value = "Ships: --".to_string();
                return;
            }

            let mut kind_counts = std::collections::BTreeMap::new();
            let mut state_counts = std::collections::BTreeMap::new();
            let mut role_counts = std::collections::BTreeMap::new();
            let mut fuel_sum = 0.0;
            let mut fuel_capacity_sum = 0.0;

            for ship in ships.iter() {
                let kind_key = match ship.kind {
                    ShipKind::PlayerShip => "Player",
                    ShipKind::Scout => "Scout",
                    ShipKind::Miner => "Miner",
                    ShipKind::Security => "Sec",
                };
                let state_key = match ship.state {
                    ShipState::Idle => "Idle",
                    ShipState::InTransit => "Transit",
                    ShipState::Executing => "Exec",
                    ShipState::Returning => "Return",
                    ShipState::Refueling => "Refuel",
                    ShipState::Damaged => "Dmg",
                    ShipState::Disabled => "Down",
                };

                let kind_entry = kind_counts.entry(kind_key).or_insert(0u32);
                *kind_entry += 1;

                let state_entry = state_counts.entry(state_key).or_insert(0u32);
                *state_entry += 1;

                let role_key = match ship_default_role(ship.kind) {
                    FleetRole::Scout => "Scout",
                    FleetRole::Mining => "Mine",
                    FleetRole::Security => "Sec",
                };
                let role_entry = role_counts.entry(role_key).or_insert(0u32);
                *role_entry += 1;

                fuel_sum += ship.fuel;
                fuel_capacity_sum += ship.fuel_capacity;
            }

            let kind_summary = kind_counts
                .iter()
                .map(|(key, count)| format!("{}:{}", key, count))
                .collect::<Vec<_>>()
                .join(" ");

            let state_summary = state_counts
                .iter()
                .map(|(key, count)| format!("{}:{}", key, count))
                .collect::<Vec<_>>()
                .join(" ");

            let role_summary = role_counts
                .iter()
                .map(|(key, count)| format!("{}:{}", key, count))
                .collect::<Vec<_>>()
                .join(" ");

            let fuel_pct = if fuel_capacity_sum > 0.0 {
                (fuel_sum / fuel_capacity_sum) * 100.0
            } else {
                0.0
            };

            section.value = format!(
                "Ships: {} | {} | Roles {} | Fuel {:.0}%",
                kind_summary, state_summary, role_summary, fuel_pct
            );
        }
    }
}

fn update_focus_panel(marker: Res<FocusMarker>, mut panel: Query<&mut Text, With<FocusText>>) {
    if let Some(mut text) = panel.iter_mut().next() {
        if let Some(section) = text.sections.get_mut(0) {
            match marker.node_id() {
                Some(node_id) => {
                    section.value = format!("Focus: node {}", node_id);
                }
                None => {
                    section.value = "Focus: --".to_string();
                }
            };
        }
    }
}

fn view_is_map(view: Res<ViewMode>) -> bool {
    matches!(*view, ViewMode::Map)
}

fn sync_map_ui_visibility(
    view: Res<ViewMode>,
    mut elements: Query<(&mut Style, Option<&MapUi>, Option<&WorldUi>)>,
) {
    let display = if matches!(*view, ViewMode::Map) {
        Display::Flex
    } else {
        Display::None
    };

    let world_display = if matches!(*view, ViewMode::World) {
        Display::Flex
    } else {
        Display::None
    };

    for (mut style, map_ui, world_ui) in elements.iter_mut() {
        if map_ui.is_some() {
            style.display = display;
        }
        if world_ui.is_some() {
            style.display = world_display;
        }
    }
}

fn sync_map_grid_visibility(
    view: Res<ViewMode>,
    toggles: Res<RenderToggles>,
    mut roots: Query<&mut Style, With<MapGridRoot>>,
) {
    let show = matches!(*view, ViewMode::Map) && toggles.grid_enabled();
    let display = if show { Display::Flex } else { Display::None };

    for mut style in roots.iter_mut() {
        style.display = display;
    }
}
