use bevy::prelude::*;

use crate::compat::SpatialBundle;

use crate::ore::{mine_amount, OreKind, OreNode};
use crate::pirates::{PirateBase, PirateShip};
use crate::plugins::core::EventLog;
use crate::plugins::core::InputBindings;
use crate::plugins::core::SimConfig;
use crate::plugins::core::ViewMode;
use crate::ships::{Cargo, Ship, ShipState, Velocity};
use crate::stations::{
    station_build_time_seconds, station_fuel_capacity, station_ore_capacity, Station, StationBuild,
    StationCrisisLog, StationKind, StationProduction, StationState,
};
use crate::world::SystemNode;

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
                )
                    .run_if(sim_not_paused),
            )
            .add_systems(
                FixedUpdate,
                autopilot_control_system
                    .run_if(sim_not_paused)
                    .run_if(autopilot_engaged),
            )
            .add_systems(FixedUpdate, scan_nearby_entities.run_if(view_is_world))
            .add_systems(
                Update,
                (handle_tactical_selection, autopilot_input_system).run_if(view_is_world),
            );
    }
}

fn sim_not_paused(config: Res<SimConfig>) -> bool {
    !config.paused
}

fn player_movement(
    time: Res<Time<Fixed>>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut ships: Query<(&mut Ship, &mut Transform, &mut Velocity), With<PlayerControl>>,
) {
    let delta_seconds = time.delta_secs();
    let minutes = delta_seconds / 60.0;

    for (mut ship, mut transform, mut velocity) in ships.iter_mut() {
        if ship.fuel <= 0.0 {
            ship.state = ShipState::Disabled;
            continue;
        }

        // Handle rotation
        let rotation_speed = 3.0; // radians per second
        if input.pressed(bindings.rotate_left) {
            transform.rotate_z(rotation_speed * delta_seconds);
        }
        if input.pressed(bindings.rotate_right) {
            transform.rotate_z(-rotation_speed * delta_seconds);
        }

        // Get ship facing direction from rotation
        // In Bevy, rotation of 0 faces right (+X), we want 0 to face up (+Y)
        // So we offset by PI/2 (90 degrees)
        let rotation = transform.rotation.to_euler(EulerRot::XYZ).2 + std::f32::consts::FRAC_PI_2;
        let facing = Vec2::new(rotation.cos(), rotation.sin());

        // Apply thrust based on input
        let mut thrust_applied = false;

        if input.pressed(bindings.move_up) {
            // Forward thrust
            velocity.x += facing.x * PLAYER_THRUST_ACCELERATION * delta_seconds;
            velocity.y += facing.y * PLAYER_THRUST_ACCELERATION * delta_seconds;
            thrust_applied = true;
        }

        if input.pressed(bindings.move_down) {
            // Reverse thrust
            velocity.x -= facing.x * PLAYER_THRUST_ACCELERATION * delta_seconds;
            velocity.y -= facing.y * PLAYER_THRUST_ACCELERATION * delta_seconds;
            thrust_applied = true;
        }

        // Braking: only when brake key pressed AND no movement keys active
        let movement_active = input.pressed(bindings.move_up) || input.pressed(bindings.move_down);
        if input.pressed(bindings.brake) && !movement_active {
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
        Name::new(format!("Station-{}", node.id)),
        SpatialBundle::from_transform(Transform::from_xyz(
            node.position.x + 12.0,
            node.position.y + 8.0,
            0.5,
        )),
    ));

    cargo.common_ore -= cost;
    log.push(format!("Outpost deployed at node {}", node.id));
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

fn scan_nearby_entities(
    mut targets: ResMut<NearbyTargets>,
    player_query: Query<&Transform, With<PlayerControl>>,
    stations: Query<(Entity, &Transform, &Name), With<Station>>,
    ore_nodes: Query<(Entity, &Transform), With<OreNode>>,
    pirates: Query<(Entity, &Transform), With<PirateShip>>,
    pirate_bases: Query<(Entity, &Transform), With<PirateBase>>,
    ships: Query<(Entity, &Transform, &Ship), Without<PlayerControl>>,
) {
    let player_transform = match player_query.single() {
        Ok(transform) => transform,
        Err(_) => {
            targets.entities.clear();
            targets.manually_selected = false;
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );
    let range = 400.0;

    // Remember previously selected entity to check if it's still in range
    let prev_selected_entity = targets
        .entities
        .get(targets.selected_index)
        .map(|(e, _, _)| *e);

    targets.entities.clear();

    // Scan stations
    for (entity, transform, name) in stations.iter() {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        if pos.distance(player_pos) <= range {
            targets.entities.push((entity, pos, name.to_string()));
        }
    }

    // Scan ore nodes
    for (entity, transform) in ore_nodes.iter() {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        if pos.distance(player_pos) <= range {
            targets.entities.push((entity, pos, "Ore Node".to_string()));
        }
    }

    // Scan pirates
    for (entity, transform) in pirates.iter() {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        if pos.distance(player_pos) <= range {
            targets.entities.push((entity, pos, "Pirate".to_string()));
        }
    }

    // Scan pirate bases
    for (entity, transform) in pirate_bases.iter() {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        if pos.distance(player_pos) <= range {
            targets
                .entities
                .push((entity, pos, "Pirate Base".to_string()));
        }
    }

    // Scan other ships
    for (entity, transform, ship) in ships.iter() {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        if pos.distance(player_pos) <= range {
            let label = format!("{:?} Ship", ship.kind);
            targets.entities.push((entity, pos, label));
        }
    }

    // Check if previously selected entity is still in range and update index
    if targets.manually_selected {
        if let Some(prev_entity) = prev_selected_entity {
            // Find the new index of the previously selected entity
            let new_index = targets
                .entities
                .iter()
                .position(|(e, _, _)| *e == prev_entity);
            if let Some(idx) = new_index {
                targets.selected_index = idx;
            } else {
                // Entity no longer in range
                targets.manually_selected = false;
                targets.selected_index = 0;
            }
        } else {
            targets.manually_selected = false;
            targets.selected_index = 0;
        }
    }

    // Ensure selected_index is valid
    if targets.selected_index >= targets.entities.len() && !targets.entities.is_empty() {
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
        let approaching_too_fast = velocity_toward_dock > AUTOPILOT_ARRIVAL_SPEED
            && dock_distance <= brake_threshold;
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
}
