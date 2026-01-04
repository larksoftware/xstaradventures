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

pub fn spawn_starfield(mut commands: Commands) {
    let mut rng_state = 42u64; // Simple LCG seed

    // Helper function to generate pseudo-random values
    let mut next_random = || -> f32 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let value = (rng_state >> 33) as u32;
        (value as f32) / (u32::MAX as f32)
    };

    // Spawn stars across entire game world area
    // Game world is -600 to 600, -360 to 360
    // Spawn in much wider area to ensure coverage everywhere
    let min_x = -1200.0;
    let max_x = 1200.0;
    let min_y = -700.0;
    let max_y = 700.0;

    // Layer 1: Distant stars (smallest, dimmest, slowest parallax)
    for _ in 0..200 {
        let x = min_x + next_random() * (max_x - min_x);
        let y = min_y + next_random() * (max_y - min_y);
        let brightness = 0.3 + next_random() * 0.3; // 0.3-0.6
        let size = 1.0 + next_random() * 1.0; // 1.0-2.0 pixels

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

    // Layer 2: Mid-distance stars (medium size, medium brightness)
    for _ in 0..150 {
        let x = min_x + next_random() * (max_x - min_x);
        let y = min_y + next_random() * (max_y - min_y);
        let brightness = 0.5 + next_random() * 0.4; // 0.5-0.9
        let size = 1.5 + next_random() * 1.5; // 1.5-3.0 pixels

        // Vary color slightly (white to light blue)
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

    // Layer 3: Close stars (larger, brighter, more parallax)
    for _ in 0..80 {
        let x = min_x + next_random() * (max_x - min_x);
        let y = min_y + next_random() * (max_y - min_y);
        let brightness = 0.7 + next_random() * 0.3; // 0.7-1.0
        let size = 2.0 + next_random() * 2.0; // 2.0-4.0 pixels

        // Some stars have color variation (blue, yellow-white)
        let color_type = next_random();
        let color = if color_type < 0.7 {
            // White/blue-white
            Color::srgba(brightness, brightness, brightness * 1.2, 1.0)
        } else {
            // Yellow-white
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
    camera: Query<&Transform, With<Camera2d>>,
    mut stars: Query<&mut Transform, (With<Starfield>, Without<Camera2d>)>,
) {
    let camera_transform = match camera.iter().next() {
        Some(transform) => transform,
        None => return,
    };

    let camera_x = camera_transform.translation.x;
    let camera_y = camera_transform.translation.y;

    // Wrap distance - teleport stars when they get this far from camera
    // Viewport is ~768x432 at scale 0.6, so wrap at ~1.5x viewport to ensure coverage
    let wrap_x = 600.0;
    let wrap_y = 350.0;
    // Tile size should be 2x wrap distance for seamless wrapping
    let tile_x = 1200.0;
    let tile_y = 700.0;

    for mut star_transform in stars.iter_mut() {
        let dx = star_transform.translation.x - camera_x;
        let dy = star_transform.translation.y - camera_y;

        // If star is too far left, move it to the right
        if dx < -wrap_x {
            star_transform.translation.x += tile_x;
        }
        // If star is too far right, move it to the left
        else if dx > wrap_x {
            star_transform.translation.x -= tile_x;
        }

        // If star is too far down, move it up
        if dy < -wrap_y {
            star_transform.translation.y += tile_y;
        }
        // If star is too far up, move it down
        else if dy > wrap_y {
            star_transform.translation.y -= tile_y;
        }
    }
}
