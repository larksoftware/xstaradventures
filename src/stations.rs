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

#[cfg(test)]
mod tests {
    use super::{station_build_time_seconds, StationKind};

    #[test]
    fn station_build_time_values() {
        assert_eq!(
            station_build_time_seconds(StationKind::MiningOutpost),
            240.0
        );
        assert_eq!(station_build_time_seconds(StationKind::FuelDepot), 180.0);
        assert_eq!(
            station_build_time_seconds(StationKind::SensorStation),
            120.0
        );
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
            1.0
        );
        assert_eq!(
            super::station_fuel_burn_per_minute(StationKind::FuelDepot),
            0.5
        );
        assert_eq!(
            super::station_fuel_burn_per_minute(StationKind::SensorStation),
            0.75
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

        assert_eq!(sensor - depot, 0.25);
    }

    #[test]
    fn station_fuel_burn_depot_is_half_sensor() {
        let sensor = super::station_fuel_burn_per_minute(StationKind::SensorStation);
        let depot = super::station_fuel_burn_per_minute(StationKind::FuelDepot);

        assert_eq!(depot * 1.5, sensor);
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
}
