use bevy::prelude::*;

use crate::plugins::core::{FogConfig, SimConfig};
use crate::plugins::core::EventLog;
use crate::ships::{ship_fuel_burn_per_minute, Ship, ShipFuelAlert, ShipState};
use crate::stations::{
    CrisisStage, CrisisType, Station, StationBuild, StationCrisis, StationState,
    station_fuel_burn_per_minute,
};
use crate::world::{zone_modifier_effect, KnowledgeLayer, Sector, SystemIntel};

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimTickCount>()
            .add_systems(
                FixedUpdate,
                (
                    tick_simulation,
                    decay_intel,
                    station_fuel_burn,
                    station_build_progress,
                    station_crisis_stub,
                    station_lifecycle_stub,
                    ship_fuel_burn,
                    ship_fuel_alerts,
                    ship_state_stub,
                )
                    .run_if(sim_not_paused),
            );
    }
}

#[derive(Resource, Default)]
pub struct SimTickCount {
    pub tick: u64,
}

fn tick_simulation(mut counter: ResMut<SimTickCount>, sector: Res<Sector>) {
    counter.tick = counter.tick.saturating_add(1);

    if counter.tick % 10 == 0 {
        let total_distance = sector
            .routes
            .iter()
            .map(|route| route.distance)
            .sum::<f32>();

        let endpoint_sum = sector
            .routes
            .iter()
            .map(|route| route.from + route.to)
            .sum::<u32>();

        let average_risk = if sector.routes.is_empty() {
            0.0
        } else {
            let total_risk = sector.routes.iter().map(|route| route.risk).sum::<f32>();
            total_risk / (sector.routes.len() as f32)
        };

        let modifier_risk = zone_modifier_risk(&sector);

        info!(
            "Sim tick {} (nodes: {}, routes: {}, distance: {:.2}, endpoints: {}, risk: {:.2}, mod: {:.2})",
            counter.tick,
            sector.nodes.len(),
            sector.routes.len(),
            total_distance,
            endpoint_sum,
            average_risk,
            modifier_risk
        );
    }
}

fn sim_not_paused(config: Res<SimConfig>) -> bool {
    !config.paused
}

fn zone_modifier_risk(sector: &Sector) -> f32 {
    if sector.nodes.is_empty() {
        return 0.0;
    }

    let total = sector
        .nodes
        .iter()
        .map(|node| {
            let effect = zone_modifier_effect(node.modifier);
            effect.fuel_risk + effect.confidence_risk + effect.pirate_risk
        })
        .sum::<f32>();

    total / (sector.nodes.len() as f32)
}

fn decay_intel(
    ticks: Res<SimTickCount>,
    config: Res<FogConfig>,
    mut intel_query: Query<&mut SystemIntel>,
) {
    for mut intel in intel_query.iter_mut() {
        let age = ticks.tick.saturating_sub(intel.last_seen_tick);
        let base_decay = match intel.layer {
            KnowledgeLayer::Existence => config.decay_existence,
            KnowledgeLayer::Geography => config.decay_geography,
            KnowledgeLayer::Resources => config.decay_resources,
            KnowledgeLayer::Threats => config.decay_threats,
            KnowledgeLayer::Stability => config.decay_stability,
        };
        let age_factor = (age as f32 / 1000.0).clamp(0.0, 1.0);
        let decay = base_decay * (1.0 + age_factor);

        if intel.confidence > decay {
            intel.confidence -= decay;
        } else {
            intel.confidence = 0.0;
        }
    }
}

pub fn refresh_intel(intel: &mut SystemIntel, tick: u64) {
    intel.last_seen_tick = tick;
    intel.confidence = 1.0;
}

pub fn advance_intel_layer(intel: &mut SystemIntel) {
    intel.layer = match intel.layer {
        KnowledgeLayer::Existence => KnowledgeLayer::Geography,
        KnowledgeLayer::Geography => KnowledgeLayer::Resources,
        KnowledgeLayer::Resources => KnowledgeLayer::Threats,
        KnowledgeLayer::Threats => KnowledgeLayer::Stability,
        KnowledgeLayer::Stability => KnowledgeLayer::Stability,
    };
}

fn station_fuel_burn(time: Res<Time<Fixed>>, mut stations: Query<&mut Station>) {
    let delta_seconds = time.delta_seconds();
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

fn station_build_progress(
    time: Res<Time<Fixed>>,
    mut commands: Commands,
    mut stations: Query<(Entity, &mut Station, &mut StationBuild)>,
) {
    let delta_seconds = time.delta_seconds();

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

fn station_lifecycle_stub(ticks: Res<SimTickCount>, stations: Query<&Station>) {
    if ticks.tick % 60 != 0 {
        return;
    }

    let mut counts = std::collections::BTreeMap::new();
    for station in stations.iter() {
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

    if !counts.is_empty() {
        let summary = counts
            .iter()
            .map(|(key, count)| format!("{}:{}", key, count))
            .collect::<Vec<_>>()
            .join(" ");
        info!("Stations: {}", summary);
    }
}

fn station_crisis_stub(
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
                    if existing.stage != stage || existing.crisis_type != CrisisType::FuelShortage
                    {
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
        } else if crisis.is_some() {
            commands.entity(entity).remove::<StationCrisis>();
        }
    }
}

fn ship_fuel_burn(time: Res<Time<Fixed>>, mut ships: Query<&mut Ship>) {
    let delta_seconds = time.delta_seconds();
    let minutes = delta_seconds / 60.0;

    for mut ship in ships.iter_mut() {
        if matches!(ship.state, ShipState::Disabled) {
            continue;
        }

        let burn = ship_fuel_burn_per_minute(ship.kind) * minutes;
        if ship.fuel > burn {
            ship.fuel -= burn;
        } else {
            ship.fuel = 0.0;
        }
    }
}

fn ship_state_stub(mut ships: Query<&mut Ship>) {
    for mut ship in ships.iter_mut() {
        if ship.fuel <= 0.0 {
            ship.state = ShipState::Disabled;
            continue;
        }

        if ship.fuel_capacity > 0.0 {
            let ratio = ship.fuel / ship.fuel_capacity;
            if ratio <= 0.1 && !matches!(ship.state, ShipState::Returning) {
                ship.state = ShipState::Returning;
            }
        }
    }
}

fn ship_fuel_alerts(
    mut log: ResMut<EventLog>,
    mut alerts: Query<(&Ship, &mut ShipFuelAlert)>,
) {
    for (ship, mut alert) in alerts.iter_mut() {
        if ship.fuel_capacity <= 0.0 {
            continue;
        }

        let ratio = ship.fuel / ship.fuel_capacity;
        let low = ratio <= 0.25;
        let critical = ratio <= 0.10;

        if low && !alert.low {
            log.push(format!("Ship {:?} low fuel", ship.kind));
            alert.low = true;
        }

        if critical && !alert.critical {
            log.push(format!("Ship {:?} critical fuel", ship.kind));
            alert.critical = true;
        }

        if !low {
            alert.low = false;
        }

        if !critical {
            alert.critical = false;
        }
    }
}
