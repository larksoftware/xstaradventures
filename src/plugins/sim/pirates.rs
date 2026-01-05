//! Pirate AI systems.

use bevy::prelude::*;

use crate::compat::SpatialBundle;
use crate::factions::Faction;
use crate::pirates::{schedule_next_launch, PirateBase, PirateShip, PirateShipBehavior};
use crate::stations::{CrisisStage, CrisisType, Station, StationCrisis, StationKind};

use super::SimTickCount;

/// Duration in ticks that pirates dock at Outposts (about 5 seconds at 60 ticks/sec)
const PIRATE_DOCK_DURATION_TICKS: u64 = 300;

// =============================================================================
// Systems
// =============================================================================

pub fn pirate_launches(
    ticks: Res<SimTickCount>,
    mut commands: Commands,
    mut bases: Query<(&Transform, &mut PirateBase)>,
) {
    for (transform, mut base) in bases.iter_mut() {
        if ticks.tick < base.next_launch_tick {
            continue;
        }

        base.next_launch_tick = schedule_next_launch(ticks.tick, base.launch_interval_ticks);
        commands.spawn((
            PirateShip {
                speed: 70.0,
                behavior: PirateShipBehavior::default(),
            },
            Faction::Pirate,
            Name::new("Pirate-Ship"),
            SpatialBundle::from_transform(*transform),
        ));
    }
}

pub fn pirate_move(
    time: Res<Time<Fixed>>,
    stations: Query<&Transform, (With<Station>, Without<PirateShip>)>,
    mut pirates: Query<(&mut Transform, &PirateShip)>,
) {
    if stations.is_empty() {
        return;
    }

    let mut station_positions = Vec::new();
    for transform in stations.iter() {
        station_positions.push(Vec2::new(transform.translation.x, transform.translation.y));
    }

    let delta_seconds = time.delta_secs();

    for (mut transform, pirate) in pirates.iter_mut() {
        // Skip docked pirates - they don't move
        if matches!(pirate.behavior, PirateShipBehavior::DockedAtOutpost { .. }) {
            continue;
        }

        let pirate_pos = Vec2::new(transform.translation.x, transform.translation.y);
        let mut target = station_positions[0];
        let mut best_dist = pirate_pos.distance(target);

        for pos in &station_positions[1..] {
            let dist = pirate_pos.distance(*pos);
            if dist < best_dist {
                best_dist = dist;
                target = *pos;
            }
        }

        let direction = (target - pirate_pos).normalize_or_zero();
        let step = direction * pirate.speed * delta_seconds;
        transform.translation.x += step.x;
        transform.translation.y += step.y;
    }
}

pub fn pirate_harassment(
    mut commands: Commands,
    stations: Query<(Entity, &Transform, &Station)>,
    pirates: Query<&Transform, With<PirateShip>>,
    crises: Query<Option<&StationCrisis>>,
) {
    let range = 18.0;

    for (station_entity, station_transform, station) in stations.iter() {
        // Outposts are neutral - pirates dock there, not harass
        if station.kind == StationKind::Outpost {
            continue;
        }

        let station_pos = Vec2::new(
            station_transform.translation.x,
            station_transform.translation.y,
        );
        let mut count = 0u32;

        for pirate_transform in pirates.iter() {
            let pirate_pos = Vec2::new(
                pirate_transform.translation.x,
                pirate_transform.translation.y,
            );
            if pirate_pos.distance(station_pos) <= range {
                count += 1;
            }
        }

        if count > 0 {
            let stage = if count >= 2 {
                CrisisStage::Failing
            } else {
                CrisisStage::Strained
            };

            commands.entity(station_entity).insert(StationCrisis {
                crisis_type: CrisisType::PirateHarassment,
                stage,
            });
        } else if let Ok(Some(existing)) = crises.get(station_entity) {
            if matches!(existing.crisis_type, CrisisType::PirateHarassment) {
                commands.entity(station_entity).remove::<StationCrisis>();
            }
        }
    }
}

/// System to handle pirates docking at Outposts
pub fn pirate_outpost_docking(
    outposts: Query<(&Transform, &Station), Without<PirateShip>>,
    mut pirates: Query<(&Transform, &mut PirateShip)>,
) {
    let dock_range = 15.0;

    // Collect Outpost positions
    let outpost_positions: Vec<Vec2> = outposts
        .iter()
        .filter(|(_, station)| station.kind == StationKind::Outpost)
        .map(|(transform, _)| Vec2::new(transform.translation.x, transform.translation.y))
        .collect();

    if outpost_positions.is_empty() {
        return;
    }

    for (transform, mut pirate) in pirates.iter_mut() {
        // Skip already docked pirates
        if matches!(pirate.behavior, PirateShipBehavior::DockedAtOutpost { .. }) {
            continue;
        }

        let pirate_pos = Vec2::new(transform.translation.x, transform.translation.y);

        // Check if near any Outpost
        for outpost_pos in &outpost_positions {
            if pirate_pos.distance(*outpost_pos) <= dock_range {
                // Dock at the Outpost
                pirate.behavior = PirateShipBehavior::DockedAtOutpost {
                    ticks_remaining: PIRATE_DOCK_DURATION_TICKS,
                };
                break;
            }
        }
    }
}

/// System to update pirate docking timers
pub fn pirate_dock_timer(mut pirates: Query<&mut PirateShip>) {
    for mut pirate in pirates.iter_mut() {
        if let PirateShipBehavior::DockedAtOutpost { ticks_remaining } = &mut pirate.behavior {
            if *ticks_remaining > 0 {
                *ticks_remaining -= 1;
            } else {
                // Undock and resume roaming
                pirate.behavior = PirateShipBehavior::Roaming;
            }
        }
    }
}
