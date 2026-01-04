//! Shared UI components, markers, and utility functions.

use bevy::prelude::*;

use crate::world::{KnowledgeLayer, ZoneModifier};

// =============================================================================
// View Markers
// =============================================================================

/// Marker for UI elements that should only be visible in Map view
#[derive(Component)]
pub struct MapUi;

/// Marker for UI elements that should only be visible in World view
#[derive(Component)]
pub struct WorldUi;

// =============================================================================
// HUD Components
// =============================================================================

#[derive(Component)]
pub struct PlayerPanelText;

// =============================================================================
// Log Components
// =============================================================================

#[derive(Component)]
pub struct LogPanelMarker;

#[derive(Component)]
pub struct LogContentText;

// =============================================================================
// Map Panel Components
// =============================================================================

#[derive(Component)]
pub struct NodeListText;

#[derive(Component)]
pub struct HoverText;

#[derive(Component)]
pub struct RiskText;

#[derive(Component)]
pub struct ModifierPanelText;

#[derive(Component)]
pub struct MapGridRoot;

#[derive(Component)]
pub struct MapGridLine;

// =============================================================================
// Contacts Panel Components
// =============================================================================

#[derive(Component)]
pub struct TacticalPanelText;

#[derive(Component)]
pub struct ContactsListContainer;

/// Component marking a clickable contact item in the Contacts panel
#[derive(Component)]
pub struct ContactItem {
    pub index: usize,
}

// =============================================================================
// Intel Panel Components
// =============================================================================

#[derive(Component)]
pub struct IntelPanelText;

#[derive(Component)]
pub struct IntelContentText;

/// Information about a targeted entity for the Intel panel
#[derive(Debug, Clone)]
pub struct IntelInfo {
    #[allow(dead_code)]
    pub entity: Entity,
    pub label: String,
    pub position: Vec2,
    pub distance: f32,
}

// =============================================================================
// Fleet Panel Components
// =============================================================================

#[derive(Component)]
pub struct FleetPanelMarker;

#[derive(Component)]
pub struct FleetListContainer;

#[derive(Component)]
pub struct FleetDetailText;

/// Component marking a clickable fleet item in the Fleet panel
#[derive(Component)]
pub struct FleetItem {
    pub index: usize,
}

/// Marker for empty state text in Fleet panel
#[derive(Component)]
pub struct FleetEmptyText;

/// Marker for the divider between fleet list and detail
#[derive(Component)]
pub struct FleetDetailDivider;

// =============================================================================
// Debug Panel Components
// =============================================================================

#[derive(Component)]
pub struct DebugPanelText;

// =============================================================================
// Resources
// =============================================================================

#[derive(Resource, Default)]
pub struct HoveredNode {
    pub id: Option<u32>,
    pub layer: Option<KnowledgeLayer>,
    pub confidence: f32,
    pub modifier: Option<ZoneModifier>,
    pub screen_pos: Option<Vec2>,
    pub screen_size: Vec2,
}

/// Tracks which fleet unit is selected for detail view
#[derive(Resource, Default)]
pub struct SelectedFleetUnit {
    pub index: Option<usize>,
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Returns the color for a contact item based on selection and hover state
pub fn contact_item_color(is_selected: bool, is_hovered: bool) -> Color {
    match (is_selected, is_hovered) {
        (true, _) => Color::srgb(1.0, 1.0, 1.0), // Selected: white
        (false, true) => Color::srgb(0.5, 0.9, 0.9), // Hovered: bright cyan
        (false, false) => Color::srgb(0.0, 1.0, 1.0), // Default: cyan
    }
}

pub fn layer_to_short(layer: KnowledgeLayer) -> &'static str {
    match layer {
        KnowledgeLayer::Existence => "0",
        KnowledgeLayer::Geography => "1",
        KnowledgeLayer::Resources => "2",
        KnowledgeLayer::Threats => "3",
        KnowledgeLayer::Stability => "4",
    }
}

pub fn modifier_to_short(modifier: Option<ZoneModifier>) -> &'static str {
    match modifier {
        Some(ZoneModifier::HighRadiation) => "RAD",
        Some(ZoneModifier::NebulaInterference) => "NEB",
        Some(ZoneModifier::RichOreVeins) => "ORE",
        Some(ZoneModifier::ClearSignals) => "CLR",
        None => "--",
    }
}

pub fn modifier_to_long(modifier: Option<ZoneModifier>) -> &'static str {
    match modifier {
        Some(ZoneModifier::HighRadiation) => "High Radiation",
        Some(ZoneModifier::NebulaInterference) => "Nebula",
        Some(ZoneModifier::RichOreVeins) => "Rich Ore",
        Some(ZoneModifier::ClearSignals) => "Clear Signals",
        None => "",
    }
}
