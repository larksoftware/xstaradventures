//! Autopilot control systems and navigation calculations.

use bevy::prelude::*;

use crate::plugins::core::{EventLog, InputBindings};
use crate::ships::{Ship, ShipState, Velocity};

use super::components::{AutopilotState, NearbyTargets, PlayerControl};
use super::movement::{
    calculate_brake_thrust, PLAYER_ROTATION_SPEED, PLAYER_THRUST_ACCELERATION,
    PLAYER_THRUST_FUEL_BURN_PER_MINUTE,
};

// =============================================================================
// Constants
// =============================================================================

const AUTOPILOT_DOCKING_DISTANCE: f32 = 18.0; // Park south of target, nose just below
const AUTOPILOT_POSITION_TOLERANCE: f32 = 3.0; // How close to dock position
const AUTOPILOT_ARRIVAL_SPEED: f32 = 3.0; // Max speed to be considered stopped
const AUTOPILOT_ROTATION_TOLERANCE: f32 = 0.1; // radians (~6 degrees)
const AUTOPILOT_BRAKE_SAFETY_FACTOR: f32 = 1.5;

// =============================================================================
// Run Conditions
// =============================================================================

pub fn autopilot_engaged(autopilot: Res<AutopilotState>) -> bool {
    autopilot.engaged
}

pub fn autopilot_not_engaged(autopilot: Res<AutopilotState>) -> bool {
    !autopilot.engaged
}

// =============================================================================
// Systems
// =============================================================================

pub fn autopilot_input_system(
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

pub fn autopilot_control_system(
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

// =============================================================================
// Navigation Calculations
// =============================================================================

/// Calculate the shortest angle difference to face a target direction
pub fn calculate_angle_difference(current_angle: f32, target_direction: Vec2) -> f32 {
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
pub fn calculate_stopping_distance(current_speed: f32, deceleration: f32) -> f32 {
    // Using kinematic equation: v^2 = u^2 + 2as
    // When v = 0: s = u^2 / (2 * a)
    if deceleration <= 0.0 {
        return f32::MAX;
    }
    (current_speed * current_speed) / (2.0 * deceleration)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
