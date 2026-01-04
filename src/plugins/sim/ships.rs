//! Ship simulation systems.

use bevy::prelude::*;

use crate::plugins::core::EventLog;
use crate::ships::{ship_fuel_burn_per_minute, Ship, ShipFuelAlert, ShipState};

// =============================================================================
// Systems
// =============================================================================

pub fn ship_fuel_burn(time: Res<Time<Fixed>>, mut ships: Query<&mut Ship>) {
    let delta_seconds = time.delta_secs();
    let minutes = delta_seconds / 60.0;

    for mut ship in ships.iter_mut() {
        if matches!(ship.state, ShipState::Disabled) {
            continue;
        }

        let burn = ship_fuel_burn_per_minute(ship.kind) * minutes;
        if ship.fuel > burn {
            ship.fuel -= burn;
        } else {
            ship.fuel = 0.0;
        }
    }
}

pub fn ship_state_stub(mut ships: Query<&mut Ship>) {
    for mut ship in ships.iter_mut() {
        if ship.fuel <= 0.0 {
            ship.state = ShipState::Disabled;
            continue;
        }

        if ship.fuel_capacity > 0.0 {
            let ratio = ship.fuel / ship.fuel_capacity;
            if ratio <= 0.1 && !matches!(ship.state, ShipState::Returning) {
                ship.state = ShipState::Returning;
            }
        }
    }
}

pub fn ship_fuel_alerts(mut log: ResMut<EventLog>, mut alerts: Query<(&Ship, &mut ShipFuelAlert)>) {
    for (ship, mut alert) in alerts.iter_mut() {
        if ship.fuel_capacity <= 0.0 {
            continue;
        }

        let ratio = ship.fuel / ship.fuel_capacity;
        let low = ratio <= 0.25;
        let critical = ratio <= 0.10;

        if low && !alert.low {
            log.push(format!("Ship {:?} low fuel", ship.kind));
            alert.low = true;
        }

        if critical && !alert.critical {
            log.push(format!("Ship {:?} critical fuel", ship.kind));
            alert.critical = true;
        }

        if !low {
            alert.low = false;
        }

        if !critical {
            alert.critical = false;
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ships::ShipKind;
    use bevy::ecs::system::SystemState;

    #[test]
    fn ship_state_stub_disables_empty_fuel() {
        let mut world = World::default();
        world.spawn(Ship {
            kind: ShipKind::Scout,
            state: ShipState::Idle,
            fuel: 0.0,
            fuel_capacity: 30.0,
        });

        let mut system_state: SystemState<Query<&mut Ship>> = SystemState::new(&mut world);
        let ships = system_state.get_mut(&mut world);
        ship_state_stub(ships);
        system_state.apply(&mut world);

        let mut query = world.query::<&Ship>();
        for ship in query.iter(&world) {
            assert_eq!(ship.state, ShipState::Disabled);
        }
    }

    #[test]
    fn ship_fuel_alerts_logs_once_and_sets_flags() {
        let mut world = World::default();
        world.insert_resource(EventLog::default());
        world.spawn((
            Ship {
                kind: ShipKind::Scout,
                state: ShipState::Idle,
                fuel: 1.0,
                fuel_capacity: 20.0,
            },
            ShipFuelAlert::default(),
        ));

        let mut system_state: SystemState<(ResMut<EventLog>, Query<(&Ship, &mut ShipFuelAlert)>)> =
            SystemState::new(&mut world);
        {
            let (log, alerts) = system_state.get_mut(&mut world);
            ship_fuel_alerts(log, alerts);
        }
        system_state.apply(&mut world);

        let log = world.resource::<EventLog>();
        assert_eq!(log.entries().len(), 2);

        let mut system_state: SystemState<(ResMut<EventLog>, Query<(&Ship, &mut ShipFuelAlert)>)> =
            SystemState::new(&mut world);
        {
            let (log, alerts) = system_state.get_mut(&mut world);
            ship_fuel_alerts(log, alerts);
        }
        system_state.apply(&mut world);

        let log = world.resource::<EventLog>();
        assert_eq!(log.entries().len(), 2);

        let mut query = world.query::<&ShipFuelAlert>();
        for alert in query.iter(&world) {
            assert!(alert.low);
            assert!(alert.critical);
        }
    }
}
