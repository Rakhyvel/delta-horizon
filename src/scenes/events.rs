use std::collections::BTreeMap;

use hecs::Entity;

use crate::{
    components::orbit::Orbit,
    scenes::{astro::HohmannTransfer, epoch::EphemerisTime},
};

pub enum Event {
    SoiChange {
        craft: Entity,
        new_parent: Entity,
        flyby_orbit: Orbit,
        soi_radius: f64,
    },
    FlybyClosure {
        craft: Entity,
        body: Entity,
    },
    ManeuverReady {
        craft: Entity,
        to: Entity,
        transfer_orbit: Orbit,
        soi_radius: Option<f64>,
    },
    TakeOff {
        craft: Entity,
    },
    Land {
        craft: Entity,
    },
}

pub struct EventQueue {
    pub events: BTreeMap<EphemerisTime, Vec<Event>>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            events: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, time: EphemerisTime, event: Event) {
        self.events.entry(time).or_default().push(event);
    }

    /// Pop all events up to and including `current_time`
    pub fn pop_due(&mut self, current_time: EphemerisTime) -> Vec<Event> {
        let future = self
            .events
            .split_off(&(current_time + EphemerisTime::new(1)));
        std::mem::replace(&mut self.events, future)
            .into_values()
            .flatten()
            .collect()
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}
