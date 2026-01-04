//! Log panel update system.

use bevy::prelude::*;

use crate::plugins::core::EventLog;

use super::components::LogContentText;

// =============================================================================
// Systems
// =============================================================================

pub fn update_log_panel(log: Res<EventLog>, mut log_text: Query<&mut Text, With<LogContentText>>) {
    if let Some(mut text) = log_text.iter_mut().next() {
        let entries = log.entries();
        if entries.is_empty() {
            text.0 = "Awaiting signal...".to_string();
        } else {
            let mut body = String::new();
            for entry in entries {
                body.push_str("› ");
                body.push_str(entry);
                body.push('\n');
            }
            text.0 = body.trim_end().to_string();
        }
    }
}
