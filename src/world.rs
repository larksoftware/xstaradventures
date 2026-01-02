use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Sector {
    pub nodes: Vec<SystemNode>,
    pub routes: Vec<RouteEdge>,
}

#[derive(Component, Clone)]
pub struct SystemNode {
    pub id: u32,
    pub position: Vec2,
    pub modifier: Option<ZoneModifier>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RouteEdge {
    pub from: u32,
    pub to: u32,
    pub distance: f32,
    pub risk: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum KnowledgeLayer {
    Existence,
    Geography,
    Resources,
    Threats,
    Stability,
}

#[derive(Component, Clone, Debug)]
pub struct SystemIntel {
    pub layer: KnowledgeLayer,
    pub confidence: f32,
    pub last_seen_tick: u64,
    pub revealed: bool,
    pub revealed_tick: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ZoneModifier {
    HighRadiation,
    NebulaInterference,
    RichOreVeins,
    ClearSignals,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ZoneModifierEffect {
    pub fuel_risk: f32,
    pub confidence_risk: f32,
    pub pirate_risk: f32,
}

pub fn zone_modifier_effect(modifier: Option<ZoneModifier>) -> ZoneModifierEffect {
    match modifier {
        Some(ZoneModifier::HighRadiation) => ZoneModifierEffect {
            fuel_risk: 0.2,
            confidence_risk: 0.05,
            pirate_risk: 0.1,
        },
        Some(ZoneModifier::NebulaInterference) => ZoneModifierEffect {
            fuel_risk: 0.0,
            confidence_risk: 0.2,
            pirate_risk: 0.1,
        },
        Some(ZoneModifier::RichOreVeins) => ZoneModifierEffect {
            fuel_risk: 0.0,
            confidence_risk: 0.0,
            pirate_risk: 0.2,
        },
        Some(ZoneModifier::ClearSignals) => ZoneModifierEffect {
            fuel_risk: 0.0,
            confidence_risk: -0.15,
            pirate_risk: 0.0,
        },
        None => ZoneModifierEffect::default(),
    }
}
