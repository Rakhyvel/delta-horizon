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
        range: (0.01, 0.1),
        weight: 11.0,
    },
    MassCategory {
        category: Category::SubEarth,
        range: (0.1, 0.8),
        weight: 9.0,
    },
    MassCategory {
        category: Category::EarthLike,
        range: (0.8, 1.5),
        weight: 7.0,
    },
    MassCategory {
        category: Category::SuperEarth,
        range: (1.5, 10.0),
        weight: 7.0,
    },
    MassCategory {
        category: Category::MiniNeptune,
        range: (10.0, 30.0),
        weight: 9.0,
    },
    MassCategory {
        category: Category::GasGiant,
        range: (30.0, 300.0),
        weight: 9.0,
    },
    MassCategory {
        category: Category::SuperGasGiant,
        range: (300.0, 1000.0),
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

        let category = categorize_planet(body_radius);
        let atmos_pressure: f32 = sample_atmos_pressure(rng, body_radius, orbital_radius);
        let temperature = calculate_temperature(orbital_radius, atmos_pressure);

        planets.push(Planet::new(
            1,
            body_radius,
            orbital_radius * 2000.0,
            orbital_radius, // TODO: Calculate this from orbital radius
            1.0,            // TODO: Generate this, make tidally locked if close
            atmos_pressure,
            temperature,
            format!(
                "{:?}\n({} AU)\n({} bar)\n({}K)",
                category, orbital_radius, atmos_pressure, temperature
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
        .any(|p| (0.8..1.5).contains(&p.atmos_pressure) && (270.0..300.0).contains(&p.temperature))
}

fn has_planet(planets: &Vec<Planet>, categories: &[Category], thresh: usize) -> bool {
    let count = planets
        .iter()
        .filter(|p| categories.contains(&p.category))
        .count();
    count >= thresh
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

fn sample_atmos_pressure(rng: &mut impl Rng, body_radius: f32, orbital_radius: f32) -> f32 {
    let retention = 1.0 + orbital_radius.powf(0.25);
    let volatile_factor = match orbital_radius {
        orbital_radius if orbital_radius < 0.5 => 0.1, // almost none
        orbital_radius if orbital_radius < 1.5 => 0.5, // some
        orbital_radius if orbital_radius < 3.5 => 1.0, // decent
        _ => 1.0,                                      // lots of ices
    };
    let base_pressure = retention * volatile_factor;
    let pressure = base_pressure * rng.gen_range(0.5..1.0) * body_radius.powf(2.0);
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
        (0.1..0.5) => Category::SubEarth,
        (0.8..1.5) => Category::EarthLike,
        (2.0..10.0) => Category::SuperEarth,
        (10.0..30.0) => Category::MiniNeptune,
        (30.0..300.0) => Category::GasGiant,
        _ => Category::SuperGasGiant,
    }
}
