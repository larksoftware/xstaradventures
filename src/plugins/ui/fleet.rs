//! Fleet panel systems.

use bevy::prelude::*;
use std::path::Path;

use crate::compat::{TextBundle, TextStyle};
use crate::fleets::{RiskTolerance, ScoutBehavior, ScoutPhase};

use super::components::{
    contact_item_color, FleetDetailDivider, FleetDetailText, FleetEmptyText, FleetItem,
    FleetListContainer, FleetPanelMarker, SelectedFleetUnit,
};

// =============================================================================
// Systems
// =============================================================================

pub fn update_fleet_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scouts: Query<&ScoutBehavior>,
    selected: Res<SelectedFleetUnit>,
    container_query: Query<Entity, With<FleetListContainer>>,
    existing_items: Query<Entity, With<FleetItem>>,
    empty_text: Query<Entity, With<FleetEmptyText>>,
) {
    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);
    if !font_on_disk.exists() {
        return;
    }
    let font = asset_server.load(font_path);

    // Get the container
    let Ok(container_entity) = container_query.single() else {
        return;
    };

    // Despawn existing items and empty text
    for item_entity in existing_items.iter() {
        commands.entity(item_entity).despawn();
    }
    for empty_entity in empty_text.iter() {
        commands.entity(empty_entity).despawn();
    }

    // Collect scout data
    let scout_data: Vec<_> = scouts.iter().collect();

    // Spawn new items
    commands.entity(container_entity).with_children(|parent| {
        if scout_data.is_empty() {
            parent.spawn((
                FleetEmptyText,
                TextBundle::from_section(
                    "(no units)",
                    TextStyle {
                        font: font.clone(),
                        font_size: 12.0,
                        color: Color::srgb(0.4, 0.6, 0.6),
                    },
                ),
            ));
        } else {
            for (index, scout) in scout_data.iter().enumerate() {
                let phase_short = match scout.phase {
                    ScoutPhase::Scanning => "Scan",
                    ScoutPhase::Investigating => "Invest",
                    ScoutPhase::ZoneComplete => "Ready",
                    ScoutPhase::TravelingToGate => "Travel",
                    ScoutPhase::Jumping => "Jump",
                    ScoutPhase::Complete => "Done",
                };

                let text_content = format!(
                    "Scout-{}  Z{}  {}",
                    index + 1,
                    scout.current_zone,
                    phase_short
                );

                let is_selected = selected.index == Some(index);
                let color = contact_item_color(is_selected, false);

                parent.spawn((
                    FleetItem { index },
                    Interaction::None,
                    TextBundle::from_section(
                        text_content,
                        TextStyle {
                            font: font.clone(),
                            font_size: 12.0,
                            color,
                        },
                    ),
                ));
            }
        }
    });
}

pub fn handle_fleet_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    mut selected: ResMut<SelectedFleetUnit>,
    items: Query<(&Interaction, &FleetItem)>,
    panel: Query<&Interaction, With<FleetPanelMarker>>,
) {
    // Check if any fleet item was clicked
    for (interaction, fleet_item) in items.iter() {
        if matches!(interaction, Interaction::Pressed) {
            // Toggle selection
            if selected.index == Some(fleet_item.index) {
                selected.index = None;
            } else {
                selected.index = Some(fleet_item.index);
            }
            return;
        }
    }

    // If mouse was just pressed and we didn't click a fleet item, check if we're outside panel
    if mouse.just_pressed(MouseButton::Left) {
        let panel_hovered = panel
            .iter()
            .any(|i| matches!(i, Interaction::Hovered | Interaction::Pressed));

        if !panel_hovered {
            selected.index = None;
        }
    }
}

pub fn update_fleet_item_styles(
    selected: Res<SelectedFleetUnit>,
    mut items: Query<(&Interaction, &FleetItem, &mut TextColor)>,
) {
    for (interaction, fleet_item, mut text_color) in items.iter_mut() {
        let is_selected = selected.index == Some(fleet_item.index);
        let is_hovered = matches!(interaction, Interaction::Hovered);
        text_color.0 = contact_item_color(is_selected, is_hovered);
    }
}

pub fn update_fleet_detail(
    scouts: Query<&ScoutBehavior>,
    selected: Res<SelectedFleetUnit>,
    mut detail_text: Query<&mut Text, With<FleetDetailText>>,
    mut divider: Query<&mut Visibility, With<FleetDetailDivider>>,
) {
    let mut text = match detail_text.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };

    let mut divider_vis = match divider.single_mut() {
        Ok(v) => v,
        Err(_) => return,
    };

    let Some(selected_index) = selected.index else {
        text.0 = String::new();
        *divider_vis = Visibility::Hidden;
        return;
    };

    // Find the scout at the selected index
    let scout_data: Vec<_> = scouts.iter().collect();
    let Some(scout) = scout_data.get(selected_index) else {
        text.0 = String::new();
        *divider_vis = Visibility::Hidden;
        return;
    };

    // Show the divider when we have detail to show
    *divider_vis = Visibility::Inherited;

    let risk_label = match scout.risk {
        RiskTolerance::Cautious => "Cautious",
        RiskTolerance::Balanced => "Balanced",
        RiskTolerance::Bold => "Bold",
    };

    let phase_label = match scout.phase {
        ScoutPhase::Scanning => "Scanning area",
        ScoutPhase::Investigating => "Investigating contacts",
        ScoutPhase::ZoneComplete => "Zone complete",
        ScoutPhase::TravelingToGate => "En route to gate",
        ScoutPhase::Jumping => "Jumping...",
        ScoutPhase::Complete => "Exploration complete",
    };

    let mut lines = Vec::new();
    lines.push(format!("Risk: {}", risk_label));
    lines.push(format!("Status: {}", phase_label));
    lines.push(format!("Gates queued: {}", scout.gates_to_explore.len()));
    lines.push(format!("Zones visited: {}", scout.visited_zones.len()));

    text.0 = lines.join("\n");
}

#[allow(deprecated)]
pub fn handle_panel_scroll(
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut scrollable: Query<(&Interaction, &mut ScrollPosition)>,
) {
    for event in scroll_events.read() {
        for (interaction, mut scroll_pos) in scrollable.iter_mut() {
            if matches!(interaction, Interaction::Hovered) {
                scroll_pos.y -= event.y * 20.0;
                scroll_pos.y = scroll_pos.y.max(0.0);
            }
        }
    }
}
