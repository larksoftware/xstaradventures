use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct PirateBase {
    pub launch_interval_ticks: u64,
    pub next_launch_tick: u64,
}

#[derive(Component, Debug)]
pub struct PirateShip {
    pub speed: f32,
}

pub fn schedule_next_launch(current_tick: u64, interval: u64) -> u64 {
    current_tick.saturating_add(interval)
}

#[cfg(test)]
mod tests {
    use super::schedule_next_launch;

    #[test]
    fn schedule_next_launch_advances_by_interval() {
        let next = schedule_next_launch(10, 25);
        assert_eq!(next, 35);
    }
}
