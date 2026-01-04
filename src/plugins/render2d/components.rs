//! Shared components, resources, and utility functions for render2d module.

use bevy::prelude::*;

use crate::ore::OreNode;
use crate::pirates::{PirateBase, PirateShip};
use crate::plugins::core::FogConfig;
use crate::ships::ShipKind;
use crate::stations::Station;
use crate::world::{KnowledgeLayer, Sector, SystemNode, ZoneModifier};

// Type aliases for complex query filter combinations
pub type StationSpawnFilter = (With<Station>, Without<StationVisualMarker>);
pub type OreSpawnFilter = (With<OreNode>, Without<OreVisualMarker>);
pub type PirateBaseSpawnFilter = (With<PirateBase>, Without<PirateBaseVisualMarker>);
pub type PirateShipSpawnFilter = (With<PirateShip>, Without<PirateShipVisualMarker>);
pub type ShipSpawnFilter = (
    Without<ShipVisual>,
    Without<ShipVisualMarker>,
    Without<Sprite>,
);

/// Check if either Shift key is pressed (for debug key modifiers)
pub fn shift_pressed(input: &ButtonInput<KeyCode>) -> bool {
    input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight)
}

// =============================================================================
// Map View Components
// =============================================================================

#[derive(Component)]
pub struct NodeVisual {
    pub target: Entity,
}

#[derive(Component)]
pub struct RouteLabel;

#[derive(Component)]
pub struct NodeLabel;

#[derive(Component)]
pub struct NodeVisualMarker;

// =============================================================================
// World View Entity Components
// =============================================================================

#[derive(Component)]
pub struct StationVisual {
    pub target: Entity,
}

#[derive(Component)]
pub struct StationVisualMarker;

#[derive(Component)]
pub struct StationLabel;

#[derive(Component)]
pub struct ShipVisual {
    pub target: Entity,
}

#[derive(Component)]
pub struct ShipVisualMarker;

#[derive(Component)]
pub struct ShipLabel;

#[derive(Component)]
pub struct OreVisual {
    pub target: Entity,
}

#[derive(Component)]
pub struct OreVisualMarker;

#[derive(Component)]
pub struct PirateBaseVisual {
    pub target: Entity,
}

#[derive(Component)]
pub struct PirateBaseVisualMarker;

#[derive(Component)]
pub struct PirateShipVisual {
    pub target: Entity,
}

#[derive(Component)]
pub struct PirateShipVisualMarker;

// =============================================================================
// Utility Functions
// =============================================================================

pub fn layer_short(layer: KnowledgeLayer) -> &'static str {
    match layer {
        KnowledgeLayer::Existence => "E",
        KnowledgeLayer::Geography => "G",
        KnowledgeLayer::Resources => "R",
        KnowledgeLayer::Threats => "T",
        KnowledgeLayer::Stability => "S",
    }
}

pub fn modifier_icon(modifier: Option<ZoneModifier>) -> &'static str {
    match modifier {
        Some(ZoneModifier::HighRadiation) => "R",
        Some(ZoneModifier::NebulaInterference) => "N",
        Some(ZoneModifier::RichOreVeins) => "O",
        Some(ZoneModifier::ClearSignals) => "C",
        None => "",
    }
}

pub fn station_kind_short(kind: crate::stations::StationKind) -> &'static str {
    match kind {
        crate::stations::StationKind::MiningOutpost => "M",
        crate::stations::StationKind::FuelDepot => "F",
        crate::stations::StationKind::SensorStation => "S",
    }
}

pub fn ship_kind_short(kind: ShipKind) -> &'static str {
    match kind {
        ShipKind::Scout => "Sct",
        ShipKind::Miner => "Min",
        ShipKind::Security => "Sec",
        ShipKind::PlayerShip => "Ply",
    }
}

pub fn ship_state_short(state: crate::ships::ShipState) -> &'static str {
    match state {
        crate::ships::ShipState::Idle => "I",
        crate::ships::ShipState::InTransit => "T",
        crate::ships::ShipState::Executing => "E",
        crate::ships::ShipState::Returning => "R",
        crate::ships::ShipState::Refueling => "F",
        crate::ships::ShipState::Damaged => "D",
        crate::ships::ShipState::Disabled => "X",
    }
}

pub fn layer_floor(layer: KnowledgeLayer, fog: &FogConfig) -> f32 {
    match layer {
        KnowledgeLayer::Existence => fog.floor_existence,
        KnowledgeLayer::Geography => fog.floor_geography,
        KnowledgeLayer::Resources => fog.floor_resources,
        KnowledgeLayer::Threats => fog.floor_threats,
        KnowledgeLayer::Stability => fog.floor_stability,
    }
}

pub fn find_node_position(nodes: &[SystemNode], id: u32) -> Option<Vec2> {
    for node in nodes {
        if node.id == id {
            return Some(node.position);
        }
    }
    None
}

pub fn risk_color(risk: f32) -> Color {
    let t = risk.clamp(0.0, 1.0);
    let low = LinearRgba::new(0.2, 0.7, 0.4, 1.0);
    let high = LinearRgba::new(0.9, 0.25, 0.2, 1.0);
    Color::linear_rgba(
        low.red + (high.red - low.red) * t,
        low.green + (high.green - low.green) * t,
        low.blue + (high.blue - low.blue) * t,
        1.0,
    )
}

/// Determines if an entity should be visible based on zone matching.
pub fn is_visible_in_zone(entity_zone: Option<u32>, player_zone: u32) -> bool {
    match entity_zone {
        Some(zone) => zone == player_zone,
        None => true, // Entities without zones are always visible
    }
}

#[allow(dead_code)] // Used in tests
pub fn map_center(sector: &Sector) -> Vec2 {
    if sector.nodes.is_empty() {
        return Vec2::ZERO;
    }

    let mut sum = Vec2::ZERO;
    let mut count = 0.0;

    for node in &sector.nodes {
        sum += node.position;
        count += 1.0;
    }

    if count > 0.0 {
        sum / count
    } else {
        Vec2::ZERO
    }
}
