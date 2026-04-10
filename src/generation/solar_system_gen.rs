use rand::{Rng, SeedableRng};

use crate::components::planet::{Category, Planet};

struct MassCategory {
    #[allow(unused)]
    category: Category,
    range: (f32, f32),
    weight: f32,
}

const MASS_CATEGORIES: &[MassCategory] = &[
    MassCategory {
        category: Category::Dwarf,
        range: (0.1, 0.3),
        weight: 11.0,
    },
    MassCategory {
        category: Category::SubEarth,
        range: (0.3, 0.8),
        weight: 9.0,
    },
    MassCategory {
        category: Category::EarthLike,
        range: (0.8, 1.5),
        weight: 7.0,
    },
    MassCategory {
        category: Category::SuperEarth,
        range: (1.5, 2.5),
        weight: 7.0,
    },
    MassCategory {
        category: Category::MiniNeptune,
        range: (2.5, 4.0),
        weight: 9.0,
    },
    MassCategory {
        category: Category::GasGiant,
        range: (4.0, 15.0),
        weight: 9.0,
    },
    MassCategory {
        category: Category::SuperGasGiant,
        range: (15.0, 20.0),
        weight: 7.0,
    },
];

pub fn generate() -> Vec<Planet> {
    let mut rng = rand::rngs::StdRng::from_entropy();
    let planets = loop {
        let planets = generate_system(&mut rng);
        // Need an earth starting point
        if !has_habitable(&planets) {
            continue;
        }
        // Need terrestrial
        if !has_planet(&planets, &[Category::SubEarth, Category::EarthLike], 2) {
            continue;
        }
        // Need gasses
        if !has_planet(&planets, &[Category::MiniNeptune, Category::GasGiant], 2) {
            continue;
        }
        // Need at least 7 planets
        if planets.len() < 7 {
            continue;
        }
        break planets;
    };

    planets
}

fn generate_system(rng: &mut impl Rng) -> Vec<Planet> {
    let mut planets: Vec<Planet> = vec![];

    let mut orbital_radius = rng.gen_range(0.2..0.6); // in AU
    while orbital_radius < 35.0 {
        let body_radius = sample_radius_with_au(rng);
        let core_mass_fraction = sample_core_mass_fraction(rng, body_radius, orbital_radius);
        let density = estimate_density(core_mass_fraction, body_radius);
        let rotation_period_hours = sample_rotation_period_hours(rng, body_radius);
        let category = categorize_planet(body_radius);
        let magnetic_field: bool =
            has_magnetic_field(body_radius, core_mass_fraction, rotation_period_hours);
        let atmos_pressure: f32 =
            sample_atmos_pressure(rng, magnetic_field, body_radius, orbital_radius);
        let temperature = calculate_temperature(orbital_radius, atmos_pressure);

        planets.push(Planet::new(
            1,
            body_radius,
            orbital_radius * 2000.0,
            orbital_radius,        // TODO: Calculate this from orbital radius
            rotation_period_hours, // TODO: Make tidally locked if close
            core_mass_fraction,
            atmos_pressure,
            temperature,
            format!(
                "{:?}\n({:.3} R🜨)\n({:.3} AU)\n({:.3} CMF)\n({:.3} g/cm^3)\n(Day: {:.3} hr)\n(Magnetic field? {})\n({:.3} bar)\n({:.3}K)",
                category,
                body_radius,
                orbital_radius,
                core_mass_fraction,
                density,
                rotation_period_hours,
                magnetic_field,
                atmos_pressure,
                temperature
            ),
            category,
        ));

        let spacing = compute_spacing(rng, orbital_radius, body_radius);
        orbital_radius += spacing;
    }

    planets
}

fn has_habitable(planets: &Vec<Planet>) -> bool {
    planets
        .iter()
        .any(|p| p.habitable() && p.category == Category::EarthLike)
}

fn has_planet(planets: &Vec<Planet>, categories: &[Category], thresh: usize) -> bool {
    let count = planets
        .iter()
        .filter(|p| categories.contains(&p.category))
        .count();
    count >= thresh
}

fn sample_rotation_period_hours(rng: &mut impl Rng, body_radius: f32) -> f32 {
    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    // Make is to bigger bodies have a faster rotation
    let r = body_radius.clamp(0.1, 15.0);
    let max_hours = lerp(200.0, 15.0, ((r - 1.0) / 14.0).clamp(0.0, 1.0));

    let min = 5.0_f32.ln();
    let max = max_hours.ln();

    rng.gen_range(min..max).exp()
}

fn sample_core_mass_fraction(rng: &mut impl Rng, body_radius: f32, orbital_radius: f32) -> f32 {
    if body_radius > 2.5 {
        return 0.0;
    }

    let base = body_radius * 0.830169 * (0.361935f32).powf(orbital_radius);
    let variation = rng.gen_range(-0.05..0.05);
    (base + variation).clamp(0.05, 0.7)
}

fn estimate_density(core_mass_fraction: f32, body_radius: f32) -> f32 {
    // base material densities (g/cm^3)
    let iron = 12.0;
    let rock = 3.5;

    // mix core + mantle
    let base_density = core_mass_fraction * iron + (1.0 - core_mass_fraction) * rock;

    let compression = if body_radius < 1.0 {
        1.0
    } else {
        1.0 / body_radius // gas giants get puffy as their radius increases
    };

    return base_density * compression;
}

fn has_magnetic_field(
    body_radius: f32,
    core_mass_fraction: f32,
    rotation_period_hours: f32,
) -> bool {
    body_radius > 2.5 || (core_mass_fraction > 0.2 && rotation_period_hours < 100.0)
}

fn sample_radius_with_au(rng: &mut impl Rng) -> f32 {
    // Compute cumulative weights
    let total_weight: f32 = MASS_CATEGORIES.iter().map(|c| c.weight).sum();
    let mut roll = rng.gen_range(0.0..total_weight);

    let mut radius = 0.0;
    for cat in MASS_CATEGORIES {
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
    body_radius: f32,
    orbital_radius: f32,
) -> f32 {
    // volatiles available
    let volatile_factor = match orbital_radius {
        r if r < 0.5 => 0.1, // almost none
        r if r < 1.5 => 0.5, // some
        r if r < 3.5 => 1.0, // decent
        _ => 1.5,            // lots of ices
    };

    // gravity retention
    let gravity_factor = if body_radius > 1.0 {
        body_radius.powf(1.5) // stronger scaling for big planets
    } else {
        body_radius.powf(4.0) // small planets struggle
    };

    // magnetic field prevents photoionization
    let magnetic_factor = if magnetic_field { 1.5 } else { 0.5 };

    // distance helps slightly (inverse square law for photoionization)
    let distance_factor = 1.0 + orbital_radius.powf(0.25);

    let pressure = volatile_factor
        * gravity_factor
        * magnetic_factor
        * distance_factor
        * rng.gen_range(0.5..1.0);

    pressure
}

fn calculate_temperature(orbital_radius: f32, atmos_pressure: f32) -> f32 {
    let greenhouse = 1.51 / (atmos_pressure + 1.51);
    278.6 * ((1.0 - 0.3) / (orbital_radius.powf(2.0) * greenhouse)).powf(0.25)
}

fn compute_spacing(rng: &mut impl Rng, orbital_radius: f32, radius: f32) -> f32 {
    let base = rng.gen_range(1.1..1.4);
    let radius_boost = 1.0 + 0.1 * radius.powf(0.25);

    orbital_radius * base * radius_boost
}

fn categorize_planet(radius: f32) -> Category {
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
