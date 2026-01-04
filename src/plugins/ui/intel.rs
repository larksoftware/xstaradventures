//! Intel panel systems.

use bevy::prelude::*;

use crate::plugins::player::{NearbyTargets, PlayerControl};

use super::components::{IntelContentText, IntelInfo};

// =============================================================================
// Systems
// =============================================================================

pub fn update_intel_panel(
    targets: Res<NearbyTargets>,
    player_query: Query<&Transform, With<PlayerControl>>,
    mut intel_text: Query<&mut Text, With<IntelContentText>>,
) {
    let mut text = match intel_text.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };

    // Get player position for distance calculation
    let player_pos = match player_query.single() {
        Ok(transform) => Vec2::new(transform.translation.x, transform.translation.y),
        Err(_) => {
            text.0 = "No target selected".to_string();
            return;
        }
    };

    // Get selected entity info
    let Some((_, pos, label)) = targets.entities.get(targets.selected_index) else {
        text.0 = "No target selected".to_string();
        return;
    };

    if !targets.manually_selected {
        text.0 = "No target selected".to_string();
        return;
    }

    let distance = player_pos.distance(*pos);
    let info = IntelInfo {
        entity: Entity::PLACEHOLDER,
        label: label.clone(),
        position: *pos,
        distance,
    };

    text.0 = format_intel_panel(Some(&info));
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Formats the Intel panel content for the selected target.
pub fn format_intel_panel(info: Option<&IntelInfo>) -> String {
    let Some(info) = info else {
        return "No target selected".to_string();
    };

    let mut lines = Vec::new();
    lines.push(format!("Target: {}", info.label));
    lines.push(format!(
        "Position: ({:.0}, {:.0})",
        info.position.x, info.position.y
    ));
    lines.push(format!("Distance: {:.0}", info.distance));

    lines.join("\n")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_intel_empty_when_no_selection() {
        let result = format_intel_panel(None);
        assert!(result.contains("No target"));
    }

    #[test]
    fn format_intel_shows_entity_details() {
        let entity = Entity::from_bits(42);
        let info = IntelInfo {
            entity,
            label: "Ore Node".to_string(),
            position: Vec2::new(100.0, 200.0),
            distance: 150.0,
        };
        let result = format_intel_panel(Some(&info));

        assert!(result.contains("Ore Node"));
        assert!(result.contains("100")); // X position
        assert!(result.contains("200")); // Y position
        assert!(result.contains("150")); // Distance
    }
}
