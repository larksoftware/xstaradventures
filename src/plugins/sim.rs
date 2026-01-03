use bevy::prelude::*;

use crate::compat::SpatialBundle;

use crate::fleets::{next_risk, RiskTolerance, ScoutBehavior, ScoutPhase};
use crate::ore::{OreKind, OreNode};
use crate::pirates::{schedule_next_launch, PirateBase, PirateShip};
use crate::plugins::core::{EventLog, InputBindings};
use crate::plugins::core::{FogConfig, SimConfig};
use crate::plugins::player::PlayerControl;
use crate::ships::{ship_fuel_burn_per_minute, Ship, ShipFuelAlert, ShipState};
use crate::stations::{
    station_fuel_burn_per_minute, station_ore_production_per_minute, CrisisStage, CrisisType,
    Station, StationBuild, StationCrisis, StationCrisisLog, StationProduction, StationState,
};
use crate::world::{
    zone_modifier_effect, JumpGate, KnowledgeLayer, Sector, SystemIntel, SystemNode, ZoneId,
    ZoneModifier, JUMP_TRANSITION_SECONDS,
};

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimTickCount>()
            .init_resource::<RevealedNodesTracker>()
            .add_systems(
                FixedUpdate,
                (
                    tick_simulation,
                    decay_intel,
                    station_fuel_burn,
                    station_ore_production,
                    station_build_progress,
                    station_crisis_stub,
                    station_lifecycle,
                    log_station_crisis_changes,
                    scout_behavior,
                    spawn_ore_at_revealed_nodes,
                    check_boundary_warnings,
                    pirate_launches,
                    pirate_move,
                    pirate_harassment,
                    ship_fuel_burn,
                    ship_fuel_alerts,
                    ship_state_stub,
                )
                    .run_if(sim_not_paused),
            )
            .add_systems(Update, handle_scout_risk_input);
    }
}

#[derive(Resource, Default)]
pub struct SimTickCount {
    pub tick: u64,
}

#[derive(Resource, Default)]
struct RevealedNodesTracker {
    spawned: std::collections::HashSet<u32>,
}

#[derive(Component, Default)]
pub struct BoundaryWarningState {
    last_level: BoundaryWarningLevel,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum BoundaryWarningLevel {
    #[default]
    Safe,
    SoftWarning,
    DangerZone,
}

const BOUNDARY_SOFT_WARNING: f32 = 1200.0;
const BOUNDARY_DANGER_ZONE: f32 = 2200.0;

fn tick_simulation(mut counter: ResMut<SimTickCount>, sector: Res<Sector>) {
    counter.tick = counter.tick.saturating_add(1);

    if counter.tick.is_multiple_of(10) {
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

fn station_ore_production(
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

fn station_build_progress(
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

fn station_lifecycle(
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

fn ship_fuel_burn(time: Res<Time<Fixed>>, mut ships: Query<&mut Ship>) {
    let delta_seconds = time.delta_secs();
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

fn ship_fuel_alerts(mut log: ResMut<EventLog>, mut alerts: Query<(&Ship, &mut ShipFuelAlert)>) {
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

fn pirate_launches(
    ticks: Res<SimTickCount>,
    mut commands: Commands,
    mut bases: Query<(&Transform, &mut PirateBase)>,
) {
    for (transform, mut base) in bases.iter_mut() {
        if ticks.tick < base.next_launch_tick {
            continue;
        }

        base.next_launch_tick = schedule_next_launch(ticks.tick, base.launch_interval_ticks);
        commands.spawn((
            PirateShip { speed: 70.0 },
            Name::new("Pirate-Ship"),
            SpatialBundle::from_transform(*transform),
        ));
    }
}

fn pirate_move(
    time: Res<Time<Fixed>>,
    stations: Query<&Transform, (With<Station>, Without<PirateShip>)>,
    mut pirates: Query<(&mut Transform, &PirateShip)>,
) {
    if stations.is_empty() {
        return;
    }

    let mut station_positions = Vec::new();
    for transform in stations.iter() {
        station_positions.push(Vec2::new(transform.translation.x, transform.translation.y));
    }

    let delta_seconds = time.delta_secs();

    for (mut transform, pirate) in pirates.iter_mut() {
        let pirate_pos = Vec2::new(transform.translation.x, transform.translation.y);
        let mut target = station_positions[0];
        let mut best_dist = pirate_pos.distance(target);

        for pos in &station_positions[1..] {
            let dist = pirate_pos.distance(*pos);
            if dist < best_dist {
                best_dist = dist;
                target = *pos;
            }
        }

        let direction = (target - pirate_pos).normalize_or_zero();
        let step = direction * pirate.speed * delta_seconds;
        transform.translation.x += step.x;
        transform.translation.y += step.y;
    }
}

fn pirate_harassment(
    mut commands: Commands,
    stations: Query<(Entity, &Transform), With<Station>>,
    pirates: Query<&Transform, With<PirateShip>>,
    crises: Query<Option<&StationCrisis>>,
) {
    let range = 18.0;

    for (station_entity, station_transform) in stations.iter() {
        let station_pos = Vec2::new(
            station_transform.translation.x,
            station_transform.translation.y,
        );
        let mut count = 0u32;

        for pirate_transform in pirates.iter() {
            let pirate_pos = Vec2::new(
                pirate_transform.translation.x,
                pirate_transform.translation.y,
            );
            if pirate_pos.distance(station_pos) <= range {
                count += 1;
            }
        }

        if count > 0 {
            let stage = if count >= 2 {
                CrisisStage::Failing
            } else {
                CrisisStage::Strained
            };

            commands.entity(station_entity).insert(StationCrisis {
                crisis_type: CrisisType::PirateHarassment,
                stage,
            });
        } else if let Ok(Some(existing)) = crises.get(station_entity) {
            if matches!(existing.crisis_type, CrisisType::PirateHarassment) {
                commands.entity(station_entity).remove::<StationCrisis>();
            }
        }
    }
}

fn log_station_crisis_changes(
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

fn crisis_changed(
    previous_type: Option<CrisisType>,
    previous_stage: Option<CrisisStage>,
    current_type: Option<CrisisType>,
    current_stage: Option<CrisisStage>,
) -> bool {
    previous_type != current_type || previous_stage != current_stage
}

fn handle_scout_risk_input(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut log: ResMut<EventLog>,
    mut scouts: Query<&mut ScoutBehavior>,
) {
    let delta = if input.just_pressed(bindings.scout_risk_down) {
        Some(-1)
    } else if input.just_pressed(bindings.scout_risk_up) {
        Some(1)
    } else {
        None
    };

    let delta = match delta {
        Some(value) => value,
        None => {
            return;
        }
    };

    let mut updated = None;
    for mut scout in scouts.iter_mut() {
        scout.risk = next_risk(scout.risk, delta);
        updated = Some(scout.risk);
    }

    if let Some(risk) = updated {
        let label = match risk {
            RiskTolerance::Cautious => "Cautious",
            RiskTolerance::Balanced => "Balanced",
            RiskTolerance::Bold => "Bold",
        };
        log.push(format!("Scout risk set to {}", label));
    }
}

const SCOUT_SPEED: f32 = 80.0;
const SCOUT_GATE_RANGE: f32 = 25.0;

#[allow(clippy::type_complexity)]
fn scout_behavior(
    time: Res<Time<Fixed>>,
    ticks: Res<SimTickCount>,
    mut log: ResMut<EventLog>,
    mut scouts: Query<(
        Entity,
        &mut Ship,
        &mut Transform,
        &mut ScoutBehavior,
        &mut ZoneId,
    )>,
    gates: Query<(Entity, &Transform, &JumpGate, &ZoneId), Without<ScoutBehavior>>,
    mut intel_query: Query<(&SystemNode, &mut SystemIntel)>,
) {
    let delta_seconds = time.delta_secs();

    for (_scout_entity, mut ship, mut transform, mut behavior, mut zone_id) in scouts.iter_mut() {
        if matches!(ship.state, ShipState::Disabled) {
            continue;
        }

        match behavior.phase {
            ScoutPhase::Scanning => {
                // Phase 1: Scan current zone
                scout_scan_zone(
                    &mut behavior,
                    &mut ship,
                    &zone_id,
                    &gates,
                    &mut intel_query,
                    ticks.tick,
                    &mut log,
                );
            }
            ScoutPhase::TravelingToGate => {
                // Phase 2: Travel to gate
                scout_travel_to_gate(
                    &mut behavior,
                    &mut ship,
                    &mut transform,
                    &gates,
                    delta_seconds,
                );
            }
            ScoutPhase::Jumping => {
                // Phase 3: Process jump transition
                scout_process_jump(
                    &mut behavior,
                    &mut ship,
                    &mut zone_id,
                    delta_seconds,
                    &mut log,
                );
            }
            ScoutPhase::Complete => {
                ship.state = ShipState::Idle;
            }
        }
    }
}

fn scout_scan_zone(
    behavior: &mut ScoutBehavior,
    ship: &mut Ship,
    zone_id: &ZoneId,
    gates: &Query<(Entity, &Transform, &JumpGate, &ZoneId), Without<ScoutBehavior>>,
    intel_query: &mut Query<(&SystemNode, &mut SystemIntel)>,
    tick: u64,
    log: &mut EventLog,
) {
    // Mark current zone as visited
    behavior.mark_zone_visited(zone_id.0);

    // Reveal intel for current zone
    for (node, mut intel) in intel_query.iter_mut() {
        if node.id == zone_id.0 && !intel.revealed {
            intel.revealed = true;
            intel.confidence = 0.8;
            intel.last_seen_tick = tick;
            intel.revealed_tick = tick;
            if matches!(intel.layer, KnowledgeLayer::Existence) {
                intel.layer = KnowledgeLayer::Geography;
            }
            log.push(format!("Scout reported zone {}", zone_id.0));
        }
    }

    // Discover all gates in current zone
    for (gate_entity, _gate_transform, gate, gate_zone) in gates.iter() {
        if gate_zone.0 == zone_id.0 {
            behavior.discover_gate(gate_entity, gate.destination_zone);
        }
    }

    // Decide next action
    if let Some((gate_entity, _dest_zone)) = behavior.next_gate_to_explore() {
        // Find the gate's position
        for (entity, gate_transform, _gate, _gate_zone) in gates.iter() {
            if entity == gate_entity {
                behavior.target_gate = Some(gate_entity);
                behavior.target_position = Some(Vec2::new(
                    gate_transform.translation.x,
                    gate_transform.translation.y,
                ));
                behavior.phase = ScoutPhase::TravelingToGate;
                ship.state = ShipState::InTransit;
                return;
            }
        }
    }

    // No more gates to explore - exploration complete
    behavior.phase = ScoutPhase::Complete;
    ship.state = ShipState::Idle;
}

fn scout_travel_to_gate(
    behavior: &mut ScoutBehavior,
    ship: &mut Ship,
    transform: &mut Transform,
    gates: &Query<(Entity, &Transform, &JumpGate, &ZoneId), Without<ScoutBehavior>>,
    delta_seconds: f32,
) {
    let target_pos = match behavior.target_position {
        Some(pos) => pos,
        None => {
            // Lost target, go back to scanning
            behavior.phase = ScoutPhase::Scanning;
            behavior.target_gate = None;
            return;
        }
    };

    let current_pos = Vec2::new(transform.translation.x, transform.translation.y);
    let to_target = target_pos - current_pos;
    let distance = to_target.length();

    if distance <= SCOUT_GATE_RANGE {
        // Arrived at gate - start jump
        if let Some(target_gate) = behavior.target_gate {
            for (entity, _gate_transform, gate, _gate_zone) in gates.iter() {
                if entity == target_gate {
                    behavior.start_jump(gate.destination_zone, JUMP_TRANSITION_SECONDS);
                    behavior.remove_gate(target_gate);
                    ship.state = ShipState::Executing;
                    return;
                }
            }
        }
        // Gate not found, go back to scanning
        behavior.phase = ScoutPhase::Scanning;
        behavior.target_gate = None;
        behavior.target_position = None;
    } else {
        // Move toward gate
        let direction = to_target.normalize_or_zero();
        let step = direction * SCOUT_SPEED * delta_seconds;
        transform.translation.x += step.x;
        transform.translation.y += step.y;
        ship.state = ShipState::InTransit;
    }
}

fn scout_process_jump(
    behavior: &mut ScoutBehavior,
    ship: &mut Ship,
    zone_id: &mut ZoneId,
    delta_seconds: f32,
    log: &mut EventLog,
) {
    behavior.jump_remaining_seconds -= delta_seconds;

    if behavior.jump_remaining_seconds <= 0.0 {
        let destination = behavior.jump_destination.unwrap_or(behavior.current_zone);
        behavior.complete_jump();
        // Update the actual ZoneId component
        zone_id.0 = behavior.current_zone;
        ship.state = ShipState::Executing;
        log.push(format!("Scout arrived at zone {}", destination));
    } else {
        ship.state = ShipState::InTransit;
    }
}

fn next_unit_ore_rng(state: &mut u64) -> f32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let value = (*state >> 33) as u32;
    (value as f32) / (u32::MAX as f32)
}

const ORE_MIN_RADIUS: f32 = 400.0;
const ORE_MAX_RADIUS: f32 = 800.0;

fn ore_count_for_zone(modifier: Option<ZoneModifier>, is_starter: bool, rng: &mut u64) -> usize {
    if is_starter {
        let rand_val = next_unit_ore_rng(rng);
        return 3 + (rand_val * 3.0) as usize;
    }

    match modifier {
        Some(ZoneModifier::RichOreVeins) => {
            let rand_val = next_unit_ore_rng(rng);
            20 + (rand_val * 11.0) as usize
        }
        Some(ZoneModifier::HighRadiation) => {
            let rand_val = next_unit_ore_rng(rng);
            (rand_val * 6.0) as usize
        }
        Some(ZoneModifier::NebulaInterference) => {
            let rand_val = next_unit_ore_rng(rng);
            8 + (rand_val * 8.0) as usize
        }
        Some(ZoneModifier::ClearSignals) => {
            let rand_val = next_unit_ore_rng(rng);
            10 + (rand_val * 6.0) as usize
        }
        None => {
            let rand_val = next_unit_ore_rng(rng);
            5 + (rand_val * 8.0) as usize
        }
    }
}

fn spawn_ore_at_revealed_nodes(
    mut commands: Commands,
    mut tracker: ResMut<RevealedNodesTracker>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
) {
    for (node, intel) in nodes.iter() {
        if intel.revealed && !tracker.spawned.contains(&node.id) {
            tracker.spawned.insert(node.id);

            let mut rng_state = node.id as u64;
            let is_starter = intel.revealed_tick == 0;
            let ore_count = ore_count_for_zone(node.modifier, is_starter, &mut rng_state);

            for index in 0..ore_count {
                let angle = next_unit_ore_rng(&mut rng_state) * std::f32::consts::TAU;
                let radius = ORE_MIN_RADIUS
                    + next_unit_ore_rng(&mut rng_state) * (ORE_MAX_RADIUS - ORE_MIN_RADIUS);

                let offset_x = angle.cos() * radius;
                let offset_y = angle.sin() * radius;

                let common_ore_count = (ore_count as f32 * 0.7) as usize;
                let kind = if index < common_ore_count {
                    OreKind::CommonOre
                } else {
                    OreKind::FuelOre
                };

                let capacity = 20.0 + (index as f32 * 6.0) + ((node.id as f32) * 0.01);
                let kind_str = match kind {
                    OreKind::CommonOre => "OreNode",
                    OreKind::FuelOre => "FuelNode",
                };

                commands.spawn((
                    OreNode {
                        kind,
                        remaining: capacity,
                        capacity,
                        rate_per_second: 3.0,
                    },
                    ZoneId(node.id),
                    Name::new(format!("{}-{}-{}", kind_str, node.id, index + 1)),
                    SpatialBundle::from_transform(Transform::from_xyz(
                        node.position.x + offset_x,
                        node.position.y + offset_y,
                        0.3,
                    )),
                ));
            }
        }
    }
}

fn check_boundary_warnings(
    mut log: ResMut<EventLog>,
    mut player_query: Query<(&Transform, &mut BoundaryWarningState), With<PlayerControl>>,
) {
    let (transform, mut warning_state) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => return,
    };

    let player_pos = Vec2::new(transform.translation.x, transform.translation.y);
    let distance_from_origin = player_pos.length();

    let current_level = if distance_from_origin >= BOUNDARY_DANGER_ZONE {
        BoundaryWarningLevel::DangerZone
    } else if distance_from_origin >= BOUNDARY_SOFT_WARNING {
        BoundaryWarningLevel::SoftWarning
    } else {
        BoundaryWarningLevel::Safe
    };

    if current_level != warning_state.last_level {
        match current_level {
            BoundaryWarningLevel::Safe => {
                // Don't log when returning to safe zone
            }
            BoundaryWarningLevel::SoftWarning => {
                log.push("Long-range sensors detect signal degradation. Consider returning to civilization.".to_string());
            }
            BoundaryWarningLevel::DangerZone => {
                log.push("WARNING: You are drifting into the void. Hull stress increasing. Fuel reserves critical. Turn back NOW.".to_string());
            }
        }
        warning_state.last_level = current_level;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ships::ShipKind;
    use crate::stations::StationKind;
    use bevy::ecs::system::SystemState;
    use std::time::Duration;

    #[test]
    fn advance_intel_layer_stops_at_stability() {
        let mut intel = SystemIntel {
            layer: KnowledgeLayer::Threats,
            confidence: 0.5,
            last_seen_tick: 0,
            revealed: false,
            revealed_tick: 0,
        };

        advance_intel_layer(&mut intel);
        assert_eq!(intel.layer, KnowledgeLayer::Stability);
        advance_intel_layer(&mut intel);
        assert_eq!(intel.layer, KnowledgeLayer::Stability);
    }

    #[test]
    fn refresh_intel_sets_confidence_and_tick() {
        let mut intel = SystemIntel {
            layer: KnowledgeLayer::Existence,
            confidence: 0.2,
            last_seen_tick: 5,
            revealed: false,
            revealed_tick: 0,
        };

        refresh_intel(&mut intel, 42);
        assert_eq!(intel.last_seen_tick, 42);
        assert_eq!(intel.confidence, 1.0);
    }

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
    fn ship_state_stub_disables_empty_fuel() {
        let mut world = World::default();
        world.spawn(Ship {
            kind: ShipKind::Scout,
            state: ShipState::Idle,
            fuel: 0.0,
            fuel_capacity: 30.0,
        });

        let mut system_state: SystemState<Query<&mut Ship>> = SystemState::new(&mut world);
        let ships = system_state.get_mut(&mut world);
        ship_state_stub(ships);
        system_state.apply(&mut world);

        let mut query = world.query::<&Ship>();
        for ship in query.iter(&world) {
            assert_eq!(ship.state, ShipState::Disabled);
        }
    }

    #[test]
    fn ship_fuel_alerts_logs_once_and_sets_flags() {
        let mut world = World::default();
        world.insert_resource(EventLog::default());
        world.spawn((
            Ship {
                kind: ShipKind::Scout,
                state: ShipState::Idle,
                fuel: 1.0,
                fuel_capacity: 20.0,
            },
            ShipFuelAlert::default(),
        ));

        let mut system_state: SystemState<(ResMut<EventLog>, Query<(&Ship, &mut ShipFuelAlert)>)> =
            SystemState::new(&mut world);
        {
            let (log, alerts) = system_state.get_mut(&mut world);
            ship_fuel_alerts(log, alerts);
        }
        system_state.apply(&mut world);

        let log = world.resource::<EventLog>();
        assert_eq!(log.entries().len(), 2);

        let mut system_state: SystemState<(ResMut<EventLog>, Query<(&Ship, &mut ShipFuelAlert)>)> =
            SystemState::new(&mut world);
        {
            let (log, alerts) = system_state.get_mut(&mut world);
            ship_fuel_alerts(log, alerts);
        }
        system_state.apply(&mut world);

        let log = world.resource::<EventLog>();
        assert_eq!(log.entries().len(), 2);

        let mut query = world.query::<&ShipFuelAlert>();
        for alert in query.iter(&world) {
            assert!(alert.low);
            assert!(alert.critical);
        }
    }

    #[test]
    fn zone_modifier_risk_empty_sector_is_zero() {
        let sector = Sector::default();
        let risk = zone_modifier_risk(&sector);
        assert_eq!(risk, 0.0);
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
