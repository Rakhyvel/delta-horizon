use rand::{Rng, SeedableRng};

use crate::components::planet::{Category, Planet, Zone};

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
        range: (0.1, 0.5),
        weight: 9.0,
    },
    MassCategory {
        category: Category::EarthLike,
        range: (0.5, 2.0),
        weight: 7.0,
    },
    MassCategory {
        category: Category::SuperEarth,
        range: (2.0, 10.0),
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
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let planets = loop {
        let planets = generate_system(&mut rng);
        println!();
        // Need an earth starting point
        if !has_planet(&planets, Category::EarthLike, Zone::Habitable) {
            continue;
        }
        // Need a hot terrestrial
        if !(has_planet(&planets, Category::Dwarf, Zone::Hot)
            || has_planet(&planets, Category::SubEarth, Zone::Hot)
            || has_planet(&planets, Category::EarthLike, Zone::Hot)
            || has_planet(&planets, Category::SuperEarth, Zone::Hot))
        {
            continue;
        }
        // Need an icy terrestrial
        if !(has_planet(&planets, Category::Dwarf, Zone::Icy)
            || has_planet(&planets, Category::SubEarth, Zone::Icy)
            || has_planet(&planets, Category::EarthLike, Zone::Icy)
            || has_planet(&planets, Category::SuperEarth, Zone::Icy))
        {
            continue;
        }
        // Need a variety of icy gasses
        if !(has_planet(&planets, Category::MiniNeptune, Zone::Icy)
            || has_planet(&planets, Category::GasGiant, Zone::Icy))
        {
            continue;
        }
        if !(has_planet(&planets, Category::MiniNeptune, Zone::Icy)
            || has_planet(&planets, Category::SuperGasGiant, Zone::Icy))
        {
            continue;
        }
        if !(has_planet(&planets, Category::GasGiant, Zone::Icy)
            || has_planet(&planets, Category::SuperGasGiant, Zone::Icy))
        {
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
        let zone = categorize_zone(orbital_radius);

        planets.push(Planet::new(
            1,
            body_radius,
            orbital_radius * 2000.0,
            orbital_radius, // TODO: Calculate this from orbital radius
            1.0,            // TODO: Generate this
            format!("{:?} {:?}", category, zone),
            category,
            zone,
        ));

        let spacing = compute_spacing(rng, orbital_radius, body_radius);
        orbital_radius += spacing;
    }

    planets
}

fn has_planet(planets: &Vec<Planet>, category: Category, zone: Zone) -> bool {
    planets
        .iter()
        .any(|p| p.category == category && p.zone == zone)
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

fn compute_spacing(rng: &mut impl Rng, orbital_radius: f32, radius: f32) -> f32 {
    let base = rng.gen_range(1.1..1.2);
    let radius_boost = 1.0 + 0.1 * radius.powf(0.25);
    let jitter = rng.gen_range(0.9..1.2);

    orbital_radius * base * radius_boost * jitter
}

fn categorize_zone(orbital_radius: f32) -> Zone {
    match orbital_radius {
        (0.0..0.8) => Zone::Hot,
        (0.8..1.5) => Zone::Habitable,
        _ => Zone::Icy,
    }
}

fn categorize_planet(radius: f32) -> Category {
    match radius {
        (0.0..0.1) => Category::Dwarf,
        (0.1..0.5) => Category::SubEarth,
        (0.5..2.0) => Category::EarthLike,
        (2.0..10.0) => Category::SuperEarth,
        (10.0..30.0) => Category::MiniNeptune,
        (30.0..300.0) => Category::GasGiant,
        _ => Category::SuperGasGiant,
    }
}
