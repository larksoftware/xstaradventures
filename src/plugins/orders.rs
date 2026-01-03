use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::plugins::core::SimConfig;
use crate::plugins::sim::SimTickCount;

pub struct OrdersPlugin;

impl Plugin for OrdersPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CommandEvent>()
            .add_message::<OrderAppliedEvent>()
            .init_resource::<OrderQueue>()
            .add_systems(Update, queue_commands)
            .add_systems(
                FixedUpdate,
                (emit_sample_command, apply_orders, log_applied_orders).run_if(sim_not_paused),
            );
    }
}

#[derive(Message)]
pub struct CommandEvent {
    pub kind: CommandKind,
}

#[derive(Clone, Copy, Debug)]
pub enum CommandKind {
    Noop,
}

#[derive(Message)]
pub struct OrderAppliedEvent {
    pub kind: OrderKind,
}

#[derive(Clone, Copy, Debug)]
pub enum OrderKind {
    Noop,
}

#[derive(Resource, Default)]
pub struct OrderQueue {
    pub pending: Vec<OrderKind>,
}

impl OrderQueue {
    fn push(&mut self, order: OrderKind) {
        self.pending.push(order);
    }
}

fn queue_commands(mut commands: MessageReader<CommandEvent>, mut queue: ResMut<OrderQueue>) {
    for command in commands.read() {
        let order = match command.kind {
            CommandKind::Noop => OrderKind::Noop,
        };
        queue.push(order);
    }
}

fn apply_orders(mut queue: ResMut<OrderQueue>, mut applied: MessageWriter<OrderAppliedEvent>) {
    for order in queue.pending.drain(..) {
        applied.write(OrderAppliedEvent { kind: order });
    }
}

fn emit_sample_command(ticks: Res<SimTickCount>, mut commands: MessageWriter<CommandEvent>) {
    if ticks.tick % 30 == 0 {
        commands.write(CommandEvent {
            kind: CommandKind::Noop,
        });
    }
}

fn sim_not_paused(config: Res<SimConfig>) -> bool {
    !config.paused
}

fn log_applied_orders(mut applied: MessageReader<OrderAppliedEvent>) {
    for event in applied.read() {
        info!("Order applied: {:?}", event.kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    #[test]
    fn sim_not_paused_returns_false_when_paused() {
        let mut world = World::default();
        world.insert_resource(SimConfig {
            tick_hz: 10.0,
            paused: true,
        });

        let mut system_state: SystemState<Res<SimConfig>> = SystemState::new(&mut world);
        let config = system_state.get(&world);
        let allowed = sim_not_paused(config);
        assert!(!allowed);
    }
}
