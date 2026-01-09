//! Contacts panel systems.

use bevy::prelude::*;
use std::path::Path;

use crate::compat::TextBundle;
use crate::compat::TextStyle;
use crate::plugins::player::NearbyTargets;

use super::components::{
    contact_item_color, ContactItem, ContactsEmptyText, ContactsListContainer,
};

// =============================================================================
// Systems
// =============================================================================

pub fn update_tactical_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    targets: Res<NearbyTargets>,
    container_query: Query<(Entity, Option<&Children>), With<ContactsListContainer>>,
    existing_items: Query<Entity, With<ContactItem>>,
    existing_empty: Query<Entity, With<ContactsEmptyText>>,
) {
    // Always rebuild - list is small and needs to reflect current proximity sort

    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);
    if !font_on_disk.exists() {
        return;
    }
    let font = asset_server.load(font_path);

    // Get the container
    let Ok((container_entity, _)) = container_query.single() else {
        return;
    };

    // Despawn existing items and empty text
    for item_entity in existing_items.iter() {
        commands.entity(item_entity).despawn();
    }
    for empty_entity in existing_empty.iter() {
        commands.entity(empty_entity).despawn();
    }

    // Spawn new items
    commands.entity(container_entity).with_children(|parent| {
        if targets.entities.is_empty() {
            parent.spawn((
                ContactsEmptyText,
                TextBundle::from_section(
                    "(no contacts)",
                    TextStyle {
                        font: font.clone(),
                        font_size: 13.0,
                        color: Color::srgb(0.4, 0.6, 0.6),
                    },
                ),
            ));
        } else {
            for (index, (_, _, label)) in targets.entities.iter().enumerate() {
                let is_selected = index == targets.selected_index;
                let indicator = if is_selected { ">" } else { " " };
                let text_content = format!("{} {}", indicator, label);
                let color = contact_item_color(is_selected, false);

                parent.spawn((
                    ContactItem { index },
                    Interaction::None,
                    TextBundle::from_section(
                        text_content,
                        TextStyle {
                            font: font.clone(),
                            font_size: 13.0,
                            color,
                        },
                    ),
                ));
            }
        }
    });
}

pub fn handle_contact_clicks(
    mut targets: ResMut<NearbyTargets>,
    items: Query<(&Interaction, &ContactItem), Changed<Interaction>>,
) {
    for (interaction, contact_item) in items.iter() {
        if matches!(interaction, Interaction::Pressed) {
            targets.selected_index = contact_item.index;
            targets.manually_selected = true;
        }
    }
}

pub fn update_contact_item_styles(
    targets: Res<NearbyTargets>,
    mut items: Query<(&Interaction, &ContactItem, &mut TextColor)>,
) {
    for (interaction, contact_item, mut text_color) in items.iter_mut() {
        let is_selected = contact_item.index == targets.selected_index;
        let is_hovered = matches!(interaction, Interaction::Hovered);
        text_color.0 = contact_item_color(is_selected, is_hovered);
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Formats the contacts panel content from a list of targets.
#[allow(dead_code)]
pub fn format_contacts_panel(
    entities: &[(bevy::prelude::Entity, Vec2, String)],
    selected_index: usize,
) -> String {
    let mut lines = Vec::new();
    lines.push("Contacts".to_string());
    lines.push("--------".to_string());

    if entities.is_empty() {
        lines.push("(no contacts)".to_string());
    } else {
        for (index, (_, _, label)) in entities.iter().enumerate() {
            let indicator = if index == selected_index { ">" } else { " " };
            lines.push(format!("{} {}", indicator, label));
        }
    }

    lines.join("\n")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contacts_panel_empty_shows_no_contacts() {
        let entities: Vec<(Entity, Vec2, String)> = vec![];
        let result = format_contacts_panel(&entities, 0);

        assert!(result.contains("Contacts"));
        assert!(result.contains("--------"));
        assert!(result.contains("(no contacts)"));
    }

    #[test]
    fn contacts_panel_single_target_shows_selection() {
        let entity = Entity::from_bits(42);
        let entities = vec![(entity, Vec2::new(10.0, 20.0), "Ore Node".to_string())];
        let result = format_contacts_panel(&entities, 0);

        assert!(result.contains("Contacts"));
        assert!(result.contains("> Ore Node"));
    }

    #[test]
    fn contacts_panel_multiple_targets_shows_all() {
        let entities = vec![
            (
                Entity::from_bits(1),
                Vec2::new(10.0, 10.0),
                "Ore Node".to_string(),
            ),
            (
                Entity::from_bits(2),
                Vec2::new(20.0, 20.0),
                "Station-1".to_string(),
            ),
            (
                Entity::from_bits(3),
                Vec2::new(30.0, 30.0),
                "Pirate".to_string(),
            ),
        ];
        let result = format_contacts_panel(&entities, 1);

        assert!(result.contains("Contacts"));
        assert!(result.contains("  Ore Node")); // Not selected
        assert!(result.contains("> Station-1")); // Selected
        assert!(result.contains("  Pirate")); // Not selected
    }

    #[test]
    fn contacts_panel_selection_indicator_on_first() {
        let entities = vec![
            (
                Entity::from_bits(1),
                Vec2::new(10.0, 10.0),
                "Alpha".to_string(),
            ),
            (
                Entity::from_bits(2),
                Vec2::new(20.0, 20.0),
                "Beta".to_string(),
            ),
        ];
        let result = format_contacts_panel(&entities, 0);

        assert!(result.contains("> Alpha"));
        assert!(result.contains("  Beta"));
    }

    #[test]
    fn contacts_panel_selection_indicator_on_last() {
        let entities = vec![
            (
                Entity::from_bits(1),
                Vec2::new(10.0, 10.0),
                "Alpha".to_string(),
            ),
            (
                Entity::from_bits(2),
                Vec2::new(20.0, 20.0),
                "Beta".to_string(),
            ),
        ];
        let result = format_contacts_panel(&entities, 1);

        assert!(result.contains("  Alpha"));
        assert!(result.contains("> Beta"));
    }

    #[test]
    fn contact_item_color_default_when_not_selected_or_hovered() {
        let is_selected = false;
        let is_hovered = false;
        let color = contact_item_color(is_selected, is_hovered);
        // Default cyan color
        assert!(color.to_srgba().red < 0.1);
        assert!(color.to_srgba().green > 0.9);
        assert!(color.to_srgba().blue > 0.9);
    }

    #[test]
    fn contact_item_color_highlight_when_selected() {
        let is_selected = true;
        let is_hovered = false;
        let color = contact_item_color(is_selected, is_hovered);
        // Selected = brighter/white
        assert!(color.to_srgba().green > 0.9);
    }

    #[test]
    fn contact_item_color_hover_when_hovered_not_selected() {
        let is_selected = false;
        let is_hovered = true;
        let color = contact_item_color(is_selected, is_hovered);
        // Hovered = slightly brighter
        assert!(color.to_srgba().green > 0.7);
    }

    #[test]
    fn contact_item_contains_index() {
        let item = ContactItem { index: 5 };
        assert_eq!(item.index, 5);
    }
}
