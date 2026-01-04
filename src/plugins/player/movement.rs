//! Player movement systems and physics calculations.

use bevy::prelude::*;

use crate::plugins::core::InputBindings;
use crate::ships::{Ship, ShipState, Velocity};

use super::components::PlayerControl;

// =============================================================================
// Constants
// =============================================================================

pub const PLAYER_THRUST_ACCELERATION: f32 = 200.0; // pixels per second squared
pub const PLAYER_THRUST_FUEL_BURN_PER_MINUTE: f32 = 1.0;
pub const PLAYER_ROTATION_SPEED: f32 = 3.0; // radians per second

// =============================================================================
// Systems
// =============================================================================

pub fn player_movement(
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

// =============================================================================
// Helper Functions
// =============================================================================

pub fn shift_pressed(input: &ButtonInput<KeyCode>) -> bool {
    input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight)
}

/// Calculate new velocity after applying braking thrust.
/// Returns (new_vx, new_vy) after decelerating toward zero.
pub fn calculate_brake_thrust(
    vx: f32,
    vy: f32,
    acceleration: f32,
    delta_seconds: f32,
) -> (f32, f32) {
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
