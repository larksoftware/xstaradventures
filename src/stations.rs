use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StationKind {
    // Player-buildable stations
    MiningOutpost,
    FuelDepot,
    SensorStation,
    Shipyard,
    Refinery,
    // NPC-only stations
    Outpost, // Independent trader station
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

#[derive(Component, Debug, Default)]
pub struct StationCrisisLog {
    pub last_type: Option<CrisisType>,
    pub last_stage: Option<CrisisStage>,
}

#[derive(Component, Debug)]
pub struct StationProduction {
    pub ore: f32,
    pub ore_capacity: f32,
}

/// A refinery job converting ore to fuel
#[derive(Component, Debug, Clone)]
#[allow(dead_code)]
pub struct RefineryJob {
    pub ore_in: u32,
    pub fuel_out: f32,
    pub remaining_seconds: f32,
}

#[allow(dead_code)]
impl RefineryJob {
    /// Create a new refinery job
    pub fn new(ore_in: u32) -> Self {
        // 2 ore -> 1 fuel, takes 60 seconds
        let fuel_out = ore_in as f32 / 2.0;
        Self {
            ore_in,
            fuel_out,
            remaining_seconds: 60.0,
        }
    }

    /// Update job timer, returns true if complete
    pub fn tick(&mut self, delta_seconds: f32) -> bool {
        self.remaining_seconds -= delta_seconds;
        self.remaining_seconds <= 0.0
    }
}

/// A shipyard job building a scout
#[derive(Component, Debug, Clone)]
#[allow(dead_code)]
pub struct ShipyardJob {
    pub ore_in: u32,
    pub fuel_in: f32,
    pub remaining_seconds: f32,
}

#[allow(dead_code)]
impl ShipyardJob {
    /// Create a new shipyard job for building a scout
    pub fn new() -> Self {
        Self {
            ore_in: 30,
            fuel_in: 15.0,
            remaining_seconds: 120.0,
        }
    }

    /// Update job timer, returns true if complete
    pub fn tick(&mut self, delta_seconds: f32) -> bool {
        self.remaining_seconds -= delta_seconds;
        self.remaining_seconds <= 0.0
    }
}

impl Default for ShipyardJob {
    fn default() -> Self {
        Self::new()
    }
}

/// Storage for refined fuel at a refinery
#[derive(Component, Debug, Clone, Default)]
#[allow(dead_code)]
pub struct RefineryStorage {
    pub fuel: f32,
    pub fuel_capacity: f32,
}

#[allow(dead_code)]
impl RefineryStorage {
    /// Create new refinery storage with default capacity
    pub fn new() -> Self {
        Self {
            fuel: 0.0,
            fuel_capacity: 50.0,
        }
    }

    /// Add fuel to storage, returns amount actually added
    pub fn add_fuel(&mut self, amount: f32) -> f32 {
        let free = (self.fuel_capacity - self.fuel).max(0.0);
        let added = amount.min(free);
        self.fuel += added;
        added
    }

    /// Remove fuel from storage, returns amount actually removed
    pub fn remove_fuel(&mut self, amount: f32) -> f32 {
        let removed = amount.min(self.fuel);
        self.fuel -= removed;
        removed
    }

    /// Get available space
    pub fn free_space(&self) -> f32 {
        (self.fuel_capacity - self.fuel).max(0.0)
    }
}

/// Storage for completed scouts at a shipyard
#[derive(Component, Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ShipyardStorage {
    pub ready_scouts: u8,
    pub capacity: u8,
}

#[allow(dead_code)]
impl ShipyardStorage {
    /// Create new shipyard storage with default capacity
    pub fn new() -> Self {
        Self {
            ready_scouts: 0,
            capacity: 3,
        }
    }

    /// Add a completed scout, returns true if successful
    pub fn add_scout(&mut self) -> bool {
        if self.ready_scouts < self.capacity {
            self.ready_scouts += 1;
            true
        } else {
            false
        }
    }

    /// Remove a scout for deployment, returns true if successful
    pub fn remove_scout(&mut self) -> bool {
        if self.ready_scouts > 0 {
            self.ready_scouts -= 1;
            true
        } else {
            false
        }
    }

    /// Check if storage is full
    pub fn is_full(&self) -> bool {
        self.ready_scouts >= self.capacity
    }

    /// Get number of available slots
    pub fn free_slots(&self) -> u8 {
        self.capacity.saturating_sub(self.ready_scouts)
    }
}

pub fn station_build_time_seconds(kind: StationKind) -> f32 {
    match kind {
        StationKind::MiningOutpost => 180.0,
        StationKind::FuelDepot => 135.0,
        StationKind::SensorStation => 90.0,
        StationKind::Shipyard => 240.0,
        StationKind::Refinery => 200.0,
        StationKind::Outpost => 0.0, // Not player-buildable
    }
}

pub fn station_fuel_capacity(kind: StationKind) -> f32 {
    match kind {
        StationKind::MiningOutpost => 30.0,
        StationKind::FuelDepot => 120.0,
        StationKind::SensorStation => 40.0,
        StationKind::Shipyard => 50.0,
        StationKind::Refinery => 60.0,
        StationKind::Outpost => 0.0, // Self-sufficient, doesn't track fuel
    }
}

pub fn station_fuel_burn_per_minute(kind: StationKind) -> f32 {
    match kind {
        StationKind::MiningOutpost => 0.6,
        StationKind::FuelDepot => 0.3,
        StationKind::SensorStation => 0.45,
        StationKind::Shipyard => 0.5,
        StationKind::Refinery => 0.4,
        StationKind::Outpost => 0.0, // Self-sufficient
    }
}

pub fn station_ore_capacity(kind: StationKind) -> f32 {
    match kind {
        StationKind::MiningOutpost => 80.0,
        StationKind::FuelDepot => 0.0,
        StationKind::SensorStation => 0.0,
        StationKind::Shipyard => 100.0,
        StationKind::Refinery => 80.0,
        StationKind::Outpost => 0.0, // Doesn't store ore
    }
}

pub fn station_ore_production_per_minute(kind: StationKind) -> f32 {
    match kind {
        StationKind::MiningOutpost => 3.5,
        StationKind::FuelDepot => 0.0,
        StationKind::SensorStation => 0.0,
        StationKind::Shipyard => 0.0,
        StationKind::Refinery => 0.0,
        StationKind::Outpost => 0.0, // Doesn't produce ore
    }
}

/// Check if station kind is an NPC-owned type (not player-buildable).
#[allow(dead_code)]
pub fn is_npc_station(kind: StationKind) -> bool {
    matches!(kind, StationKind::Outpost)
}

// =============================================================================
// Outpost Trading
// =============================================================================

/// Outpost trading option for buying fuel
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub struct OutpostBuyFuelOption {
    pub fuel_amount: u32,
    pub credit_cost: u32,
}

/// Outpost trading option for selling ore
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub struct OutpostSellOreOption {
    pub ore_amount: u32,
    pub credit_reward: u32,
}

/// Available fuel purchase options at Outposts.
/// Price ratio: 5 credits → 10 fuel (2 fuel per credit)
#[allow(dead_code)]
pub const OUTPOST_BUY_FUEL_OPTIONS: [OutpostBuyFuelOption; 3] = [
    OutpostBuyFuelOption {
        fuel_amount: 10,
        credit_cost: 5,
    },
    OutpostBuyFuelOption {
        fuel_amount: 25,
        credit_cost: 12,
    },
    OutpostBuyFuelOption {
        fuel_amount: 50,
        credit_cost: 25,
    },
];

/// Available ore selling options at Outposts.
/// Price ratio: 5 ore → 10 credits (2 credits per ore)
#[allow(dead_code)]
pub const OUTPOST_SELL_ORE_OPTIONS: [OutpostSellOreOption; 2] = [
    OutpostSellOreOption {
        ore_amount: 5,
        credit_reward: 10,
    },
    OutpostSellOreOption {
        ore_amount: 10,
        credit_reward: 20,
    },
];

/// Calculate credit reward for selling a specific amount of ore.
/// Rate: 2 credits per ore
#[allow(dead_code)]
pub fn outpost_ore_to_credits(ore: u32) -> u32 {
    ore * 2
}

#[cfg(test)]
mod tests {
    use super::{station_build_time_seconds, StationKind};

    #[test]
    fn station_build_time_values() {
        assert_eq!(
            station_build_time_seconds(StationKind::MiningOutpost),
            180.0
        );
        assert_eq!(station_build_time_seconds(StationKind::FuelDepot), 135.0);
        assert_eq!(station_build_time_seconds(StationKind::SensorStation), 90.0);
    }

    #[test]
    fn station_fuel_capacity_values() {
        assert_eq!(
            super::station_fuel_capacity(StationKind::MiningOutpost),
            30.0
        );
        assert_eq!(super::station_fuel_capacity(StationKind::FuelDepot), 120.0);
        assert_eq!(
            super::station_fuel_capacity(StationKind::SensorStation),
            40.0
        );
    }

    #[test]
    fn station_fuel_burn_values() {
        assert_eq!(
            super::station_fuel_burn_per_minute(StationKind::MiningOutpost),
            0.6
        );
        assert_eq!(
            super::station_fuel_burn_per_minute(StationKind::FuelDepot),
            0.3
        );
        assert_eq!(
            super::station_fuel_burn_per_minute(StationKind::SensorStation),
            0.45
        );
    }

    #[test]
    fn station_fuel_burn_rates_nonnegative() {
        let kinds = [
            StationKind::MiningOutpost,
            StationKind::FuelDepot,
            StationKind::SensorStation,
        ];

        for kind in kinds {
            let burn = super::station_fuel_burn_per_minute(kind);
            assert!(burn >= 0.0);
        }
    }

    #[test]
    fn station_build_time_nonzero_for_all_kinds() {
        let kinds = [
            StationKind::MiningOutpost,
            StationKind::FuelDepot,
            StationKind::SensorStation,
        ];

        for kind in kinds {
            let time = station_build_time_seconds(kind);
            assert!(time > 0.0);
        }
    }

    #[test]
    fn station_fuel_capacity_positive_for_all_kinds() {
        let kinds = [
            StationKind::MiningOutpost,
            StationKind::FuelDepot,
            StationKind::SensorStation,
        ];

        for kind in kinds {
            let capacity = super::station_fuel_capacity(kind);
            assert!(capacity > 0.0);
        }
    }

    #[test]
    fn station_build_time_ordering_mine_longest() {
        let mine = station_build_time_seconds(StationKind::MiningOutpost);
        let depot = station_build_time_seconds(StationKind::FuelDepot);
        let sensor = station_build_time_seconds(StationKind::SensorStation);

        assert!(mine >= depot);
        assert!(mine >= sensor);
    }

    #[test]
    fn station_fuel_capacity_ordering_fuel_depot_max() {
        let mine = super::station_fuel_capacity(StationKind::MiningOutpost);
        let depot = super::station_fuel_capacity(StationKind::FuelDepot);
        let sensor = super::station_fuel_capacity(StationKind::SensorStation);

        assert!(depot >= mine);
        assert!(depot >= sensor);
    }

    #[test]
    fn station_fuel_burn_ordering_mine_highest() {
        let mine = super::station_fuel_burn_per_minute(StationKind::MiningOutpost);
        let depot = super::station_fuel_burn_per_minute(StationKind::FuelDepot);
        let sensor = super::station_fuel_burn_per_minute(StationKind::SensorStation);

        assert!(mine >= depot);
        assert!(mine >= sensor);
    }

    #[test]
    fn station_fuel_burn_ordering_depot_lowest() {
        let mine = super::station_fuel_burn_per_minute(StationKind::MiningOutpost);
        let depot = super::station_fuel_burn_per_minute(StationKind::FuelDepot);
        let sensor = super::station_fuel_burn_per_minute(StationKind::SensorStation);

        assert!(depot <= mine);
        assert!(depot <= sensor);
    }

    #[test]
    fn station_build_time_ordering_sensor_shortest() {
        let mine = station_build_time_seconds(StationKind::MiningOutpost);
        let depot = station_build_time_seconds(StationKind::FuelDepot);
        let sensor = station_build_time_seconds(StationKind::SensorStation);

        assert!(sensor <= mine);
        assert!(sensor <= depot);
    }

    #[test]
    fn station_fuel_capacity_ordering_mine_between_sensor_and_depot() {
        let mine = super::station_fuel_capacity(StationKind::MiningOutpost);
        let depot = super::station_fuel_capacity(StationKind::FuelDepot);
        let sensor = super::station_fuel_capacity(StationKind::SensorStation);

        assert!(mine <= sensor);
        assert!(mine <= depot);
    }

    #[test]
    fn station_fuel_capacity_sensor_exceeds_mine() {
        let mine = super::station_fuel_capacity(StationKind::MiningOutpost);
        let sensor = super::station_fuel_capacity(StationKind::SensorStation);
        assert!(sensor > mine);
    }

    #[test]
    fn station_build_time_depot_between_mine_and_sensor() {
        let mine = station_build_time_seconds(StationKind::MiningOutpost);
        let depot = station_build_time_seconds(StationKind::FuelDepot);
        let sensor = station_build_time_seconds(StationKind::SensorStation);

        assert!(depot <= mine);
        assert!(depot >= sensor);
    }

    #[test]
    fn station_build_time_values_strict_order() {
        let mine = station_build_time_seconds(StationKind::MiningOutpost);
        let depot = station_build_time_seconds(StationKind::FuelDepot);
        let sensor = station_build_time_seconds(StationKind::SensorStation);

        assert!(mine > depot);
        assert!(depot > sensor);
    }

    #[test]
    fn station_fuel_capacity_values_strict_order() {
        let mine = super::station_fuel_capacity(StationKind::MiningOutpost);
        let sensor = super::station_fuel_capacity(StationKind::SensorStation);
        let depot = super::station_fuel_capacity(StationKind::FuelDepot);

        assert!(depot > sensor);
        assert!(sensor > mine);
    }

    #[test]
    fn station_fuel_capacity_mine_plus_sensor_less_than_depot() {
        let mine = super::station_fuel_capacity(StationKind::MiningOutpost);
        let sensor = super::station_fuel_capacity(StationKind::SensorStation);
        let depot = super::station_fuel_capacity(StationKind::FuelDepot);

        assert!(mine + sensor < depot);
    }

    #[test]
    fn station_fuel_capacity_depot_exceeds_mine() {
        let depot = super::station_fuel_capacity(StationKind::FuelDepot);
        let mine = super::station_fuel_capacity(StationKind::MiningOutpost);
        assert!(depot > mine);
    }

    #[test]
    fn station_build_time_mine_greater_than_depot() {
        let mine = station_build_time_seconds(StationKind::MiningOutpost);
        let depot = station_build_time_seconds(StationKind::FuelDepot);
        assert!(mine > depot);
    }

    #[test]
    fn station_fuel_capacity_sensor_less_than_depot() {
        let sensor = super::station_fuel_capacity(StationKind::SensorStation);
        let depot = super::station_fuel_capacity(StationKind::FuelDepot);
        assert!(sensor < depot);
    }

    #[test]
    fn station_fuel_capacity_depot_exceeds_sensor_by_constant() {
        let sensor = super::station_fuel_capacity(StationKind::SensorStation);
        let depot = super::station_fuel_capacity(StationKind::FuelDepot);

        assert_eq!(depot - sensor, 80.0);
    }

    #[test]
    fn station_build_time_sensor_less_than_depot() {
        let sensor = station_build_time_seconds(StationKind::SensorStation);
        let depot = station_build_time_seconds(StationKind::FuelDepot);
        assert!(sensor < depot);
    }

    #[test]
    fn station_build_time_sensor_is_two_thirds_of_depot() {
        let sensor = station_build_time_seconds(StationKind::SensorStation);
        let depot = station_build_time_seconds(StationKind::FuelDepot);

        assert_eq!(sensor * 3.0, depot * 2.0);
    }

    #[test]
    fn station_fuel_burn_values_strict_order() {
        let mine = super::station_fuel_burn_per_minute(StationKind::MiningOutpost);
        let sensor = super::station_fuel_burn_per_minute(StationKind::SensorStation);
        let depot = super::station_fuel_burn_per_minute(StationKind::FuelDepot);

        assert!(mine > sensor);
        assert!(sensor > depot);
    }

    #[test]
    fn station_fuel_burn_sensor_minus_depot_delta() {
        let sensor = super::station_fuel_burn_per_minute(StationKind::SensorStation);
        let depot = super::station_fuel_burn_per_minute(StationKind::FuelDepot);

        let delta = sensor - depot;
        assert!((delta - 0.15).abs() < 1e-6);
    }

    #[test]
    fn station_fuel_burn_depot_is_half_sensor() {
        let sensor = super::station_fuel_burn_per_minute(StationKind::SensorStation);
        let depot = super::station_fuel_burn_per_minute(StationKind::FuelDepot);

        let result = depot * 1.5;
        assert!((result - sensor).abs() < 1e-6);
    }

    #[test]
    fn station_fuel_burn_depot_less_than_sensor() {
        let depot = super::station_fuel_burn_per_minute(StationKind::FuelDepot);
        let sensor = super::station_fuel_burn_per_minute(StationKind::SensorStation);
        assert!(depot < sensor);
    }

    #[test]
    fn station_fuel_burn_mine_greater_than_depot() {
        let mine = super::station_fuel_burn_per_minute(StationKind::MiningOutpost);
        let depot = super::station_fuel_burn_per_minute(StationKind::FuelDepot);
        assert!(mine > depot);
    }

    #[test]
    fn station_fuel_burn_sensor_between_mine_and_depot() {
        let mine = super::station_fuel_burn_per_minute(StationKind::MiningOutpost);
        let sensor = super::station_fuel_burn_per_minute(StationKind::SensorStation);
        let depot = super::station_fuel_burn_per_minute(StationKind::FuelDepot);

        assert!(sensor < mine);
        assert!(sensor > depot);
    }

    #[test]
    fn station_ore_capacity_mine_is_positive() {
        let capacity = super::station_ore_capacity(StationKind::MiningOutpost);
        assert!(capacity > 0.0);
    }

    #[test]
    fn station_ore_capacity_depot_is_zero() {
        let capacity = super::station_ore_capacity(StationKind::FuelDepot);
        assert_eq!(capacity, 0.0);
    }

    #[test]
    fn station_ore_capacity_sensor_is_zero() {
        let capacity = super::station_ore_capacity(StationKind::SensorStation);
        assert_eq!(capacity, 0.0);
    }

    #[test]
    fn station_ore_production_mine_is_positive() {
        let rate = super::station_ore_production_per_minute(StationKind::MiningOutpost);
        assert!(rate > 0.0);
    }

    #[test]
    fn station_ore_production_depot_is_zero() {
        let rate = super::station_ore_production_per_minute(StationKind::FuelDepot);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn station_ore_production_sensor_is_zero() {
        let rate = super::station_ore_production_per_minute(StationKind::SensorStation);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn station_ore_capacity_values() {
        assert_eq!(
            super::station_ore_capacity(StationKind::MiningOutpost),
            80.0
        );
        assert_eq!(super::station_ore_capacity(StationKind::FuelDepot), 0.0);
        assert_eq!(super::station_ore_capacity(StationKind::SensorStation), 0.0);
    }

    #[test]
    fn station_ore_production_values() {
        assert_eq!(
            super::station_ore_production_per_minute(StationKind::MiningOutpost),
            3.5
        );
        assert_eq!(
            super::station_ore_production_per_minute(StationKind::FuelDepot),
            0.0
        );
        assert_eq!(
            super::station_ore_production_per_minute(StationKind::SensorStation),
            0.0
        );
    }

    // =============================================================================
    // Shipyard and Refinery tests (TDD - write failing tests first)
    // =============================================================================

    #[test]
    fn shipyard_build_time_is_240_seconds() {
        assert_eq!(station_build_time_seconds(StationKind::Shipyard), 240.0);
    }

    #[test]
    fn refinery_build_time_is_200_seconds() {
        assert_eq!(station_build_time_seconds(StationKind::Refinery), 200.0);
    }

    #[test]
    fn shipyard_fuel_capacity_is_50() {
        assert_eq!(super::station_fuel_capacity(StationKind::Shipyard), 50.0);
    }

    #[test]
    fn refinery_fuel_capacity_is_60() {
        assert_eq!(super::station_fuel_capacity(StationKind::Refinery), 60.0);
    }

    #[test]
    fn shipyard_fuel_burn_per_minute() {
        assert_eq!(
            super::station_fuel_burn_per_minute(StationKind::Shipyard),
            0.5
        );
    }

    #[test]
    fn refinery_fuel_burn_per_minute() {
        assert_eq!(
            super::station_fuel_burn_per_minute(StationKind::Refinery),
            0.4
        );
    }

    #[test]
    fn shipyard_ore_capacity_is_100() {
        assert_eq!(super::station_ore_capacity(StationKind::Shipyard), 100.0);
    }

    #[test]
    fn refinery_ore_capacity_is_80() {
        assert_eq!(super::station_ore_capacity(StationKind::Refinery), 80.0);
    }

    #[test]
    fn shipyard_no_ore_production() {
        assert_eq!(
            super::station_ore_production_per_minute(StationKind::Shipyard),
            0.0
        );
    }

    #[test]
    fn refinery_no_ore_production() {
        assert_eq!(
            super::station_ore_production_per_minute(StationKind::Refinery),
            0.0
        );
    }

    // =============================================================================
    // Job model tests
    // =============================================================================

    #[test]
    fn refinery_job_new_calculates_fuel_output() {
        let job = super::RefineryJob::new(20);
        assert_eq!(job.ore_in, 20);
        assert!((job.fuel_out - 10.0).abs() < f32::EPSILON);
        assert!((job.remaining_seconds - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn refinery_job_tick_decreases_time() {
        let mut job = super::RefineryJob::new(10);
        let complete = job.tick(30.0);
        assert!(!complete);
        assert!((job.remaining_seconds - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn refinery_job_tick_returns_true_when_complete() {
        let mut job = super::RefineryJob::new(10);
        let complete = job.tick(60.0);
        assert!(complete);
    }

    #[test]
    fn shipyard_job_new_has_correct_values() {
        let job = super::ShipyardJob::new();
        assert_eq!(job.ore_in, 30);
        assert!((job.fuel_in - 15.0).abs() < f32::EPSILON);
        assert!((job.remaining_seconds - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn shipyard_job_tick_decreases_time() {
        let mut job = super::ShipyardJob::new();
        let complete = job.tick(60.0);
        assert!(!complete);
        assert!((job.remaining_seconds - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn shipyard_job_tick_returns_true_when_complete() {
        let mut job = super::ShipyardJob::new();
        let complete = job.tick(120.0);
        assert!(complete);
    }

    #[test]
    fn shipyard_job_default_same_as_new() {
        let job1 = super::ShipyardJob::new();
        let job2 = super::ShipyardJob::default();
        assert_eq!(job1.ore_in, job2.ore_in);
        assert!((job1.fuel_in - job2.fuel_in).abs() < f32::EPSILON);
    }

    // =============================================================================
    // Storage tests
    // =============================================================================

    #[test]
    fn refinery_storage_new_has_correct_capacity() {
        let storage = super::RefineryStorage::new();
        assert!((storage.fuel_capacity - 50.0).abs() < f32::EPSILON);
        assert!(storage.fuel.abs() < f32::EPSILON);
    }

    #[test]
    fn refinery_storage_add_fuel_respects_capacity() {
        let mut storage = super::RefineryStorage::new();
        let added = storage.add_fuel(30.0);
        assert!((added - 30.0).abs() < f32::EPSILON);
        assert!((storage.fuel - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn refinery_storage_add_fuel_cannot_exceed_capacity() {
        let mut storage = super::RefineryStorage::new();
        storage.add_fuel(40.0);
        let added = storage.add_fuel(20.0);
        assert!((added - 10.0).abs() < f32::EPSILON);
        assert!((storage.fuel - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn refinery_storage_remove_fuel() {
        let mut storage = super::RefineryStorage::new();
        storage.add_fuel(30.0);
        let removed = storage.remove_fuel(20.0);
        assert!((removed - 20.0).abs() < f32::EPSILON);
        assert!((storage.fuel - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn refinery_storage_free_space() {
        let mut storage = super::RefineryStorage::new();
        storage.add_fuel(30.0);
        assert!((storage.free_space() - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn shipyard_storage_new_has_correct_capacity() {
        let storage = super::ShipyardStorage::new();
        assert_eq!(storage.capacity, 3);
        assert_eq!(storage.ready_scouts, 0);
    }

    #[test]
    fn shipyard_storage_add_scout() {
        let mut storage = super::ShipyardStorage::new();
        assert!(storage.add_scout());
        assert_eq!(storage.ready_scouts, 1);
    }

    #[test]
    fn shipyard_storage_add_scout_respects_capacity() {
        let mut storage = super::ShipyardStorage::new();
        assert!(storage.add_scout());
        assert!(storage.add_scout());
        assert!(storage.add_scout());
        assert!(!storage.add_scout()); // Full
        assert_eq!(storage.ready_scouts, 3);
    }

    #[test]
    fn shipyard_storage_remove_scout() {
        let mut storage = super::ShipyardStorage::new();
        storage.add_scout();
        storage.add_scout();
        assert!(storage.remove_scout());
        assert_eq!(storage.ready_scouts, 1);
    }

    #[test]
    fn shipyard_storage_remove_scout_empty() {
        let mut storage = super::ShipyardStorage::new();
        assert!(!storage.remove_scout());
    }

    #[test]
    fn shipyard_storage_is_full() {
        let mut storage = super::ShipyardStorage::new();
        assert!(!storage.is_full());
        storage.add_scout();
        storage.add_scout();
        storage.add_scout();
        assert!(storage.is_full());
    }

    #[test]
    fn shipyard_storage_free_slots() {
        let mut storage = super::ShipyardStorage::new();
        assert_eq!(storage.free_slots(), 3);
        storage.add_scout();
        assert_eq!(storage.free_slots(), 2);
    }

    // =============================================================================
    // Outpost tests (NPC station)
    // =============================================================================

    #[test]
    fn outpost_has_zero_build_time() {
        assert_eq!(station_build_time_seconds(StationKind::Outpost), 0.0);
    }

    #[test]
    fn outpost_has_zero_fuel_capacity() {
        assert_eq!(super::station_fuel_capacity(StationKind::Outpost), 0.0);
    }

    #[test]
    fn outpost_has_zero_fuel_burn() {
        assert_eq!(
            super::station_fuel_burn_per_minute(StationKind::Outpost),
            0.0
        );
    }

    #[test]
    fn outpost_has_zero_ore_capacity() {
        assert_eq!(super::station_ore_capacity(StationKind::Outpost), 0.0);
    }

    #[test]
    fn outpost_has_zero_ore_production() {
        assert_eq!(
            super::station_ore_production_per_minute(StationKind::Outpost),
            0.0
        );
    }

    #[test]
    fn outpost_is_npc_station() {
        assert!(super::is_npc_station(StationKind::Outpost));
    }

    #[test]
    fn player_stations_are_not_npc() {
        assert!(!super::is_npc_station(StationKind::MiningOutpost));
        assert!(!super::is_npc_station(StationKind::FuelDepot));
        assert!(!super::is_npc_station(StationKind::SensorStation));
        assert!(!super::is_npc_station(StationKind::Shipyard));
        assert!(!super::is_npc_station(StationKind::Refinery));
    }

    // =============================================================================
    // Outpost trading tests
    // =============================================================================

    #[test]
    fn outpost_buy_fuel_has_three_options() {
        assert_eq!(super::OUTPOST_BUY_FUEL_OPTIONS.len(), 3);
    }

    #[test]
    fn outpost_buy_fuel_first_option_is_10_fuel_5_credits() {
        let option = super::OUTPOST_BUY_FUEL_OPTIONS[0];
        assert_eq!(option.fuel_amount, 10);
        assert_eq!(option.credit_cost, 5);
    }

    #[test]
    fn outpost_sell_ore_has_two_options() {
        assert_eq!(super::OUTPOST_SELL_ORE_OPTIONS.len(), 2);
    }

    #[test]
    fn outpost_sell_ore_first_option_is_5_ore_10_credits() {
        let option = super::OUTPOST_SELL_ORE_OPTIONS[0];
        assert_eq!(option.ore_amount, 5);
        assert_eq!(option.credit_reward, 10);
    }

    #[test]
    fn outpost_ore_to_credits_rate_is_2x() {
        assert_eq!(super::outpost_ore_to_credits(5), 10);
        assert_eq!(super::outpost_ore_to_credits(10), 20);
        assert_eq!(super::outpost_ore_to_credits(23), 46);
    }
}
