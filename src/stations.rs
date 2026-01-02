use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StationKind {
    MiningOutpost,
    FuelDepot,
    SensorStation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StationState {
    Deploying,
    Operational,
    Strained,
    Failing,
    Failed,
}

#[derive(Component, Debug)]
pub struct Station {
    pub kind: StationKind,
    pub state: StationState,
    pub fuel: f32,
    pub fuel_capacity: f32,
}

#[derive(Component, Debug)]
pub struct StationBuild {
    pub remaining_seconds: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CrisisType {
    FuelShortage,
    PirateHarassment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CrisisStage {
    Stable,
    Strained,
    Failing,
    Resolved,
}

#[derive(Component, Debug)]
pub struct StationCrisis {
    pub crisis_type: CrisisType,
    pub stage: CrisisStage,
}

pub fn station_build_time_seconds(kind: StationKind) -> f32 {
    match kind {
        StationKind::MiningOutpost => 240.0,
        StationKind::FuelDepot => 180.0,
        StationKind::SensorStation => 120.0,
    }
}

pub fn station_fuel_capacity(kind: StationKind) -> f32 {
    match kind {
        StationKind::MiningOutpost => 30.0,
        StationKind::FuelDepot => 120.0,
        StationKind::SensorStation => 40.0,
    }
}

pub fn station_fuel_burn_per_minute(kind: StationKind) -> f32 {
    match kind {
        StationKind::MiningOutpost => 1.0,
        StationKind::FuelDepot => 0.5,
        StationKind::SensorStation => 0.75,
    }
}
