use apricot::high_precision::WorldPosition;
use hecs::{Entity, World};

use crate::{
    astro::{epoch::EphemerisTime, units::EARTH_RADII_PER_AU},
    components::body::Parent,
};

pub struct Station {
    /// The charge at `charge_et`, in [0..=capacity_kwh]
    pub charge_kwh: f32,
    /// The max charge capacity this station has
    pub capacity_kwh: f32,
    /// When `charge_kwh` was committed
    pub charge_et: EphemerisTime,

    pub num_crew: usize,

    /// bumped whenever a module is added or removed
    pub modules_gen: u32,
}

pub fn station_r_au(world: &World, station: Entity) -> f64 {
    let Ok(pos) = world.get::<&WorldPosition>(station) else {
        return 0.0;
    };
    pos.pos.magnitude() / EARTH_RADII_PER_AU // sun at origin
}

/// Get the net power in kW
pub fn station_net_kw(world: &World, station: Entity) -> f32 {
    let r_au = station_r_au(world, station);

    let mut kw = 0.0;

    // Sum up all the generators
    for (_, (_, parent, panel)) in world
        .query::<(&StationModule, &Parent, &SolarPanel)>()
        .iter()
    {
        if parent.id == station {
            kw += panel.output_kw(r_au);
        }
    }

    // TODO: subtract consumers

    kw
}

pub fn station_charge_at(world: &World, station: Entity, t: EphemerisTime) -> f32 {
    let Ok(s) = world.get::<&Station>(station) else {
        return 0.0;
    };
    let dt_hours = (t - s.charge_et).as_hours() as f32;
    (s.charge_kwh + station_net_kw(world, station) * dt_hours).clamp(0.0, s.capacity_kwh)
}

/// Gets the total station-wide mass flow for a resource
pub fn station_resource_mass_flow(world: &World, station: Entity, r: Resource) -> f32 {
    let Ok(s) = world.get::<&Station>(station) else {
        return 0.0;
    };

    // Accumulate producers of a resource
    const O2_PER_CREW_DAY: f32 = -0.84;
    const WATER_PER_CREW_DAY: f32 = -3.5;
    match r {
        Resource::Water => s.num_crew as f32 * WATER_PER_CREW_DAY,
        Resource::Oxygen => s.num_crew as f32 * O2_PER_CREW_DAY,
        Resource::Hydrogen => 0.0,
    }

    // TODO: Decumulate consumers of a resource
}

/// Interpolates the mass for a specific tank
pub fn tank_resource_mass(world: &World, module: Entity, t: EphemerisTime) -> f32 {
    let station = world.get::<&Parent>(module).unwrap().id;
    let Ok(tank) = world.get::<&Tank>(module) else {
        return 0.0;
    };
    let dt_hours = (t - tank.mass_et).as_days() as f32;
    (tank.mass_kg + station_resource_mass_flow(world, station, tank.resource) * dt_hours)
        .clamp(0.0, tank.capacity_kg)
}

/// Joins a module to a station
pub struct StationModule {
    pub slot: u32,
}

pub struct SolarPanel {
    /// How much power the panels produce at 1 AU
    pub rated_kw: f32,
}

impl SolarPanel {
    pub fn output_kw(&self, r_au: f64) -> f32 {
        self.rated_kw / (r_au * r_au) as f32
    }
}

// TODO: This doens't belong here!
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    Water,
    Oxygen,
    Hydrogen,
}

impl Resource {
    pub fn long_name(&self) -> &'static str {
        match self {
            Resource::Water => "Water",
            Resource::Oxygen => "Oxygen",
            Resource::Hydrogen => "Hydrogen",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Resource::Water => "H2O",
            Resource::Oxygen => "O2",
            Resource::Hydrogen => "H2",
        }
    }
}

pub struct Tank {
    /// The wet mass at `mass_et`, in [0..=capacity_kg]
    pub mass_kg: f32,
    /// The max mass capacity this tank module has
    pub capacity_kg: f32,
    /// When `mass_kg` was committed
    pub mass_et: EphemerisTime,
    /// What's in the tank
    pub resource: Resource,
}
