//! Station simulation systems.

use bevy::prelude::*;

use crate::plugins::core::EventLog;
use crate::stations::{
    station_fuel_burn_per_minute, station_ore_production_per_minute, CrisisStage, CrisisType,
    Station, StationBuild, StationCrisis, StationCrisisLog, StationProduction, StationState,
};

use super::SimTickCount;

// =============================================================================
// Systems
// =============================================================================

pub fn station_fuel_burn(time: Res<Time<Fixed>>, mut stations: Query<&mut Station>) {
    let delta_seconds = time.delta_secs();
    let minutes = delta_seconds / 60.0;

    for mut station in stations.iter_mut() {
        if matches!(station.state, StationState::Failed) {
            continue;
        }

        let burn = station_fuel_burn_per_minute(station.kind) * minutes;
        if station.fuel > burn {
            station.fuel -= burn;
        } else {
            station.fuel = 0.0;
        }
    }
}

pub fn station_ore_production(
    time: Res<Time<Fixed>>,
    mut stations: Query<(&Station, &mut StationProduction)>,
) {
    let delta_seconds = time.delta_secs();
    let minutes = delta_seconds / 60.0;

    for (station, mut production) in stations.iter_mut() {
        if !matches!(station.state, StationState::Operational) {
            continue;
        }

        let rate = station_ore_production_per_minute(station.kind);
        let produced = rate * minutes;
        let free_capacity = (production.ore_capacity - production.ore).max(0.0);
        let added = produced.min(free_capacity);

        if added > 0.0 {
            production.ore += added;
        }
    }
}

pub fn station_build_progress(
    time: Res<Time<Fixed>>,
    mut commands: Commands,
    mut stations: Query<(Entity, &mut Station, &mut StationBuild)>,
) {
    let delta_seconds = time.delta_secs();

    for (entity, mut station, mut build) in stations.iter_mut() {
        if build.remaining_seconds > delta_seconds {
            build.remaining_seconds -= delta_seconds;
        } else {
            build.remaining_seconds = 0.0;
            station.state = StationState::Operational;
            commands.entity(entity).remove::<StationBuild>();
        }
    }
}

pub fn station_lifecycle(
    ticks: Res<SimTickCount>,
    mut stations: Query<(&mut Station, Option<&StationBuild>, Option<&StationCrisis>)>,
) {
    let mut counts = std::collections::BTreeMap::new();

    for (mut station, build, crisis) in stations.iter_mut() {
        if matches!(station.state, StationState::Failed) {
            let entry = counts.entry("Failed").or_insert(0u32);
            *entry += 1;
            continue;
        }

        if build.is_some() {
            station.state = StationState::Deploying;
        } else if station.fuel <= 0.0 {
            station.state = StationState::Failed;
        } else if let Some(crisis) = crisis {
            station.state = match crisis.stage {
                CrisisStage::Failing => StationState::Failing,
                CrisisStage::Strained => StationState::Strained,
                CrisisStage::Stable | CrisisStage::Resolved => StationState::Operational,
            };
        } else {
            station.state = StationState::Operational;
        }

        let key = match station.state {
            StationState::Deploying => "Deploying",
            StationState::Operational => "Operational",
            StationState::Strained => "Strained",
            StationState::Failing => "Failing",
            StationState::Failed => "Failed",
        };
        let entry = counts.entry(key).or_insert(0u32);
        *entry += 1;
    }

    if ticks.tick.is_multiple_of(60) && !counts.is_empty() {
        let summary = counts
            .iter()
            .map(|(key, count)| format!("{}:{}", key, count))
            .collect::<Vec<_>>()
            .join(" ");
        info!("Stations: {}", summary);
    }
}

pub fn station_crisis_stub(
    mut commands: Commands,
    stations: Query<(Entity, &Station, Option<&StationCrisis>)>,
) {
    for (entity, station, crisis) in stations.iter() {
        if station.fuel_capacity <= 0.0 {
            continue;
        }

        let ratio = station.fuel / station.fuel_capacity;
        if ratio <= 0.25 {
            let stage = if ratio <= 0.10 {
                CrisisStage::Failing
            } else {
                CrisisStage::Strained
            };

            match crisis {
                Some(existing) => {
                    if existing.stage != stage || existing.crisis_type != CrisisType::FuelShortage {
                        commands.entity(entity).insert(StationCrisis {
                            crisis_type: CrisisType::FuelShortage,
                            stage,
                        });
                    }
                }
                None => {
                    commands.entity(entity).insert(StationCrisis {
                        crisis_type: CrisisType::FuelShortage,
                        stage,
                    });
                }
            }
        } else if let Some(existing) = crisis {
            if matches!(existing.crisis_type, CrisisType::FuelShortage) {
                commands.entity(entity).remove::<StationCrisis>();
            }
        }
    }
}

pub fn log_station_crisis_changes(
    mut log: ResMut<EventLog>,
    mut stations: Query<(&Station, Option<&StationCrisis>, &mut StationCrisisLog)>,
) {
    for (station, crisis, mut log_state) in stations.iter_mut() {
        let current_type = crisis.map(|crisis| crisis.crisis_type);
        let current_stage = crisis.map(|crisis| crisis.stage);

        if crisis_changed(
            log_state.last_type,
            log_state.last_stage,
            current_type,
            current_stage,
        ) {
            match (current_type, current_stage) {
                (Some(kind), Some(stage)) => {
                    log.push(format!(
                        "Station {:?} crisis: {:?} {:?}",
                        station.kind, kind, stage
                    ));
                }
                _ => {
                    log.push(format!("Station {:?} crisis resolved", station.kind));
                }
            }
            log_state.last_type = current_type;
            log_state.last_stage = current_stage;
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

pub fn crisis_changed(
    previous_type: Option<CrisisType>,
    previous_stage: Option<CrisisStage>,
    current_type: Option<CrisisType>,
    current_stage: Option<CrisisStage>,
) -> bool {
    previous_type != current_type || previous_stage != current_stage
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stations::StationKind;
    use bevy::ecs::system::SystemState;
    use std::time::Duration;

    #[test]
    fn station_fuel_burn_skips_failed_state() {
        let mut world = World::default();
        let mut time = Time::<Fixed>::from_duration(Duration::from_secs_f32(60.0));
        time.advance_by(Duration::from_secs_f32(60.0));
        world.insert_resource(time);
        world.spawn(Station {
            kind: StationKind::MiningOutpost,
            state: StationState::Failed,
            fuel: 10.0,
            fuel_capacity: 30.0,
        });

        let mut system_state: SystemState<(Res<Time<Fixed>>, Query<&mut Station>)> =
            SystemState::new(&mut world);
        let (time, stations) = system_state.get_mut(&mut world);
        station_fuel_burn(time, stations);
        system_state.apply(&mut world);

        let mut query = world.query::<&Station>();
        for station in query.iter(&world) {
            assert_eq!(station.fuel, 10.0);
        }
    }

    #[test]
    fn station_lifecycle_marks_failed_when_fuel_empty() {
        let mut world = World::default();
        world.insert_resource(SimTickCount { tick: 1 });
        world.spawn(Station {
            kind: StationKind::MiningOutpost,
            state: StationState::Operational,
            fuel: 0.0,
            fuel_capacity: 30.0,
        });

        let mut system_state: SystemState<(
            Res<SimTickCount>,
            Query<(&mut Station, Option<&StationBuild>, Option<&StationCrisis>)>,
        )> = SystemState::new(&mut world);
        let (ticks, stations) = system_state.get_mut(&mut world);
        station_lifecycle(ticks, stations);
        system_state.apply(&mut world);

        let mut query = world.query::<&Station>();
        for station in query.iter(&world) {
            assert_eq!(station.state, StationState::Failed);
        }
    }

    #[test]
    fn crisis_changed_detects_resolve() {
        let changed = crisis_changed(
            Some(CrisisType::FuelShortage),
            Some(CrisisStage::Strained),
            None,
            None,
        );
        assert!(changed);
    }

    #[test]
    fn station_ore_production_only_when_operational() {
        let mut world = World::default();
        let mut time = Time::<Fixed>::from_duration(Duration::from_secs_f32(60.0));
        time.advance_by(Duration::from_secs_f32(60.0));
        world.insert_resource(time);

        world.spawn((
            Station {
                kind: StationKind::MiningOutpost,
                state: StationState::Operational,
                fuel: 20.0,
                fuel_capacity: 30.0,
            },
            StationProduction {
                ore: 0.0,
                ore_capacity: 60.0,
            },
        ));

        world.spawn((
            Station {
                kind: StationKind::MiningOutpost,
                state: StationState::Deploying,
                fuel: 20.0,
                fuel_capacity: 30.0,
            },
            StationProduction {
                ore: 0.0,
                ore_capacity: 60.0,
            },
        ));

        let mut system_state: SystemState<(
            Res<Time<Fixed>>,
            Query<(&Station, &mut StationProduction)>,
        )> = SystemState::new(&mut world);
        let (time, stations) = system_state.get_mut(&mut world);
        station_ore_production(time, stations);
        system_state.apply(&mut world);

        let mut query = world.query::<(&Station, &StationProduction)>();
        let mut count = 0;
        for (station, production) in query.iter(&world) {
            count += 1;
            if matches!(station.state, StationState::Operational) {
                assert!(
                    production.ore > 0.0,
                    "Operational station should produce ore"
                );
            } else {
                assert_eq!(
                    production.ore, 0.0,
                    "Non-operational station should not produce ore"
                );
            }
        }
        assert_eq!(count, 2, "Should have spawned 2 stations");
    }

    #[test]
    fn station_ore_production_respects_capacity() {
        let mut world = World::default();
        let mut time = Time::<Fixed>::from_duration(Duration::from_secs_f32(60.0));
        time.advance_by(Duration::from_secs_f32(60.0));
        world.insert_resource(time);

        world.spawn((
            Station {
                kind: StationKind::MiningOutpost,
                state: StationState::Operational,
                fuel: 20.0,
                fuel_capacity: 30.0,
            },
            StationProduction {
                ore: 59.0,
                ore_capacity: 60.0,
            },
        ));

        let mut system_state: SystemState<(
            Res<Time<Fixed>>,
            Query<(&Station, &mut StationProduction)>,
        )> = SystemState::new(&mut world);
        let (time, stations) = system_state.get_mut(&mut world);
        station_ore_production(time, stations);
        system_state.apply(&mut world);

        let mut query = world.query::<&StationProduction>();
        for production in query.iter(&world) {
            assert!(production.ore <= production.ore_capacity);
        }
    }

    #[test]
    fn station_crisis_recovers_when_refueled() {
        let mut world = World::default();

        let entity = world
            .spawn((
                Station {
                    kind: StationKind::MiningOutpost,
                    state: StationState::Strained,
                    fuel: 5.0,
                    fuel_capacity: 30.0,
                },
                StationCrisis {
                    crisis_type: CrisisType::FuelShortage,
                    stage: CrisisStage::Strained,
                },
            ))
            .id();

        let mut system_state: SystemState<(
            Commands,
            Query<(Entity, &Station, Option<&StationCrisis>)>,
        )> = SystemState::new(&mut world);
        let (commands, stations) = system_state.get_mut(&mut world);
        station_crisis_stub(commands, stations);
        system_state.apply(&mut world);

        let has_crisis = world.get::<StationCrisis>(entity).is_some();
        assert!(has_crisis, "Station should still have crisis with low fuel");

        world.get_mut::<Station>(entity).unwrap().fuel = 20.0;

        let mut system_state: SystemState<(
            Commands,
            Query<(Entity, &Station, Option<&StationCrisis>)>,
        )> = SystemState::new(&mut world);
        let (commands, stations) = system_state.get_mut(&mut world);
        station_crisis_stub(commands, stations);
        system_state.apply(&mut world);

        let has_crisis = world.get::<StationCrisis>(entity).is_some();
        assert!(!has_crisis, "Station should recover after refueling");
    }

    #[test]
    fn log_station_crisis_changes_logs_new_crisis() {
        let mut world = World::default();
        world.insert_resource(EventLog::default());

        world.spawn((
            Station {
                kind: StationKind::MiningOutpost,
                state: StationState::Strained,
                fuel: 5.0,
                fuel_capacity: 30.0,
            },
            StationCrisis {
                crisis_type: CrisisType::FuelShortage,
                stage: CrisisStage::Strained,
            },
            StationCrisisLog::default(),
        ));

        let mut system_state: SystemState<(
            ResMut<EventLog>,
            Query<(&Station, Option<&StationCrisis>, &mut StationCrisisLog)>,
        )> = SystemState::new(&mut world);
        let (log, stations) = system_state.get_mut(&mut world);
        log_station_crisis_changes(log, stations);
        system_state.apply(&mut world);

        let log = world.resource::<EventLog>();
        assert_eq!(log.entries().len(), 1);
        assert!(log.entries()[0].contains("crisis"));
    }
}
