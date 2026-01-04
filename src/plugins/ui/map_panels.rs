//! Map view panels: grid, node list, hover, risk, modifiers.

use bevy::prelude::*;
use bevy::ui::Node as UiNode;

use crate::compat::NodeBundle;
use crate::plugins::core::{DebugWindow, ViewMode};
use crate::plugins::render2d::RenderToggles;
use crate::world::{
    zone_modifier_effect, KnowledgeLayer, Sector, SystemIntel, SystemNode, ZoneModifier,
};

use super::components::{
    layer_to_short, modifier_to_long, modifier_to_short, HoverText, HoveredNode, MapGridLine,
    MapGridRoot, MapUi, ModifierPanelText, NodeListText, RiskText, WorldUi,
};

// =============================================================================
// Setup Systems
// =============================================================================

pub fn setup_map_grid(mut commands: Commands) {
    let grid_color = Color::srgba(0.2, 0.25, 0.3, 0.35);
    let line_thickness = 1.0;
    let divisions = 12;

    commands
        .spawn((
            MapGridRoot,
            NodeBundle {
                node: UiNode {
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
                        node: UiNode {
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
                        node: UiNode {
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

pub fn setup_hover_panel(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = std::path::Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        return;
    }

    let font = asset_server.load(font_path);

    commands.spawn((
        HoverText,
        MapUi,
        crate::compat::TextBundle::from_section(
            "",
            crate::compat::TextStyle {
                font,
                font_size: 13.0,
                color: Color::srgb(0.9, 0.95, 1.0),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            display: Display::None,
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        })
        .with_background_color(Color::srgba(0.05, 0.08, 0.12, 0.9)),
    ));
}

// =============================================================================
// Update Systems
// =============================================================================

pub fn update_node_panel(
    nodes: Query<(&SystemNode, &SystemIntel)>,
    mut panel: Query<&mut Text, With<NodeListText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        let mut entries = nodes
            .iter()
            .filter(|(_, intel)| intel.revealed)
            .map(|(node, intel)| (node.id, intel.layer, intel.confidence, node.modifier))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.0);

        if entries.is_empty() {
            text.0 = "Nodes: --".to_string();
        } else {
            let mut body = String::from("Nodes:\n");
            for (id, layer, confidence, modifier) in entries {
                let modifier_suffix = match modifier {
                    Some(_) => format!(
                        " {} ({})",
                        modifier_to_short(modifier),
                        modifier_to_long(modifier)
                    ),
                    None => String::new(),
                };
                body.push_str(&format!(
                    "- {} L{} {:.0}%{}\n",
                    id,
                    layer_to_short(layer),
                    confidence * 100.0,
                    modifier_suffix
                ));
            }
            text.0 = body.trim_end().to_string();
        }
    }
}

pub fn update_hover_panel(
    hovered: Res<HoveredNode>,
    mut panel: Query<(&mut Text, &mut UiNode), With<HoverText>>,
) {
    if let Some((mut text, mut node)) = panel.iter_mut().next() {
        match (hovered.id, hovered.screen_pos) {
            (Some(id), Some(pos)) => {
                let layer = hovered.layer.unwrap_or(KnowledgeLayer::Existence);
                let modifier_suffix = match hovered.modifier {
                    Some(_) => format!(
                        " {} {}",
                        modifier_to_short(hovered.modifier),
                        modifier_to_long(hovered.modifier)
                    ),
                    None => String::new(),
                };
                text.0 = format!(
                    "{} | L{} {:.0}%{}",
                    id,
                    layer_to_short(layer),
                    hovered.confidence * 100.0,
                    modifier_suffix,
                );
                node.display = Display::Flex;
                node.right = Val::Auto;
                node.left = Val::Px(pos.x + 20.0);
                node.top = Val::Px(pos.y + 20.0);
            }
            _ => {
                node.display = Display::None;
                node.left = Val::Px(-1000.0);
                node.top = Val::Px(-1000.0);
                text.0.clear();
            }
        }
    }
}

pub fn update_risk_panel(sector: Res<Sector>, mut panel: Query<&mut Text, With<RiskText>>) {
    if let Some(mut text) = panel.iter_mut().next() {
        let (route_risk, modifier_risk) = risk_breakdown(&sector);
        text.0 = format!("Risk: route {:.2} | mod {:.2}", route_risk, modifier_risk);
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

pub fn update_modifier_panel(
    sector: Res<Sector>,
    mut panel: Query<&mut Text, With<ModifierPanelText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
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

        text.0 = format!("Modifiers: {}", summary);
    }
}

// =============================================================================
// Visibility Systems
// =============================================================================

pub fn sync_map_ui_visibility(
    view: Res<ViewMode>,
    debug_window: Res<DebugWindow>,
    mut elements: Query<(&mut UiNode, Option<&MapUi>, Option<&WorldUi>)>,
) {
    let display = if matches!(*view, ViewMode::Map) && !debug_window.open {
        Display::Flex
    } else {
        Display::None
    };

    let world_display = if matches!(*view, ViewMode::World) {
        Display::Flex
    } else {
        Display::None
    };

    for (mut node, map_ui, world_ui) in elements.iter_mut() {
        if map_ui.is_some() {
            node.display = display;
        }
        if world_ui.is_some() {
            node.display = world_display;
        }
    }
}

pub fn sync_map_grid_visibility(
    view: Res<ViewMode>,
    toggles: Res<RenderToggles>,
    debug_window: Res<DebugWindow>,
    mut roots: Query<&mut UiNode, With<MapGridRoot>>,
) {
    let show = matches!(*view, ViewMode::Map) && toggles.grid_enabled() && !debug_window.open;
    let display = if show { Display::Flex } else { Display::None };

    for mut node in roots.iter_mut() {
        node.display = display;
    }
}
