use std::f64::consts::PI;

use rand::{seq::SliceRandom, Rng, SeedableRng};

use crate::{
    astro::{
        epoch::EphemerisTime,
        state::State,
        units::{EARTH_MASSES_PER_SUN_MASS, EARTH_RADII_PER_AU, G, SUN_MU},
    },
    components::body::{Body, Category},
};

const DENSITY_IRON_G_CM3: f64 = 12.0;
const DENSITY_ROCK_G_CM3: f64 = 3.5;

pub struct BodySystem {
    pub(crate) planet: (Body, State),
    pub(crate) moons: Vec<(Body, State)>,
}

struct MassCategory {
    #[allow(unused)]
    category: Category,
    range: (f64, f64),
    weight: f64,
}

const PLANET_MASS_CATEGORIES: &[MassCategory] = &[
    MassCategory {
        category: Category::Dwarf,
        range: (0.1, 0.3),
        weight: 3.0,
    },
    MassCategory {
        category: Category::SubEarth,
        range: (0.3, 0.8),
        weight: 3.0,
    },
    MassCategory {
        category: Category::EarthLike,
        range: (0.8, 1.5),
        weight: 8.0,
    },
    MassCategory {
        category: Category::SuperEarth,
        range: (1.5, 2.5),
        weight: 7.0,
    },
    MassCategory {
        category: Category::MiniNeptune,
        range: (2.5, 4.0),
        weight: 10.0,
    },
    MassCategory {
        category: Category::GasGiant,
        range: (4.0, 15.0),
        weight: 9.0,
    },
    MassCategory {
        category: Category::SuperGasGiant,
        range: (15.0, 20.0),
        weight: 3.0,
    },
];

const MOON_MASS_CATEGORIES: &[MassCategory] = &[
    MassCategory {
        category: Category::Dwarf,
        range: (0.1, 0.3),
        weight: 11.0,
    },
    MassCategory {
        category: Category::SubEarth,
        range: (0.3, 0.8),
        weight: 2.0,
    },
    MassCategory {
        category: Category::EarthLike,
        range: (0.8, 1.5),
        weight: 1.0,
    },
];

pub fn generate() -> Vec<BodySystem> {
    let mut rng = rand::rngs::StdRng::from_entropy();

    loop {
        let planets = generate_system(&mut rng);

        // if !has_habitable(&planets) {
        //     continue;
        // }
        if !all_moons_small(&planets) {
            continue;
        }
        // if !has_planet(&planets, &[Category::SubEarth, Category::EarthLike], 1) {
        //     continue;
        // }
        if !has_planet(&planets, &[Category::MiniNeptune, Category::GasGiant], 3) {
            continue;
        }
        // if !no_stripped(&planets) {
        //     continue;
        // }
        if planets.len() < 7 {
            continue;
        }
        break planets;
    }
}

fn generate_system(mut rng: &mut impl Rng) -> Vec<BodySystem> {
    let mut planets: Vec<BodySystem> = vec![];

    let mut orbital_radius_au = rng.gen_range(0.2..0.6); // in AU
    while orbital_radius_au < 35.0 {
        let orbital_radius_earth_radii = orbital_radius_au * EARTH_RADII_PER_AU;
        let planet = generate_planet(rng, orbital_radius_au, PLANET_MASS_CATEGORIES);
        let planet_inclination = (4.0_f64.to_radians()) * rng.gen::<f64>().powf(2.0);
        let planet_raan = rng.gen_range(0.0..0.2 * PI);
        let initial_state = State::from_kepler(
            orbital_radius_earth_radii,
            rng.gen_range(0.0..0.25),
            planet_inclination,
            planet_raan,
            rng.gen_range(0.0..2.0 * PI),
            rng.gen_range(0.0..2.0 * PI),
            EphemerisTime::new(0),
            SUN_MU,
        );

        let spacing = compute_spacing(rng, orbital_radius_au, planet.body_radius);
        orbital_radius_au += spacing;

        let roche_limit = 2.44 * planet.body_radius * (planet.density).powf(1.0 / 3.0);
        let hill_sphere = orbital_radius_earth_radii
            * (planet.mass() / (3.0 * EARTH_MASSES_PER_SUN_MASS)).powf(1.0 / 3.0);

        let mut moons = vec![];
        let max = max_moons(planet.body_radius);
        let mut moon_orbital_radius = rng.gen_range(2.5..20.0) * roche_limit;
        while moon_orbital_radius < hill_sphere && moons.len() < max {
            let moon = generate_planet(rng, orbital_radius_au, MOON_MASS_CATEGORIES);
            let moon_initial_state = State::from_kepler(
                moon_orbital_radius,
                rng.gen_range(0.0..0.25),
                (4.0_f64.to_radians()) * rng.gen::<f64>().powf(2.0) + planet_inclination,
                planet_raan,
                rng.gen_range(0.0..2.0 * PI),
                rng.gen_range(0.0..2.0 * PI),
                EphemerisTime::new(0),
                planet.mu,
            );
            moons.push((moon, moon_initial_state));
            let resonances: Vec<f64> = (2..100)
                .flat_map(|p| (2..100).map(move |q| (p, q)))
                .map(|(p, q)| (p as f64 / q as f64).powf(2.0 / 3.0))
                .filter(|ratio| (1.5..4.0).contains(ratio))
                .collect();
            let choice = resonances.choose(&mut rng).unwrap();
            moon_orbital_radius *= choice;
        }

        planets.push(BodySystem {
            planet: (planet, initial_state),
            moons,
        });
    }

    planets
}

fn generate_planet(rng: &mut impl Rng, dist_from_sun: f64, category_dist: &[MassCategory]) -> Body {
    let body_radius = sample_radius_with_au(rng, category_dist);
    let core_mass_fraction = sample_core_mass_fraction(rng, body_radius, dist_from_sun);
    let density = estimate_density(core_mass_fraction, body_radius);
    let rotation_period_hours = sample_rotation_period_hours(rng, body_radius);
    let category = categorize_planet(body_radius);
    let magnetic_field: bool =
        has_magnetic_field(body_radius, core_mass_fraction, rotation_period_hours);
    let atmos_pressure: f64 =
        sample_atmos_pressure(rng, magnetic_field, body_radius, dist_from_sun);
    let temperature = calculate_temperature(dist_from_sun, atmos_pressure);

    pub fn mass(density: f64, body_radius: f64) -> f64 {
        let earth_density = 5.51;
        (density / earth_density) * body_radius.powi(3)
    }
    let mu = G * mass(density, body_radius);

    Body {
        category,
        body_radius,
        rotation_period_hours,
        rotation: 0.0,
        temperature,
        atmos_pressure,
        core_mass_fraction,
        magnetic_field,
        density,
        mu,
    }
}

fn max_moons(body_radius: f64) -> usize {
    (-4.0 * 0.7f64.powf(body_radius) + 4.0) as usize
}

fn has_habitable(planets: &[BodySystem]) -> bool {
    planets.iter().any(|p| {
        p.planet.0.habitable() && p.planet.0.category == Category::EarthLike && !p.moons.is_empty()
    })
}

fn has_planet(planets: &[BodySystem], categories: &[Category], thresh: usize) -> bool {
    let count = planets
        .iter()
        .filter(|p| categories.contains(&p.planet.0.category))
        .count();
    count >= thresh
}

fn all_moons_small(planets: &Vec<BodySystem>) -> bool {
    for system in planets {
        let planet_mass = system.planet.0.mass();
        for moon in &system.moons {
            let moon_mass = moon.0.mass();
            if moon_mass / planet_mass > 0.012 {
                return false;
            }
        }
    }
    true
}

fn no_stripped(planets: &[BodySystem]) -> bool {
    planets
        .iter()
        .all(|p| !p.planet.0.is_giant() || p.planet.0.gaseous())
}

fn sample_rotation_period_hours(rng: &mut impl Rng, body_radius: f64) -> f64 {
    fn lerp(a: f64, b: f64, t: f64) -> f64 {
        a + (b - a) * t
    }

    // Make is to bigger bodies have a faster rotation
    let r = body_radius.clamp(0.1, 15.0);
    let max_hours = lerp(200.0, 15.0, ((r - 1.0) / 14.0).clamp(0.0, 1.0));

    let min = 5.0_f64.ln();
    let max = max_hours.ln();

    rng.gen_range(min..max).exp()
}

fn sample_core_mass_fraction(rng: &mut impl Rng, body_radius: f64, orbital_radius_au: f64) -> f64 {
    if body_radius > 2.5 {
        return 0.0;
    }

    let base = body_radius * 0.830169 * (0.361935f64).powf(orbital_radius_au);
    let variation = rng.gen_range(-0.05..0.05);
    (base + variation).clamp(0.05, 0.7)
}

fn estimate_density(core_mass_fraction: f64, body_radius: f64) -> f64 {
    // mix core + mantle
    let base_density =
        core_mass_fraction * DENSITY_IRON_G_CM3 + (1.0 - core_mass_fraction) * DENSITY_ROCK_G_CM3;

    let compression = if body_radius < 1.0 {
        1.0
    } else {
        1.0 / body_radius + 0.6 / DENSITY_ROCK_G_CM3 // gas giants get puffy as their radius increases
    };

    base_density * compression
}

fn has_magnetic_field(
    body_radius: f64,
    core_mass_fraction: f64,
    rotation_period_hours: f64,
) -> bool {
    body_radius > 2.5 || (core_mass_fraction > 0.2 && rotation_period_hours < 100.0)
}

fn sample_radius_with_au(rng: &mut impl Rng, category_dist: &[MassCategory]) -> f64 {
    // Compute cumulative weights
    let total_weight: f64 = category_dist.iter().map(|c| c.weight).sum();
    let mut roll = rng.gen_range(0.0..total_weight);

    let mut radius = 0.0;
    for cat in category_dist {
        if roll <= cat.weight {
            radius = rng.gen_range(cat.range.0..cat.range.1);
            break;
        }
        roll -= cat.weight;
    }

    radius
}

fn sample_atmos_pressure(
    rng: &mut impl Rng,
    magnetic_field: bool,
    body_radius: f64,
    orbital_radius_au: f64,
) -> f64 {
    // volatiles available
    let volatile_factor = match orbital_radius_au {
        r if r < 0.5 => 0.1, // almost none
        r if r < 1.5 => 0.5, // some
        r if r < 3.5 => 1.0, // decent
        _ => 1.5,            // lots of ices
    };

    // gravity retention
    let gravity_factor = if body_radius > 1.0 {
        1.0 // stronger scaling for big planets
    } else {
        body_radius.powf(4.0) // small planets struggle
    };

    // magnetic field prevents photoionization
    let magnetic_factor = if magnetic_field { 1.5 } else { 0.5 };

    // distance helps slightly (inverse square law for photoionization)
    let distance_factor = 1.0 + orbital_radius_au.powf(0.25);

    volatile_factor * gravity_factor * magnetic_factor * distance_factor * rng.gen_range(0.5..1.0)
}

fn calculate_temperature(orbital_radius_au: f64, atmos_pressure: f64) -> f64 {
    let inv_greenhouse = 1.51 / (atmos_pressure + 1.51);
    278.6 * ((1.0 - 0.3) / (orbital_radius_au.powf(2.0) * inv_greenhouse)).powf(0.25)
}

fn compute_spacing(rng: &mut impl Rng, orbital_radius_au: f64, radius: f64) -> f64 {
    let base = rng.gen_range(1.1..1.4);
    let radius_boost = 1.0 + 0.1 * radius.powf(0.25);

    orbital_radius_au * base * radius_boost
}

fn categorize_planet(radius: f64) -> Category {
    match radius {
        (0.0..0.1) => Category::Dwarf,
        (0.1..0.8) => Category::SubEarth,
        (0.8..1.5) => Category::EarthLike,
        (1.5..2.5) => Category::SuperEarth,
        (2.5..4.0) => Category::MiniNeptune,
        (4.0..15.0) => Category::GasGiant,
        (15.0..20.0) => Category::SuperGasGiant,
        _ => Category::Star,
    }
}
