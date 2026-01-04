//! UI plugin for the game interface.
//!
//! This module provides all UI panels and systems:
//! - HUD (view info, player stats, cooldowns)
//! - Log panel (subspace transmissions)
//! - Map panels (nodes, hover, risk, modifiers, grid)
//! - Contacts and Intel panels (world view targeting)
//! - Fleet panel (scout management)
//! - Debug panel (F3)

mod components;
mod contacts;
mod debug;
mod fleet;
mod hud;
mod intel;
mod log;
mod map_panels;
pub mod panel;

use bevy::prelude::*;

use crate::plugins::core::{GameState, ViewMode};

// Re-export public types
#[allow(unused_imports)]
pub use components::{
    contact_item_color, ContactItem, FleetItem, HoveredNode, IntelInfo, MapUi, SelectedFleetUnit,
};
#[allow(unused_imports)]
pub use panel::{PanelConfig, PanelPosition};

// =============================================================================
// Plugin
// =============================================================================

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                map_panels::setup_map_grid,
                hud::setup_hud,
                debug::setup_debug_panel,
            ),
        )
        .add_systems(
            Update,
            (ui_root, debug::update_debug_panel).run_if(in_state(GameState::InGame)),
        )
        .add_systems(
            Update,
            (
                log::update_log_panel,
                hud::update_player_panel,
                fleet::update_fleet_panel,
                map_panels::sync_map_ui_visibility,
                map_panels::sync_map_grid_visibility,
            ),
        )
        .add_systems(
            Update,
            (
                map_panels::update_node_panel,
                map_panels::update_hover_panel,
                map_panels::update_risk_panel,
                map_panels::update_modifier_panel,
            )
                .run_if(view_is_map),
        )
        .add_systems(
            Update,
            (
                contacts::update_tactical_panel,
                contacts::handle_contact_clicks,
                contacts::update_contact_item_styles,
                intel::update_intel_panel,
                fleet::handle_fleet_clicks,
                fleet::update_fleet_item_styles,
                fleet::update_fleet_detail,
                fleet::handle_panel_scroll,
            )
                .run_if(view_is_world),
        )
        .init_resource::<components::HoveredNode>()
        .init_resource::<components::SelectedFleetUnit>();
    }
}

// =============================================================================
// Run Conditions
// =============================================================================

fn view_is_map(view: Res<ViewMode>) -> bool {
    matches!(*view, ViewMode::Map)
}

fn view_is_world(view: Res<ViewMode>) -> bool {
    matches!(*view, ViewMode::World)
}

// =============================================================================
// Systems
// =============================================================================

fn ui_root() {
    // Placeholder: delegation panels and problems feed will render here.
}
