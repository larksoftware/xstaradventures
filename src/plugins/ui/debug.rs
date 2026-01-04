//! Debug panel systems.

use bevy::prelude::*;
use bevy::ui::Node as UiNode;
use std::path::Path;

use crate::compat::NodeBundle;
use crate::fleets::ScoutBehavior;
use crate::plugins::core::{DebugWindow, GameState, SimConfig};
use crate::plugins::render2d::{FocusMarker, IntelRefreshCooldown, MapZoomOverride, RenderToggles};
use crate::plugins::sim::SimTickCount;
use crate::plugins::worldgen::WorldSeed;
use crate::ships::Ship;
use crate::stations::Station;
use crate::world::{SystemIntel, SystemNode};

use super::components::DebugPanelText;
use super::panel::{PanelConfig, PanelPosition};

// =============================================================================
// Setup Systems
// =============================================================================

pub fn setup_debug_panel(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        return;
    }

    let font = asset_server.load(font_path);

    let debug_config = PanelConfig::at(PanelPosition::TopLeft)
        .with_margin(14.0)
        .with_background(Color::srgb(0.08, 0.1, 0.12))
        .with_padding(10.0);

    let mut debug_node = UiNode {
        width: Val::Auto,
        height: Val::Auto,
        ..default()
    };
    debug_config.apply_to_node(&mut debug_node);
    debug_node.top = Val::Px(80.0);

    commands
        .spawn((
            DebugPanelText,
            NodeBundle {
                node: debug_node,
                background_color: debug_config.background_color.unwrap_or(Color::NONE).into(),
                visibility: Visibility::Hidden,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(crate::compat::TextBundle::from_section(
                "Debug Panel",
                crate::compat::TextStyle {
                    font,
                    font_size: 12.0,
                    color: Color::srgb(0.85, 0.9, 0.95),
                },
            ));
        });
}

// =============================================================================
// Update Systems
// =============================================================================

#[allow(clippy::too_many_arguments)]
pub fn update_debug_panel(
    debug_window: Res<DebugWindow>,
    seed: Res<WorldSeed>,
    ticks: Res<SimTickCount>,
    config: Res<SimConfig>,
    toggles: Res<RenderToggles>,
    zoom: Res<MapZoomOverride>,
    cooldown: Res<IntelRefreshCooldown>,
    marker: Res<FocusMarker>,
    state: Res<State<GameState>>,
    stations: Query<&Station>,
    ships: Query<&Ship>,
    scouts: Query<&ScoutBehavior>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
    mut panel_container: Query<(&mut Visibility, &Children), With<DebugPanelText>>,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((mut visibility, children)) = panel_container.single_mut() {
        if debug_window.open {
            *visibility = Visibility::Visible;

            for child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    let mut body = String::from("=== DEBUG PANEL (F3 to close) ===\n\n");

                    body.push_str(&format!("Seed: {} | Tick: {}\n", seed.value, ticks.tick));
                    body.push_str(&format!(
                        "Rate: {:.0} Hz | Paused: {}\n",
                        config.tick_hz, config.paused
                    ));
                    body.push_str(&format!("State: {:?}\n\n", state.get()));

                    body.push_str("Render Toggles:\n");
                    body.push_str(&format!(
                        "  Rings: {} | Grid: {}\n",
                        if toggles.rings_enabled() { "On" } else { "Off" },
                        if toggles.grid_enabled() { "On" } else { "Off" }
                    ));
                    body.push_str(&format!(
                        "  Route Labels: {} | Node Labels: {}\n",
                        if toggles.route_labels_enabled() {
                            "On"
                        } else {
                            "Off"
                        },
                        if toggles.node_labels_enabled() {
                            "On"
                        } else {
                            "Off"
                        }
                    ));
                    body.push_str(&format!("  Zoom: {}\n\n", zoom.label()));

                    body.push_str(&format!("Stations: {}\n", stations.iter().count()));
                    body.push_str(&format!("Ships: {}\n", ships.iter().count()));
                    body.push_str(&format!("Scouts: {}\n\n", scouts.iter().count()));

                    body.push_str(&format!(
                        "Intel Refresh CD: {} ticks\n",
                        cooldown.remaining_ticks(ticks.tick)
                    ));

                    match marker.node_id() {
                        Some(node_id) => {
                            body.push_str(&format!("Focus: node {}\n", node_id));
                        }
                        None => {
                            body.push_str("Focus: --\n");
                        }
                    }

                    let revealed_count = nodes.iter().filter(|(_, intel)| intel.revealed).count();
                    body.push_str(&format!(
                        "\nNodes: {} revealed / {} total\n",
                        revealed_count,
                        nodes.iter().count()
                    ));

                    body.push_str("\nDebug Keybinds:\n");
                    body.push_str("  -/= : change seed\n");
                    body.push_str("  V   : reveal adjacent\n");
                    body.push_str("  U   : reveal all\n");
                    body.push_str("  Z   : clear reveals\n");
                    body.push_str("  B   : spawn station\n");
                    body.push_str("  S   : spawn scout\n");
                    body.push_str("  P   : spawn pirate\n");
                    body.push_str("  I   : refresh intel\n");
                    body.push_str("  O   : advance intel\n");
                    body.push_str("  K   : randomize modifiers\n");
                    body.push_str("\nRender Toggles:\n");
                    body.push_str("  N   : toggle nodes\n");
                    body.push_str("  R   : toggle routes\n");
                    body.push_str("  F   : toggle rings\n");
                    body.push_str("  G   : toggle grid\n");
                    body.push_str("  T   : toggle route labels\n");
                    body.push_str("  Y   : toggle node labels\n");

                    text.0 = body;
                    break;
                }
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}
