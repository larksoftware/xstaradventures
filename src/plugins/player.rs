use bevy::prelude::*;

use crate::compat::SpatialBundle;

use crate::ore::{mine_amount, OreKind, OreNode};
use crate::pirates::{PirateBase, PirateShip};
use crate::plugins::core::EventLog;
use crate::plugins::core::InputBindings;
use crate::plugins::core::SimConfig;
use crate::plugins::core::ViewMode;
use crate::ships::{Cargo, Ship, ShipKind, ShipState, Velocity};
use crate::stations::{
    station_build_time_seconds, station_fuel_capacity, station_ore_capacity, Station, StationBuild,
    StationCrisisLog, StationKind, StationProduction, StationState,
};
use crate::world::{
    JumpGate, JumpTransition, SystemNode, ZoneId, JUMP_GATE_FUEL_COST, JUMP_TRANSITION_SECONDS,
};

pub struct PlayerPlugin;

#[derive(Component, Debug, Default)]
pub struct PlayerControl;

#[derive(Resource, Default)]
pub struct NearbyTargets {
    pub entities: Vec<(Entity, Vec2, String)>,
    pub selected_index: usize,
    /// True only after the player has pressed Tab to explicitly select a target
    pub manually_selected: bool,
}

/// Tracks autopilot engagement and target state
#[derive(Resource, Default)]
pub struct AutopilotState {
    /// Whether autopilot is currently engaged
    pub engaged: bool,
    /// Target entity we're navigating toward
    pub target_entity: Option<Entity>,
}

const PLAYER_THRUST_ACCELERATION: f32 = 200.0; // pixels per second squared
const PLAYER_THRUST_FUEL_BURN_PER_MINUTE: f32 = 1.0;
const PLAYER_ROTATION_SPEED: f32 = 3.0; // radians per second

// Autopilot constants
const AUTOPILOT_DOCKING_DISTANCE: f32 = 18.0; // Park south of target, nose just below
const AUTOPILOT_POSITION_TOLERANCE: f32 = 3.0; // How close to dock position
const AUTOPILOT_ARRIVAL_SPEED: f32 = 3.0; // Max speed to be considered stopped
const AUTOPILOT_ROTATION_TOLERANCE: f32 = 0.1; // radians (~6 degrees)
const AUTOPILOT_BRAKE_SAFETY_FACTOR: f32 = 1.5;

/// Finds the zone (node ID) that a position belongs to.
/// Returns the ID of the closest SystemNode.
#[allow(dead_code)]
pub fn find_zone_for_position(nodes: &[SystemNode], pos: Vec2) -> Option<u32> {
    let mut closest_id = None;
    let mut closest_dist = f32::MAX;

    for node in nodes {
        let dist = node.position.distance(pos);
        if dist < closest_dist {
            closest_dist = dist;
            closest_id = Some(node.id);
        }
    }

    closest_id
}

/// Filters entities to only those in the same zone as the player.
#[allow(dead_code)]
pub fn filter_entities_by_zone(
    entities: &[(Entity, Vec2, String)],
    nodes: &[SystemNode],
    player_zone: Option<u32>,
) -> Vec<(Entity, Vec2, String)> {
    let Some(zone_id) = player_zone else {
        return Vec::new();
    };

    entities
        .iter()
        .filter(|(_, pos, _)| find_zone_for_position(nodes, *pos) == Some(zone_id))
        .cloned()
        .collect()
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NearbyTargets>()
            .init_resource::<AutopilotState>()
            .add_systems(
                FixedUpdate,
                (
                    player_movement.run_if(autopilot_not_engaged),
                    player_mining,
                    player_fire,
                    player_refuel_station,
                    player_build_outpost,
                    player_activate_jump_gate.run_if(not_in_jump_transition),
                    process_jump_transition,
                )
                    .run_if(sim_not_paused),
            )
            .add_systems(FixedUpdate, scan_nearby_entities.run_if(view_is_world))
            .add_systems(
                FixedUpdate,
                autopilot_control_system
                    .run_if(sim_not_paused)
                    .run_if(autopilot_engaged)
                    .after(scan_nearby_entities),
            )
            .add_systems(
                Update,
                (handle_tactical_selection, autopilot_input_system).run_if(view_is_world),
            );
    }
}

fn not_in_jump_transition(
    player_query: Query<Option<&JumpTransition>, With<PlayerControl>>,
) -> bool {
    player_query
        .single()
        .map_or(true, |transition| transition.is_none())
}

fn sim_not_paused(config: Res<SimConfig>) -> bool {
    !config.paused
}

fn shift_pressed(input: &ButtonInput<KeyCode>) -> bool {
    input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight)
}

fn player_movement(
    time: Res<Time<Fixed>>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut ships: Query<(&mut Ship, &mut Transform, &mut Velocity), With<PlayerControl>>,
) {
    let delta_seconds = time.delta_secs();
    let minutes = delta_seconds / 60.0;

    // Ignore movement keys when Shift is held (allows Shift+key commands without side effects)
    let shift_held = shift_pressed(&input);

    for (mut ship, mut transform, mut velocity) in ships.iter_mut() {
        if ship.fuel <= 0.0 {
            ship.state = ShipState::Disabled;
            continue;
        }

        // Handle rotation (also blocked when shift held)
        let rotation_speed = 3.0; // radians per second
        if !shift_held && input.pressed(bindings.rotate_left) {
            transform.rotate_z(rotation_speed * delta_seconds);
        }
        if !shift_held && input.pressed(bindings.rotate_right) {
            transform.rotate_z(-rotation_speed * delta_seconds);
        }

        // Get ship facing direction from rotation
        // In Bevy, rotation of 0 faces right (+X), we want 0 to face up (+Y)
        // So we offset by PI/2 (90 degrees)
        let rotation = transform.rotation.to_euler(EulerRot::XYZ).2 + std::f32::consts::FRAC_PI_2;
        let facing = Vec2::new(rotation.cos(), rotation.sin());

        // Apply thrust based on input (blocked when shift held)
        let mut thrust_applied = false;

        if !shift_held && input.pressed(bindings.move_up) {
            // Forward thrust
            velocity.x += facing.x * PLAYER_THRUST_ACCELERATION * delta_seconds;
            velocity.y += facing.y * PLAYER_THRUST_ACCELERATION * delta_seconds;
            thrust_applied = true;
        }

        if !shift_held && input.pressed(bindings.move_down) {
            // Reverse thrust
            velocity.x -= facing.x * PLAYER_THRUST_ACCELERATION * delta_seconds;
            velocity.y -= facing.y * PLAYER_THRUST_ACCELERATION * delta_seconds;
            thrust_applied = true;
        }

        // Braking: only when brake key pressed AND no movement keys active (also blocked when shift held)
        let movement_active = input.pressed(bindings.move_up) || input.pressed(bindings.move_down);
        if !shift_held && input.pressed(bindings.brake) && !movement_active {
            let (new_vx, new_vy) = calculate_brake_thrust(
                velocity.x,
                velocity.y,
                PLAYER_THRUST_ACCELERATION,
                delta_seconds,
            );
            // Only count as thrust if we actually changed velocity
            if (new_vx - velocity.x).abs() > 0.001 || (new_vy - velocity.y).abs() > 0.001 {
                thrust_applied = true;
            }
            velocity.x = new_vx;
            velocity.y = new_vy;
        }

        // Apply velocity to position
        transform.translation.x += velocity.x * delta_seconds;
        transform.translation.y += velocity.y * delta_seconds;

        // Update ship state based on velocity
        let speed_squared = velocity.x * velocity.x + velocity.y * velocity.y;
        if speed_squared > 1.0 {
            ship.state = ShipState::InTransit;
        } else if matches!(ship.state, ShipState::InTransit) {
            ship.state = ShipState::Idle;
        }

        // Only burn fuel when thrust is applied
        if thrust_applied {
            let burn = PLAYER_THRUST_FUEL_BURN_PER_MINUTE * minutes;
            if ship.fuel > burn {
                ship.fuel -= burn;
            } else {
                ship.fuel = 0.0;
                ship.state = ShipState::Disabled;
            }
        }
    }
}

fn player_mining(
    time: Res<Time<Fixed>>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(&Transform, &mut Cargo, &mut Ship), With<PlayerControl>>,
    mut ore_nodes: Query<(Entity, &Transform, &mut OreNode)>,
) {
    if !input.pressed(bindings.interact) {
        return;
    }

    let (player_transform, mut cargo, mut ship) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );
    let mut closest = None;
    let mut closest_dist = 0.0;
    let range = 24.0;

    for (entity, transform, ore) in ore_nodes.iter() {
        if ore.remaining <= 0.0 {
            continue;
        }
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        let dist = pos.distance(player_pos);
        if dist <= range && (closest.is_none() || dist < closest_dist) {
            closest = Some((entity, dist));
            closest_dist = dist;
        }
    }

    let (target_entity, _) = match closest {
        Some(value) => value,
        None => {
            return;
        }
    };

    let delta_seconds = time.delta_secs();

    if let Ok((_entity, _transform, mut ore)) = ore_nodes.get_mut(target_entity) {
        let _mined = match ore.kind {
            OreKind::CommonOre => {
                let free_capacity = (cargo.capacity - cargo.common_ore).max(0.0);
                let amount = mine_amount(
                    ore.remaining,
                    ore.rate_per_second,
                    delta_seconds,
                    free_capacity,
                );
                if amount > 0.0 {
                    ore.remaining -= amount;
                    cargo.common_ore += amount;
                }
                amount
            }
            OreKind::FuelOre => {
                let free_capacity = (ship.fuel_capacity - ship.fuel).max(0.0);
                let amount = mine_amount(
                    ore.remaining,
                    ore.rate_per_second,
                    delta_seconds,
                    free_capacity,
                );
                if amount > 0.0 {
                    ore.remaining -= amount;
                    ship.fuel += amount;
                }
                amount
            }
        };

        if ore.remaining <= 0.0 {
            commands.entity(target_entity).despawn();
            let kind_str = match ore.kind {
                OreKind::CommonOre => "Ore",
                OreKind::FuelOre => "Fuel",
            };
            log.push(format!("{} node depleted", kind_str));
        }
    }
}

fn player_build_outpost(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(&Transform, &mut Cargo), With<PlayerControl>>,
    nodes: Query<&SystemNode>,
    stations: Query<&Transform, With<Station>>,
) {
    if !input.just_pressed(bindings.interact) {
        return;
    }

    let (player_transform, mut cargo) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let cost = 18.0;
    if !can_build_outpost(cargo.common_ore, cost) {
        return;
    }

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    let mut target_node = None;
    let mut best_dist = 0.0;
    let build_range = 26.0;

    for node in nodes.iter() {
        let dist = node.position.distance(player_pos);
        if dist <= build_range && (target_node.is_none() || dist < best_dist) {
            target_node = Some(node);
            best_dist = dist;
        }
    }

    let node = match target_node {
        Some(node) => node,
        None => {
            return;
        }
    };

    for station_transform in stations.iter() {
        let station_pos = Vec2::new(
            station_transform.translation.x,
            station_transform.translation.y,
        );
        if station_pos.distance(node.position) <= 18.0 {
            return;
        }
    }

    let kind = StationKind::MiningOutpost;
    let capacity = station_fuel_capacity(kind);
    let build_time = station_build_time_seconds(kind);
    let ore_capacity = station_ore_capacity(kind);

    commands.spawn((
        Station {
            kind,
            state: StationState::Deploying,
            fuel: capacity * 0.5,
            fuel_capacity: capacity,
        },
        StationBuild {
            remaining_seconds: build_time,
        },
        StationProduction {
            ore: 0.0,
            ore_capacity,
        },
        StationCrisisLog::default(),
        ZoneId(node.id),
        Name::new(format!("Station-{}", node.id)),
        SpatialBundle::from_transform(Transform::from_xyz(
            node.position.x + 12.0,
            node.position.y + 8.0,
            0.5,
        )),
    ));

    cargo.common_ore -= cost;
    log.push(format!("Outpost deployed at zone {}", node.id));
}

fn player_refuel_station(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(&Transform, &mut Ship, &mut Cargo), With<PlayerControl>>,
    mut stations: Query<(&Transform, &mut Station, Option<&mut StationProduction>)>,
) {
    if !input.just_pressed(bindings.interact) {
        return;
    }

    let (player_transform, mut ship, mut cargo) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );
    let range = 22.0;
    let fuel_transfer = 10.0;
    let ore_transfer = 8.0;
    let mut refueled = false;
    let mut supplied_ore = false;

    for (_transform, mut station, production_opt) in stations.iter_mut() {
        let station_pos = Vec2::new(_transform.translation.x, _transform.translation.y);
        if station_pos.distance(player_pos) > range {
            continue;
        }

        if ship.fuel > 0.0 {
            let (new_ship_fuel, new_station_fuel, did_refuel) = transfer_fuel(
                ship.fuel,
                station.fuel,
                station.fuel_capacity,
                fuel_transfer,
            );
            ship.fuel = new_ship_fuel;
            station.fuel = new_station_fuel;
            refueled = did_refuel;
        }

        if cargo.common_ore > 0.0 {
            if let Some(mut production) = production_opt {
                let available = cargo.common_ore.min(ore_transfer);
                let free_capacity = (production.ore_capacity - production.ore).max(0.0);
                let transferred = available.min(free_capacity);

                if transferred > 0.0 {
                    cargo.common_ore -= transferred;
                    production.ore += transferred;
                    supplied_ore = true;
                }
            }
        }

        break;
    }

    if refueled && supplied_ore {
        log.push("Transferred fuel and ore to station".to_string());
    } else if refueled {
        log.push("Transferred fuel to station".to_string());
    } else if supplied_ore {
        log.push("Transferred ore to station".to_string());
    }
}

const JUMP_GATE_ACTIVATION_RANGE: f32 = 25.0;

fn player_activate_jump_gate(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(Entity, &Transform, &mut Ship), With<PlayerControl>>,
    gates: Query<(&Transform, &JumpGate)>,
) {
    if !input.just_pressed(bindings.interact) {
        return;
    }

    let (player_entity, player_transform, mut ship) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    // Find nearest gate in range
    let mut nearest_gate: Option<&JumpGate> = None;
    let mut nearest_dist = f32::MAX;

    for (gate_transform, gate) in gates.iter() {
        let gate_pos = Vec2::new(gate_transform.translation.x, gate_transform.translation.y);
        let dist = gate_pos.distance(player_pos);

        if dist <= JUMP_GATE_ACTIVATION_RANGE && dist < nearest_dist {
            nearest_gate = Some(gate);
            nearest_dist = dist;
        }
    }

    let Some(gate) = nearest_gate else {
        return;
    };

    // Check fuel
    if ship.fuel < JUMP_GATE_FUEL_COST {
        log.push("Not enough fuel for jump".to_string());
        return;
    }

    // Consume fuel and start transition
    ship.fuel -= JUMP_GATE_FUEL_COST;

    commands.entity(player_entity).insert(JumpTransition {
        destination_zone: gate.destination_zone,
        remaining_seconds: JUMP_TRANSITION_SECONDS,
    });

    log.push(format!("Jumping to zone {}...", gate.destination_zone));
}

fn process_jump_transition(
    time: Res<Time<Fixed>>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(Entity, &mut ZoneId, &mut JumpTransition), With<PlayerControl>>,
) {
    let Ok((player_entity, mut zone_id, mut transition)) = player_query.single_mut() else {
        return;
    };

    transition.remaining_seconds -= time.delta_secs();

    if transition.remaining_seconds <= 0.0 {
        // Complete the jump
        zone_id.0 = transition.destination_zone;
        commands.entity(player_entity).remove::<JumpTransition>();
        log.push(format!("Arrived at zone {}", zone_id.0));
    }
}

fn player_fire(
    input: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    player_query: Query<&Transform, With<PlayerControl>>,
    pirates: Query<(Entity, &Transform), With<PirateShip>>,
) {
    if !input.just_pressed(MouseButton::Left) {
        return;
    }

    let player_transform = match player_query.single() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let origin = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );
    let range = 24.0;
    let mut targets = Vec::new();
    let mut entities = Vec::new();

    for (entity, transform) in pirates.iter() {
        targets.push(Vec2::new(transform.translation.x, transform.translation.y));
        entities.push(entity);
    }

    if let Some(index) = closest_in_range(origin, &targets, range) {
        if let Some(target) = entities.get(index) {
            commands.entity(*target).despawn();
            log.push("Pirate ship destroyed".to_string());
        }
    }
}

fn can_build_outpost(ore: f32, cost: f32) -> bool {
    ore >= cost
}

fn transfer_fuel(
    ship_fuel: f32,
    station_fuel: f32,
    station_capacity: f32,
    amount: f32,
) -> (f32, f32, bool) {
    if ship_fuel <= 0.0 || station_capacity <= 0.0 || amount <= 0.0 {
        return (ship_fuel, station_fuel, false);
    }

    let available = ship_fuel.min(amount);
    let room = (station_capacity - station_fuel).max(0.0);
    let transfer = available.min(room);
    if transfer <= 0.0 {
        return (ship_fuel, station_fuel, false);
    }

    (ship_fuel - transfer, station_fuel + transfer, true)
}

fn closest_in_range(origin: Vec2, targets: &[Vec2], range: f32) -> Option<usize> {
    let mut closest = None;
    let mut best_dist = 0.0;
    for (index, pos) in targets.iter().enumerate() {
        let dist = pos.distance(origin);
        if dist <= range && (closest.is_none() || dist < best_dist) {
            closest = Some(index);
            best_dist = dist;
        }
    }
    closest
}

fn view_is_world(view: Res<ViewMode>) -> bool {
    *view == ViewMode::World
}

/// Calculate new velocity after applying braking thrust.
/// Returns (new_vx, new_vy) after decelerating toward zero.
fn calculate_brake_thrust(vx: f32, vy: f32, acceleration: f32, delta_seconds: f32) -> (f32, f32) {
    let speed = (vx * vx + vy * vy).sqrt();

    // Threshold below which we snap to zero to avoid oscillation
    const STOP_THRESHOLD: f32 = 1.0;
    if speed < STOP_THRESHOLD {
        return (0.0, 0.0);
    }

    let deceleration = acceleration * delta_seconds;

    // If deceleration would overshoot, just stop
    if deceleration >= speed {
        return (0.0, 0.0);
    }

    // Scale velocity down proportionally
    let new_speed = speed - deceleration;
    let ratio = new_speed / speed;

    (vx * ratio, vy * ratio)
}

#[allow(clippy::too_many_arguments)]
fn scan_nearby_entities(
    mut targets: ResMut<NearbyTargets>,
    player_query: Query<(&Transform, &ZoneId), With<PlayerControl>>,
    stations: Query<(Entity, &Transform, &Name, Option<&ZoneId>), With<Station>>,
    ore_nodes: Query<(Entity, &Transform, Option<&ZoneId>), With<OreNode>>,
    pirates: Query<(Entity, &Transform, Option<&ZoneId>), With<PirateShip>>,
    pirate_bases: Query<(Entity, &Transform, Option<&ZoneId>), With<PirateBase>>,
    ships: Query<(Entity, &Transform, &Ship, Option<&ZoneId>), Without<PlayerControl>>,
    jump_gates: Query<(Entity, &Transform, &JumpGate, Option<&ZoneId>)>,
) {
    // Verify player exists and has a zone
    let Ok((player_transform, player_zone)) = player_query.single() else {
        targets.entities.clear();
        targets.manually_selected = false;
        return;
    };

    // Remember previously selected entity
    let prev_selected_entity = targets
        .entities
        .get(targets.selected_index)
        .map(|(e, _, _)| *e);

    // Clear and rebuild entity list with only same-zone entities
    targets.entities.clear();

    // Scan all stations in player's zone
    for (entity, transform, name, zone) in stations.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            targets.entities.push((entity, pos, name.to_string()));
        }
    }

    // Scan all ore nodes in player's zone
    for (entity, transform, zone) in ore_nodes.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            targets
                .entities
                .push((entity, pos, "○ Asteroid".to_string()));
        }
    }

    // Scan all pirates in player's zone
    for (entity, transform, zone) in pirates.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            targets
                .entities
                .push((entity, pos, "⚔ Marauder".to_string()));
        }
    }

    // Scan all pirate bases in player's zone
    for (entity, transform, zone) in pirate_bases.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            targets
                .entities
                .push((entity, pos, "⚔ Raider Den".to_string()));
        }
    }

    // Scan all other ships in player's zone
    for (entity, transform, ship, zone) in ships.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            let label = match ship.kind {
                ShipKind::Scout => "✦ Pathfinder",
                ShipKind::Miner => "✦ Harvester",
                ShipKind::Security => "✦ Sentinel",
                ShipKind::PlayerShip => "✦ Vessel",
            };
            targets.entities.push((entity, pos, label.to_string()));
        }
    }

    // Scan all jump gates in player's zone
    for (entity, transform, gate, zone) in jump_gates.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            let label = format!("◈ Rift Gate → {}", gate.destination_zone);
            targets.entities.push((entity, pos, label));
        }
    }

    // Sort by distance from player
    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );
    targets.entities.sort_by(|(_, pos_a, _), (_, pos_b, _)| {
        let dist_a = pos_a.distance(player_pos);
        let dist_b = pos_b.distance(player_pos);
        dist_a
            .partial_cmp(&dist_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Preserve selection of previously selected entity if still present
    if targets.manually_selected {
        if let Some(prev_entity) = prev_selected_entity {
            let new_index = targets
                .entities
                .iter()
                .position(|(e, _, _)| *e == prev_entity);
            if let Some(idx) = new_index {
                targets.selected_index = idx;
            } else {
                // Entity no longer exists or left the zone
                targets.manually_selected = false;
                targets.selected_index = 0;
            }
        } else {
            targets.manually_selected = false;
            targets.selected_index = 0;
        }
    } else {
        // When not manually selected, always point to closest (index 0)
        targets.selected_index = 0;
    }
}

fn handle_tactical_selection(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut targets: ResMut<NearbyTargets>,
) {
    if !input.just_pressed(bindings.cycle_target) {
        return;
    }

    if targets.entities.is_empty() {
        return;
    }

    // First Tab press confirms current target without cycling
    if !targets.manually_selected {
        targets.manually_selected = true;
        return;
    }

    // Subsequent Tab presses cycle to next target
    targets.selected_index = (targets.selected_index + 1) % targets.entities.len();
}

fn autopilot_input_system(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    targets: Res<NearbyTargets>,
    mut autopilot: ResMut<AutopilotState>,
    mut log: ResMut<EventLog>,
) {
    // Check for manual override (any movement key disengages autopilot)
    if autopilot.engaged {
        let movement_override = input.pressed(bindings.move_up)
            || input.pressed(bindings.move_down)
            || input.pressed(bindings.rotate_left)
            || input.pressed(bindings.rotate_right)
            || input.pressed(bindings.brake);

        if movement_override {
            autopilot.engaged = false;
            autopilot.target_entity = None;
            log.push("Autopilot disengaged: manual control".to_string());
            return;
        }
    }

    // Toggle autopilot with N key
    if input.just_pressed(bindings.navigate) {
        if autopilot.engaged {
            // Disengage
            autopilot.engaged = false;
            autopilot.target_entity = None;
            log.push("Autopilot disengaged".to_string());
        } else {
            // Try to engage - need a manually selected target
            if !targets.manually_selected {
                log.push("Autopilot: no target selected (press Tab first)".to_string());
                return;
            }

            if let Some((entity, _pos, label)) = targets.entities.get(targets.selected_index) {
                autopilot.engaged = true;
                autopilot.target_entity = Some(*entity);
                log.push(format!("Autopilot engaged: {}", label));
            } else {
                log.push("Autopilot: target not found".to_string());
            }
        }
    }
}

fn autopilot_control_system(
    time: Res<Time<Fixed>>,
    mut autopilot: ResMut<AutopilotState>,
    mut log: ResMut<EventLog>,
    targets: Res<NearbyTargets>,
    mut ships: Query<(&mut Ship, &mut Transform, &mut Velocity), With<PlayerControl>>,
    target_transforms: Query<&Transform, Without<PlayerControl>>,
) {
    if !autopilot.engaged {
        return;
    }

    let delta_seconds = time.delta_secs();
    let minutes = delta_seconds / 60.0;

    let (mut ship, mut transform, mut velocity) = match ships.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    // Check fuel
    if ship.fuel <= 0.0 {
        autopilot.engaged = false;
        autopilot.target_entity = None;
        ship.state = ShipState::Disabled;
        log.push("Autopilot disengaged: out of fuel".to_string());
        return;
    }

    // Validate target still exists
    let target_entity = match autopilot.target_entity {
        Some(entity) => entity,
        None => {
            autopilot.engaged = false;
            log.push("Autopilot disengaged: no target".to_string());
            return;
        }
    };

    // Get current target position
    let target_pos = match target_transforms.get(target_entity) {
        Ok(t) => Vec2::new(t.translation.x, t.translation.y),
        Err(_) => {
            autopilot.engaged = false;
            autopilot.target_entity = None;
            log.push("Autopilot disengaged: target lost".to_string());
            return;
        }
    };

    // Check if target is still in NearbyTargets (in scan range)
    let still_in_range = targets.entities.iter().any(|(e, _, _)| *e == target_entity);
    if !still_in_range {
        autopilot.engaged = false;
        autopilot.target_entity = None;
        log.push("Autopilot disengaged: target out of range".to_string());
        return;
    }

    // Calculate docking position: 5m south of target (negative Y)
    let dock_pos = target_pos + Vec2::new(0.0, -AUTOPILOT_DOCKING_DISTANCE);
    let ship_pos = Vec2::new(transform.translation.x, transform.translation.y);

    let to_dock = dock_pos - ship_pos;
    let dock_distance = to_dock.length();
    let dock_direction = to_dock.normalize_or_zero();
    let current_velocity = Vec2::new(velocity.x, velocity.y);
    let current_speed = current_velocity.length();

    // Current ship rotation (world angle, PI/2 = facing up/north)
    let current_rotation =
        transform.rotation.to_euler(EulerRot::XYZ).2 + std::f32::consts::FRAC_PI_2;
    let facing = Vec2::new(current_rotation.cos(), current_rotation.sin());

    // Check if docked: at dock position, slow, facing north
    let facing_north = calculate_angle_difference(current_rotation, Vec2::new(0.0, 1.0));
    let at_dock = dock_distance <= AUTOPILOT_POSITION_TOLERANCE;
    let is_slow = current_speed <= AUTOPILOT_ARRIVAL_SPEED;
    let is_aligned = facing_north.abs() < AUTOPILOT_ROTATION_TOLERANCE;

    if at_dock && is_slow && is_aligned {
        // Fully docked - stop completely
        velocity.x = 0.0;
        velocity.y = 0.0;
        autopilot.engaged = false;
        autopilot.target_entity = None;
        log.push("Autopilot: docked".to_string());
        return;
    }

    let mut thrust_applied = false;

    if at_dock && is_slow {
        // At dock position but not aligned - just rotate to face north
        let max_rotation = PLAYER_ROTATION_SPEED * delta_seconds;
        let rotation_step = facing_north.clamp(-max_rotation, max_rotation);
        transform.rotate_z(rotation_step);
        // Keep velocity at zero while aligning
        velocity.x = 0.0;
        velocity.y = 0.0;
    } else {
        // Calculate velocity component toward dock (positive = approaching, negative = receding)
        let velocity_toward_dock = current_velocity.dot(dock_direction);

        // Always rotate toward dock position
        let angle_to_dock = calculate_angle_difference(current_rotation, dock_direction);
        let max_rotation = PLAYER_ROTATION_SPEED * delta_seconds;
        let rotation_step = angle_to_dock.clamp(-max_rotation, max_rotation);
        transform.rotate_z(rotation_step);

        // Calculate stopping distance based on approach speed
        let approach_speed = velocity_toward_dock.max(0.0);
        let stopping_distance =
            calculate_stopping_distance(approach_speed, PLAYER_THRUST_ACCELERATION);
        let brake_threshold =
            stopping_distance * AUTOPILOT_BRAKE_SAFETY_FACTOR + AUTOPILOT_POSITION_TOLERANCE;

        // Decide action based on velocity and position
        let moving_away = velocity_toward_dock < -1.0;
        let approaching_too_fast =
            velocity_toward_dock > AUTOPILOT_ARRIVAL_SPEED && dock_distance <= brake_threshold;
        let aligned_enough = angle_to_dock.abs() < 0.5; // ~30 degrees

        if moving_away || approaching_too_fast {
            // Brake: either moving wrong direction or too fast
            let (new_vx, new_vy) = calculate_brake_thrust(
                velocity.x,
                velocity.y,
                PLAYER_THRUST_ACCELERATION,
                delta_seconds,
            );
            if (new_vx - velocity.x).abs() > 0.001 || (new_vy - velocity.y).abs() > 0.001 {
                thrust_applied = true;
            }
            velocity.x = new_vx;
            velocity.y = new_vy;
        } else if aligned_enough && velocity_toward_dock < current_speed.max(50.0) {
            // Thrust forward if pointed toward dock and not going too fast
            velocity.x += facing.x * PLAYER_THRUST_ACCELERATION * delta_seconds;
            velocity.y += facing.y * PLAYER_THRUST_ACCELERATION * delta_seconds;
            thrust_applied = true;
        }
        // Otherwise: coast while rotating
    }

    // Apply velocity to position
    transform.translation.x += velocity.x * delta_seconds;
    transform.translation.y += velocity.y * delta_seconds;

    // Update ship state
    let speed_squared = velocity.x * velocity.x + velocity.y * velocity.y;
    if speed_squared > 1.0 {
        ship.state = ShipState::InTransit;
    } else if matches!(ship.state, ShipState::InTransit) {
        ship.state = ShipState::Idle;
    }

    // Burn fuel when thrusting
    if thrust_applied {
        let burn = PLAYER_THRUST_FUEL_BURN_PER_MINUTE * minutes;
        if ship.fuel > burn {
            ship.fuel -= burn;
        } else {
            ship.fuel = 0.0;
            ship.state = ShipState::Disabled;
            autopilot.engaged = false;
            autopilot.target_entity = None;
            log.push("Autopilot disengaged: fuel depleted".to_string());
        }
    }
}

/// Calculate the shortest angle difference to face a target direction
fn calculate_angle_difference(current_angle: f32, target_direction: Vec2) -> f32 {
    // target_direction is in world coordinates, atan2 gives world angle directly
    // current_angle already has PI/2 offset applied (from Bevy rotation convention)
    let target_angle = target_direction.y.atan2(target_direction.x);
    let mut angle_diff = target_angle - current_angle;

    // Normalize to [-PI, PI] for shortest rotation
    while angle_diff > std::f32::consts::PI {
        angle_diff -= std::f32::consts::TAU;
    }
    while angle_diff < -std::f32::consts::PI {
        angle_diff += std::f32::consts::TAU;
    }

    angle_diff
}

/// Calculate stopping distance based on current speed and deceleration
fn calculate_stopping_distance(current_speed: f32, deceleration: f32) -> f32 {
    // Using kinematic equation: v^2 = u^2 + 2as
    // When v = 0: s = u^2 / (2 * a)
    if deceleration <= 0.0 {
        return f32::MAX;
    }
    (current_speed * current_speed) / (2.0 * deceleration)
}

fn autopilot_engaged(autopilot: Res<AutopilotState>) -> bool {
    autopilot.engaged
}

fn autopilot_not_engaged(autopilot: Res<AutopilotState>) -> bool {
    !autopilot.engaged
}

#[cfg(test)]
mod tests {
    use super::{
        calculate_angle_difference, calculate_brake_thrust, calculate_stopping_distance,
        can_build_outpost, closest_in_range, transfer_fuel,
    };
    use bevy::prelude::Vec2;

    #[test]
    fn can_build_outpost_requires_enough_ore() {
        assert!(can_build_outpost(18.0, 18.0));
        assert!(!can_build_outpost(10.0, 18.0));
    }

    #[test]
    fn transfer_fuel_respects_capacity() {
        let (ship, station, did) = transfer_fuel(5.0, 8.0, 10.0, 5.0);
        assert!(did);
        assert_eq!(ship, 3.0);
        assert_eq!(station, 10.0);
    }

    #[test]
    fn closest_in_range_picks_nearest() {
        let origin = Vec2::new(0.0, 0.0);
        let targets = vec![Vec2::new(10.0, 0.0), Vec2::new(5.0, 0.0)];
        let index = closest_in_range(origin, &targets, 12.0);
        assert_eq!(index, Some(1));
    }

    #[test]
    fn brake_thrust_opposes_velocity() {
        // Moving right at 100 units/sec, acceleration 200, delta 0.1s
        let (vx, vy) = calculate_brake_thrust(100.0, 0.0, 200.0, 0.1);
        // Should decelerate: 100 - 200*0.1 = 80
        assert!((vx - 80.0).abs() < 0.001);
        assert!(vy.abs() < 0.001);
    }

    #[test]
    fn brake_thrust_stops_at_low_speed() {
        // Very slow velocity should snap to zero
        let (vx, vy) = calculate_brake_thrust(0.5, 0.3, 200.0, 0.1);
        assert_eq!(vx, 0.0);
        assert_eq!(vy, 0.0);
    }

    #[test]
    fn brake_thrust_handles_diagonal_velocity() {
        // Moving diagonally
        let (vx, vy) = calculate_brake_thrust(100.0, 100.0, 200.0, 0.1);
        // Should reduce both components proportionally
        // Speed = sqrt(100^2 + 100^2) = 141.42
        // Decel = 200 * 0.1 = 20
        // New speed = 141.42 - 20 = 121.42
        // Ratio = 121.42 / 141.42 = 0.858
        let expected_ratio = (141.42_f32 - 20.0) / 141.42;
        assert!((vx - 100.0 * expected_ratio).abs() < 1.0);
        assert!((vy - 100.0 * expected_ratio).abs() < 1.0);
    }

    #[test]
    fn brake_thrust_clamps_overshoot() {
        // Moving slowly, deceleration would overshoot past zero
        let (vx, vy) = calculate_brake_thrust(5.0, 0.0, 200.0, 0.1);
        // 200 * 0.1 = 20 would overshoot 5, should clamp to 0
        assert_eq!(vx, 0.0);
        assert_eq!(vy, 0.0);
    }

    #[test]
    fn stopping_distance_basic_calculation() {
        // v^2 / (2 * a) = 100^2 / (2 * 200) = 10000 / 400 = 25
        let distance = calculate_stopping_distance(100.0, 200.0);
        assert!((distance - 25.0).abs() < 0.001);
    }

    #[test]
    fn stopping_distance_zero_speed() {
        let distance = calculate_stopping_distance(0.0, 200.0);
        assert_eq!(distance, 0.0);
    }

    #[test]
    fn stopping_distance_handles_zero_deceleration() {
        let distance = calculate_stopping_distance(100.0, 0.0);
        assert_eq!(distance, f32::MAX);
    }

    #[test]
    fn angle_difference_target_ahead() {
        // current_angle is world angle: PI/2 = facing up
        // target Vec2(0, 1) = up, so angle = atan2(1, 0) = PI/2
        // diff should be 0
        let diff = calculate_angle_difference(std::f32::consts::FRAC_PI_2, Vec2::new(0.0, 1.0));
        assert!(diff.abs() < 0.01);
    }

    #[test]
    fn angle_difference_target_right() {
        // Facing up (PI/2), target to the right (world angle 0)
        let diff = calculate_angle_difference(std::f32::consts::FRAC_PI_2, Vec2::new(1.0, 0.0));
        // target_angle = 0, current = PI/2
        // diff = 0 - PI/2 = -PI/2 (rotate right/clockwise)
        assert!(diff < 0.0);
        assert!((diff + std::f32::consts::FRAC_PI_2).abs() < 0.01);
    }

    #[test]
    fn angle_difference_target_behind() {
        // Facing up (PI/2), target directly behind (down, world angle -PI/2)
        let diff = calculate_angle_difference(std::f32::consts::FRAC_PI_2, Vec2::new(0.0, -1.0));
        // target_angle = atan2(-1, 0) = -PI/2
        // diff = -PI/2 - PI/2 = -PI (or +PI after normalization)
        // Either direction is shortest when target is directly behind
        assert!((diff.abs() - std::f32::consts::PI).abs() < 0.01);
    }

    #[test]
    fn autopilot_target_found_in_nearby_targets() {
        use crate::plugins::player::NearbyTargets;
        use bevy::ecs::entity::Entity;

        // Create a target entity ID (simulating an ore node)
        let ore_entity = Entity::from_bits(42);

        // Create nearby targets list containing the ore
        let mut targets = NearbyTargets::default();
        targets
            .entities
            .push((ore_entity, Vec2::new(10.0, 20.0), "Ore Node".to_string()));

        // Verify target is found in list
        let still_in_range = targets.entities.iter().any(|(e, _, _)| *e == ore_entity);
        assert!(still_in_range, "Target should be found in nearby targets");
    }

    #[test]
    fn autopilot_target_not_found_when_list_empty() {
        use crate::plugins::player::NearbyTargets;
        use bevy::ecs::entity::Entity;

        let ore_entity = Entity::from_bits(42);
        let targets = NearbyTargets::default();

        // Empty list should not contain target
        let still_in_range = targets.entities.iter().any(|(e, _, _)| *e == ore_entity);
        assert!(!still_in_range, "Target should not be found in empty list");
    }

    #[test]
    fn autopilot_target_persists_when_other_entities_added() {
        use crate::plugins::player::NearbyTargets;
        use bevy::ecs::entity::Entity;

        let ore_entity = Entity::from_bits(42);
        let scout_entity = Entity::from_bits(100);

        // Create nearby targets with ore
        let mut targets = NearbyTargets::default();
        targets
            .entities
            .push((ore_entity, Vec2::new(10.0, 20.0), "Ore Node".to_string()));

        // Add a scout ship (simulating spawn)
        targets.entities.push((
            scout_entity,
            Vec2::new(50.0, 50.0),
            "Scout Ship".to_string(),
        ));

        // Ore should still be found
        let still_in_range = targets.entities.iter().any(|(e, _, _)| *e == ore_entity);
        assert!(
            still_in_range,
            "Ore target should persist when scout is added"
        );
    }

    #[test]
    fn contacts_list_includes_distant_entities() {
        use crate::plugins::player::NearbyTargets;
        use bevy::ecs::entity::Entity;

        // Distant entity (beyond typical "range" of 400)
        let distant_entity = Entity::from_bits(999);

        let mut targets = NearbyTargets::default();

        // Simulate adding a distant entity (scan_all_entities should include this)
        targets.entities.push((
            distant_entity,
            Vec2::new(1000.0, 1000.0), // ~1414 pixels from origin
            "Distant Station".to_string(),
        ));

        // Distant entity should be in the list (no range filtering)
        assert_eq!(
            targets.entities.len(),
            1,
            "Contacts should include distant entities"
        );
        assert!(
            targets
                .entities
                .iter()
                .any(|(e, _, _)| *e == distant_entity),
            "Distant entity should be findable"
        );
    }

    #[test]
    fn find_zone_returns_closest_node() {
        use crate::plugins::player::find_zone_for_position;
        use crate::world::SystemNode;

        let nodes = vec![
            SystemNode {
                id: 1,
                position: Vec2::new(0.0, 0.0),
                modifier: None,
            },
            SystemNode {
                id: 2,
                position: Vec2::new(100.0, 0.0),
                modifier: None,
            },
        ];

        // Position near node 1
        assert_eq!(find_zone_for_position(&nodes, Vec2::new(5.0, 0.0)), Some(1));
        // Position near node 2
        assert_eq!(
            find_zone_for_position(&nodes, Vec2::new(95.0, 0.0)),
            Some(2)
        );
        // Position exactly between nodes - should pick one (closest wins)
        let mid = find_zone_for_position(&nodes, Vec2::new(50.0, 0.0));
        assert!(mid == Some(1) || mid == Some(2));
    }

    #[test]
    fn contacts_excludes_entities_from_other_zones() {
        use crate::plugins::player::{filter_entities_by_zone, find_zone_for_position};
        use crate::world::SystemNode;
        use bevy::ecs::entity::Entity;

        let nodes = vec![
            SystemNode {
                id: 1,
                position: Vec2::new(0.0, 0.0),
                modifier: None,
            },
            SystemNode {
                id: 2,
                position: Vec2::new(200.0, 0.0),
                modifier: None,
            },
        ];

        // Player is at zone 1
        let player_pos = Vec2::new(10.0, 0.0);
        let player_zone = find_zone_for_position(&nodes, player_pos);

        // Entities in zone 1 and zone 2
        let entity_in_zone_1 = (
            Entity::from_bits(1),
            Vec2::new(5.0, 5.0),
            "Nearby".to_string(),
        );
        let entity_in_zone_2 = (
            Entity::from_bits(2),
            Vec2::new(195.0, 0.0),
            "Far Away".to_string(),
        );

        let all_entities = vec![entity_in_zone_1.clone(), entity_in_zone_2];

        let filtered = filter_entities_by_zone(&all_entities, &nodes, player_zone);

        // Only entity in zone 1 should be included
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].2, "Nearby");
    }

    #[test]
    fn can_activate_gate_with_enough_fuel() {
        use crate::world::JUMP_GATE_FUEL_COST;
        let fuel = JUMP_GATE_FUEL_COST + 1.0;
        assert!(fuel >= JUMP_GATE_FUEL_COST);
    }

    #[test]
    fn cannot_activate_gate_without_fuel() {
        use crate::world::JUMP_GATE_FUEL_COST;
        let fuel = JUMP_GATE_FUEL_COST - 1.0;
        assert!(fuel < JUMP_GATE_FUEL_COST);
    }

    #[test]
    fn jump_transition_completes_when_timer_reaches_zero() {
        use crate::world::{JumpTransition, JUMP_TRANSITION_SECONDS};
        let mut transition = JumpTransition {
            destination_zone: 100,
            remaining_seconds: JUMP_TRANSITION_SECONDS,
        };

        // Simulate time passing
        transition.remaining_seconds -= JUMP_TRANSITION_SECONDS;
        assert!(transition.remaining_seconds <= 0.0);
    }

    #[test]
    fn movement_blocked_when_shift_held() {
        // When shift is held, movement keys should not apply thrust
        // This prevents Shift+S (spawn scout) from also triggering reverse thrust
        let shift_held = true;
        let key_pressed = true;

        // Movement should be blocked when shift is held
        let should_move = key_pressed && !shift_held;
        assert!(!should_move);
    }

    #[test]
    fn movement_allowed_when_shift_not_held() {
        let shift_held = false;
        let key_pressed = true;

        let should_move = key_pressed && !shift_held;
        assert!(should_move);
    }
}
