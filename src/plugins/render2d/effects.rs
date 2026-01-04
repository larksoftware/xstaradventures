//! Visual effects: focus marker, tactical navigation, home beacon.

use bevy::prelude::*;

use crate::plugins::player::{NearbyTargets, PlayerControl};
use crate::world::SystemIntel;

use super::map::FocusMarker;

// =============================================================================
// Resources
// =============================================================================

#[derive(Resource, Default)]
pub struct HomeBeaconEnabled {
    pub enabled: bool,
}

// =============================================================================
// Constants
// =============================================================================

const BEACON_ARROW_OFFSET: f32 = 40.0;
const BEACON_ARROW_LENGTH: f32 = 40.0;
const BEACON_TIP_SIZE: f32 = 8.0;

// =============================================================================
// Systems
// =============================================================================

pub fn draw_focus_marker(mut gizmos: Gizmos, marker: Res<FocusMarker>) {
    let position = match marker.position() {
        Some(position) => position,
        None => {
            return;
        }
    };

    let color = Color::srgba(0.85, 0.9, 1.0, 0.6);
    let size = 10.0;

    gizmos.line_2d(
        position + Vec2::new(-size, 0.0),
        position + Vec2::new(size, 0.0),
        color,
    );
    gizmos.line_2d(
        position + Vec2::new(0.0, -size),
        position + Vec2::new(0.0, size),
        color,
    );
    gizmos.circle_2d(position, size * 0.6, Color::srgba(0.7, 0.85, 0.95, 0.35));
}

pub fn draw_tactical_navigation(
    mut gizmos: Gizmos,
    targets: Res<NearbyTargets>,
    player_query: Query<&Transform, With<PlayerControl>>,
) {
    // Only draw when player has manually selected a target
    if !targets.manually_selected {
        return;
    }

    if targets.entities.is_empty() {
        return;
    }

    let player_transform = match player_query.single() {
        Ok(transform) => transform,
        Err(_) => {
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    let selected = match targets.entities.get(targets.selected_index) {
        Some(target) => target,
        None => {
            return;
        }
    };

    let target_pos = selected.1;
    let distance = player_pos.distance(target_pos);

    // Green color for locked-on target
    let arrow_color = Color::srgba(0.2, 1.0, 0.3, 0.9);
    let target_color = Color::srgba(0.2, 1.0, 0.3, 0.8);

    // Draw targeting reticle on the target
    let reticle_size = 18.0;
    let inner_gap = reticle_size * 0.4;
    // Draw crosshair lines with gap in center
    gizmos.line_2d(
        target_pos + Vec2::new(-reticle_size, 0.0),
        target_pos + Vec2::new(-inner_gap, 0.0),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(inner_gap, 0.0),
        target_pos + Vec2::new(reticle_size, 0.0),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(0.0, -reticle_size),
        target_pos + Vec2::new(0.0, -inner_gap),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(0.0, inner_gap),
        target_pos + Vec2::new(0.0, reticle_size),
        target_color,
    );
    // Draw corner brackets
    let bracket_size = reticle_size + 6.0;
    let corner_len = 6.0;
    // Top-left
    gizmos.line_2d(
        target_pos + Vec2::new(-bracket_size, bracket_size),
        target_pos + Vec2::new(-bracket_size + corner_len, bracket_size),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(-bracket_size, bracket_size),
        target_pos + Vec2::new(-bracket_size, bracket_size - corner_len),
        target_color,
    );
    // Top-right
    gizmos.line_2d(
        target_pos + Vec2::new(bracket_size, bracket_size),
        target_pos + Vec2::new(bracket_size - corner_len, bracket_size),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(bracket_size, bracket_size),
        target_pos + Vec2::new(bracket_size, bracket_size - corner_len),
        target_color,
    );
    // Bottom-left
    gizmos.line_2d(
        target_pos + Vec2::new(-bracket_size, -bracket_size),
        target_pos + Vec2::new(-bracket_size + corner_len, -bracket_size),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(-bracket_size, -bracket_size),
        target_pos + Vec2::new(-bracket_size, -bracket_size + corner_len),
        target_color,
    );
    // Bottom-right
    gizmos.line_2d(
        target_pos + Vec2::new(bracket_size, -bracket_size),
        target_pos + Vec2::new(bracket_size - corner_len, -bracket_size),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(bracket_size, -bracket_size),
        target_pos + Vec2::new(bracket_size, -bracket_size + corner_len),
        target_color,
    );

    const NEAR_THRESHOLD: f32 = 150.0;

    // Draw directional arrow near player when target is far
    if distance > NEAR_THRESHOLD {
        let direction = (target_pos - player_pos).normalize_or_zero();
        if direction.length_squared() < 0.01 {
            return;
        }

        let arrow_offset = 40.0;
        let arrow_length = 40.0;
        let arrow_start = player_pos + direction * arrow_offset;
        let arrow_end = arrow_start + direction * arrow_length;

        gizmos.line_2d(arrow_start, arrow_end, arrow_color);

        let tip_size = 8.0;
        let perpendicular = Vec2::new(-direction.y, direction.x);
        gizmos.line_2d(
            arrow_end,
            arrow_end - direction * tip_size + perpendicular * (tip_size * 0.5),
            arrow_color,
        );
        gizmos.line_2d(
            arrow_end,
            arrow_end - direction * tip_size - perpendicular * (tip_size * 0.5),
            arrow_color,
        );

        // Draw lock-on brackets near player
        let bracket_size = 6.0;
        let bracket_offset = arrow_offset - 10.0;
        let bracket_pos = player_pos + direction * bracket_offset;
        gizmos.line_2d(
            bracket_pos + perpendicular * bracket_size,
            bracket_pos + perpendicular * bracket_size * 0.5,
            arrow_color,
        );
        gizmos.line_2d(
            bracket_pos - perpendicular * bracket_size,
            bracket_pos - perpendicular * bracket_size * 0.5,
            arrow_color,
        );
    }
}

pub fn draw_home_beacon(
    mut gizmos: Gizmos,
    beacon: Res<HomeBeaconEnabled>,
    player_query: Query<&Transform, With<PlayerControl>>,
    nodes: Query<(&crate::world::SystemNode, &SystemIntel)>,
) {
    if !beacon.enabled {
        return;
    }

    let player_transform = match player_query.single() {
        Ok(t) => t,
        Err(_) => return,
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    let mut nearest: Option<Vec2> = None;
    let mut best_dist = f32::MAX;

    for (node, intel) in nodes.iter() {
        if !intel.revealed {
            continue;
        }
        let dist = node.position.distance(player_pos);
        if dist < best_dist {
            best_dist = dist;
            nearest = Some(node.position);
        }
    }

    if let Some(target) = nearest {
        let direction = (target - player_pos).normalize_or_zero();
        if direction.length_squared() < 0.01 {
            return;
        }

        let arrow_start = player_pos + direction * BEACON_ARROW_OFFSET;
        let arrow_end = arrow_start + direction * BEACON_ARROW_LENGTH;

        let beacon_color = Color::srgba(0.0, 1.0, 1.0, 0.8);
        gizmos.line_2d(arrow_start, arrow_end, beacon_color);

        let perpendicular = Vec2::new(-direction.y, direction.x);
        gizmos.line_2d(
            arrow_end,
            arrow_end - direction * BEACON_TIP_SIZE + perpendicular * (BEACON_TIP_SIZE * 0.5),
            beacon_color,
        );
        gizmos.line_2d(
            arrow_end,
            arrow_end - direction * BEACON_TIP_SIZE - perpendicular * (BEACON_TIP_SIZE * 0.5),
            beacon_color,
        );
    }
}
