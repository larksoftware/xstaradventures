use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShipKind {
    PlayerShip,
    Scout,
    Miner,
    Security,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FleetRole {
    Scout,
    Mining,
    Security,
}

impl Default for FleetRole {
    fn default() -> Self {
        FleetRole::Security
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShipState {
    Idle,
    InTransit,
    Executing,
    Returning,
    Refueling,
    Damaged,
    Disabled,
}

#[derive(Component, Debug)]
pub struct Ship {
    pub kind: ShipKind,
    pub state: ShipState,
    pub fuel: f32,
    pub fuel_capacity: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Fleet {
    pub role: FleetRole,
}

#[derive(Component, Debug, Default)]
pub struct ShipFuelAlert {
    pub low: bool,
    pub critical: bool,
}

pub fn ship_default_role(kind: ShipKind) -> FleetRole {
    match kind {
        ShipKind::PlayerShip => FleetRole::Security,
        ShipKind::Scout => FleetRole::Scout,
        ShipKind::Miner => FleetRole::Mining,
        ShipKind::Security => FleetRole::Security,
    }
}

pub fn ship_fuel_capacity(kind: ShipKind) -> f32 {
    match kind {
        ShipKind::PlayerShip => 60.0,
        ShipKind::Scout => 30.0,
        ShipKind::Miner => 45.0,
        ShipKind::Security => 45.0,
    }
}

pub fn ship_fuel_burn_per_minute(kind: ShipKind) -> f32 {
    match kind {
        ShipKind::PlayerShip => 2.0,
        ShipKind::Scout => 1.0,
        ShipKind::Miner => 1.5,
        ShipKind::Security => 1.5,
    }
}
