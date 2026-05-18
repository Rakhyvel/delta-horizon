use std::collections::HashSet;

use hecs::{Entity, World};

use crate::{
    astro::state::State,
    components::{
        body::Parent,
        craft::{Craft, Landed},
        parts::PartDef,
    },
};

pub struct TechTree {
    pub unlocked: HashSet<String>,
    /// The player's starting body (Earth analog)
    home_planet: Entity,
    /// The star that all planets orbit
    sun: Entity,
}

impl TechTree {
    pub fn new(home_planet: Entity, sun: Entity) -> Self {
        Self {
            unlocked: HashSet::new(),
            home_planet,
            sun,
        }
    }

    pub fn is_available(&self, part: &PartDef) -> bool {
        match &part.requires {
            None => true,
            Some(req) => self.unlocked.contains(req),
        }
    }

    /// Returns true if this was a newly achieved unlock.
    pub fn unlock(&mut self, id: &str) -> bool {
        self.unlocked.insert(id.to_string())
    }

    /// Check state-based milestone conditions. Call once per turn after world state is updated.
    pub fn update(&mut self, world: &World) {
        self.check_sat_in_orbit(world);
        self.check_moon_orbit(world);
        self.check_moon_landing(world);
        self.check_planet_orbit(world);
        self.check_planet_landing(world);
    }

    /// Called from handle_event when a SoiChange fires. Detects flybys.
    pub fn on_soi_change(&mut self, new_parent: Entity, world: &World) {
        if let Ok(body_parent) = world.get::<&Parent>(new_parent) {
            if body_parent.id == self.home_planet {
                self.unlock("moon-flyby");
            } else if body_parent.id == self.sun && new_parent != self.home_planet {
                self.unlock("planet-flyby");
            }
        }
    }

    fn check_sat_in_orbit(&mut self, world: &World) {
        if self.unlocked.contains("first-sat") {
            return;
        }
        for (_, (_, parent, _)) in world.query::<(&Craft, &Parent, &State)>().iter() {
            if parent.id == self.home_planet {
                self.unlock("first-sat");
                return;
            }
        }
    }

    fn check_moon_orbit(&mut self, world: &World) {
        if self.unlocked.contains("moon-orbit") {
            return;
        }
        for (_, (_, parent, _)) in world.query::<(&Craft, &Parent, &State)>().iter() {
            if self.is_moon_of_home(parent.id, world) {
                self.unlock("moon-orbit");
                return;
            }
        }
    }

    fn check_moon_landing(&mut self, world: &World) {
        if self.unlocked.contains("moon-landing") {
            return;
        }
        for (_, (_, parent, _)) in world.query::<(&Craft, &Parent, &Landed)>().iter() {
            if self.is_moon_of_home(parent.id, world) {
                self.unlock("moon-landing");
                return;
            }
        }
    }

    fn check_planet_orbit(&mut self, world: &World) {
        if self.unlocked.contains("planet-orbit") {
            return;
        }
        for (_, (_, parent, _)) in world.query::<(&Craft, &Parent, &State)>().iter() {
            if self.is_other_planet(parent.id, world) {
                self.unlock("planet-orbit");
                return;
            }
        }
    }

    fn check_planet_landing(&mut self, world: &World) {
        if self.unlocked.contains("planet-landing") {
            return;
        }
        for (_, (_, parent, _)) in world.query::<(&Craft, &Parent, &Landed)>().iter() {
            if self.is_other_planet(parent.id, world) {
                self.unlock("planet-landing");
                return;
            }
        }
    }

    /// True if `body` is a body that orbits the home planet (i.e. a moon of home).
    fn is_moon_of_home(&self, body: Entity, world: &World) -> bool {
        world
            .get::<&Parent>(body)
            .map(|p| p.id == self.home_planet)
            .unwrap_or(false)
    }

    /// True if `body` orbits the sun and is not the home planet.
    fn is_other_planet(&self, body: Entity, world: &World) -> bool {
        if body == self.home_planet {
            return false;
        }
        world
            .get::<&Parent>(body)
            .map(|p| p.id == self.sun)
            .unwrap_or(false)
    }
}
