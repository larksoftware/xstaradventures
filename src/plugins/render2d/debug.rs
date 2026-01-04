//! Debug systems for render toggles and intel manipulation.

use bevy::prelude::*;

use crate::plugins::core::InputBindings;
use crate::plugins::sim::{advance_intel_layer, refresh_intel};
use crate::world::SystemIntel;

use super::components::{shift_pressed, NodeVisual};
use super::map::{IntelRefreshCooldown, RenderToggles};

// =============================================================================
// Systems
// =============================================================================

pub fn handle_render_toggles(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut toggles: ResMut<RenderToggles>,
    mut commands: Commands,
    visuals: Query<Entity, With<NodeVisual>>,
) {
    if !shift_pressed(&input) {
        return;
    }

    let mut updated = false;

    if input.just_pressed(bindings.toggle_nodes) {
        toggles.show_nodes = !toggles.show_nodes;
        updated = true;

        if !toggles.show_nodes {
            for entity in visuals.iter() {
                commands.entity(entity).despawn();
            }
        }

        info!("Render nodes: {}", toggles.show_nodes);
    }

    if input.just_pressed(bindings.toggle_routes) {
        toggles.show_routes = !toggles.show_routes;
        updated = true;
        info!("Render routes: {}", toggles.show_routes);
    }

    if input.just_pressed(bindings.toggle_rings) {
        toggles.show_rings = !toggles.show_rings;
        updated = true;
        info!("Render rings: {}", toggles.show_rings);
    }

    if input.just_pressed(bindings.toggle_grid) {
        toggles.show_grid = !toggles.show_grid;
        updated = true;
        info!("Render grid: {}", toggles.show_grid);
    }

    if input.just_pressed(bindings.toggle_route_labels) {
        toggles.show_route_labels = !toggles.show_route_labels;
        updated = true;
        info!("Render route labels: {}", toggles.show_route_labels);
    }

    if input.just_pressed(bindings.toggle_node_labels) {
        toggles.show_node_labels = !toggles.show_node_labels;
        updated = true;
        info!("Render node labels: {}", toggles.show_node_labels);
    }

    if updated {
        // Leave toggles updated; drawing uses current flags.
    }
}

pub fn handle_intel_refresh(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    ticks: Res<crate::plugins::sim::SimTickCount>,
    mut cooldown: ResMut<IntelRefreshCooldown>,
    mut intel_query: Query<&mut SystemIntel>,
) {
    if !shift_pressed(&input) || !input.just_pressed(bindings.refresh_intel) {
        return;
    }

    if ticks.tick < cooldown.next_allowed_tick {
        return;
    }

    for mut intel in intel_query.iter_mut() {
        refresh_intel(&mut intel, ticks.tick);
    }
    cooldown.next_allowed_tick = ticks.tick.saturating_add(cooldown.cooldown_ticks);
    info!("Intel refreshed");
}

pub fn handle_intel_advance(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut intel_query: Query<&mut SystemIntel>,
) {
    if !shift_pressed(&input) || !input.just_pressed(bindings.advance_intel) {
        return;
    }

    for mut intel in intel_query.iter_mut() {
        advance_intel_layer(&mut intel);
    }
    info!("Intel layer advanced");
}

#[allow(clippy::type_complexity)]
pub fn debug_player_components(
    player: Query<
        (
            &Transform,
            Option<&Sprite>,
            Option<&Visibility>,
            Option<&ViewVisibility>,
            Option<&InheritedVisibility>,
        ),
        With<crate::plugins::player::PlayerControl>,
    >,
) {
    static mut LOGGED: bool = false;
    unsafe {
        if !LOGGED {
            if let Ok((transform, sprite, vis, view_vis, inherited_vis)) = player.single() {
                info!("=== PLAYER SHIP COMPONENTS ===");
                info!(
                    "  Transform: pos=({:.1}, {:.1}, {:.1})",
                    transform.translation.x, transform.translation.y, transform.translation.z
                );
                info!("  Sprite: {}", if sprite.is_some() { "YES" } else { "NO" });
                info!("  Visibility: {:?}", vis);
                info!("  ViewVisibility: {:?}", view_vis);
                info!("  InheritedVisibility: {:?}", inherited_vis);
                LOGGED = true;
            }
        }
    }
}
