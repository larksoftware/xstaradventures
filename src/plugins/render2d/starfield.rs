//! Starfield background rendering.

use bevy::prelude::*;

use crate::compat::SpriteBundle;
use crate::plugins::core::ViewMode;

// =============================================================================
// Components
// =============================================================================

#[derive(Component)]
pub struct Starfield {
    #[allow(dead_code)]
    pub layer: u8,
}

// =============================================================================
// Systems
// =============================================================================

pub fn spawn_starfield(
    mut commands: Commands,
    player: Query<&Transform, With<crate::plugins::player::PlayerControl>>,
) {
    // Get player position to spawn stars around it, or use origin as fallback
    let (center_x, center_y) = player
        .iter()
        .next()
        .map(|t| (t.translation.x, t.translation.y))
        .unwrap_or((0.0, 0.0));

    let mut rng_state = 42u64; // Simple LCG seed

    // Helper function to generate pseudo-random values in [0, 1)
    let mut next_random = || -> f32 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        // Use top 32 bits for full [0, 1) range
        let value = (rng_state >> 32) as u32;
        (value as f32) / (u32::MAX as f32)
    };

    // Tile size must be slightly larger than the viewport so stars wrap
    // just as they leave the screen. At 4K (3840x2160) with scale 0.6,
    // viewport is ~2304x1296. Using ~1.5x viewport for tile.
    let half_tile_x = 1800.0; // tile_x = 3600
    let half_tile_y = 1100.0; // tile_y = 2200

    // Layer 1: Distant stars (smallest, dimmest)
    for _ in 0..200 {
        let x = center_x + (-half_tile_x + next_random() * (half_tile_x * 2.0));
        let y = center_y + (-half_tile_y + next_random() * (half_tile_y * 2.0));
        let brightness = 0.3 + next_random() * 0.3;
        let size = 1.0 + next_random() * 1.0;

        commands.spawn((
            Starfield { layer: 1 },
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(brightness, brightness, brightness * 1.1, 1.0),
                    custom_size: Some(Vec2::new(size, size)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, -10.0),
                ..default()
            },
            Name::new("Star-Distant"),
        ));
    }

    // Layer 2: Mid-distance stars
    for _ in 0..150 {
        let x = center_x + (-half_tile_x + next_random() * (half_tile_x * 2.0));
        let y = center_y + (-half_tile_y + next_random() * (half_tile_y * 2.0));
        let brightness = 0.5 + next_random() * 0.4;
        let size = 1.5 + next_random() * 1.5;
        let blue_tint = next_random() * 0.2;
        let color = Color::srgba(brightness, brightness, brightness + blue_tint, 1.0);

        commands.spawn((
            Starfield { layer: 2 },
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::new(size, size)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, -9.0),
                ..default()
            },
            Name::new("Star-Mid"),
        ));
    }

    // Layer 3: Close stars (larger, brighter)
    for _ in 0..80 {
        let x = center_x + (-half_tile_x + next_random() * (half_tile_x * 2.0));
        let y = center_y + (-half_tile_y + next_random() * (half_tile_y * 2.0));
        let brightness = 0.7 + next_random() * 0.3;
        let size = 2.0 + next_random() * 2.0;

        let color_type = next_random();
        let color = if color_type < 0.7 {
            Color::srgba(brightness, brightness, brightness * 1.2, 1.0)
        } else {
            Color::srgba(brightness, brightness * 0.95, brightness * 0.8, 1.0)
        };

        commands.spawn((
            Starfield { layer: 3 },
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::new(size, size)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, -8.0),
                ..default()
            },
            Name::new("Star-Close"),
        ));
    }

    info!("Starfield spawned with 430 stars across 3 layers");
}

pub fn toggle_starfield_visibility(
    view: Res<ViewMode>,
    mut stars: Query<&mut Visibility, With<Starfield>>,
) {
    let visible = matches!(*view, ViewMode::World);
    for mut visibility in stars.iter_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn wrap_starfield(
    player: Query<&Transform, With<crate::plugins::player::PlayerControl>>,
    mut stars: Query<
        &mut Transform,
        (
            With<Starfield>,
            Without<crate::plugins::player::PlayerControl>,
        ),
    >,
) {
    // Use player position directly - more reliable than camera which may lag
    let player_transform = match player.iter().next() {
        Some(p) => p,
        None => return,
    };

    let center_x = player_transform.translation.x;
    let center_y = player_transform.translation.y;

    // Tile size - must match spawn area for consistent density
    // Sized to be slightly larger than 4K viewport at scale 0.6 (~2304x1296)
    let tile_x = 3600.0;
    let tile_y = 2200.0;
    let half_tile_x = tile_x / 2.0;
    let half_tile_y = tile_y / 2.0;

    for mut star_transform in stars.iter_mut() {
        // Calculate offset from player
        let dx = star_transform.translation.x - center_x;
        let dy = star_transform.translation.y - center_y;

        // Use modulo to wrap into [-half_tile, half_tile] range
        // This guarantees stars stay within bounds regardless of distance traveled
        let wrapped_dx = ((dx % tile_x) + tile_x + half_tile_x) % tile_x - half_tile_x;
        let wrapped_dy = ((dy % tile_y) + tile_y + half_tile_y) % tile_y - half_tile_y;

        star_transform.translation.x = center_x + wrapped_dx;
        star_transform.translation.y = center_y + wrapped_dy;
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn starfield_module_compiles() {
        // Basic compile test
        assert!(true);
    }
}
