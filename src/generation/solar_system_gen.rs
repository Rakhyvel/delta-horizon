use rand::{Rng, SeedableRng};

#[derive(Debug, PartialEq)]
pub enum Category {
    Dwarf,
    SubEarth,
    EarthLike,
    SuperEarth,
    MiniNeptune,
    GasGiant,
    SuperGasGiant,
}

struct MassCategory {
    category: Category,
    range: (f32, f32),
    weight: f32,
}

const MASS_CATEGORIES: &[MassCategory] = &[
    MassCategory {
        category: Category::Dwarf,
        range: (0.01, 0.1),
        weight: 10.0,
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

#[derive(Debug, PartialEq)]
pub enum Zone {
    Hot,
    Habitable,
    Icy,
}

pub struct PlanetSpec {
    pub orbital_radius: f32, // AU
    pub mass: f32,           // Earth masses
    pub category: Category,
    pub zone: Zone,
}

pub fn generate() -> Vec<PlanetSpec> {
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

fn generate_system(rng: &mut impl Rng) -> Vec<PlanetSpec> {
    let mut planets: Vec<PlanetSpec> = vec![];

    let mut r = rng.gen_range(0.2..0.6);
    while r < 35.0 {
        let mass = sample_mass_with_au(rng);

        let category = categorize_planet(mass);
        let zone = categorize_zone(r);

        planets.push(PlanetSpec {
            orbital_radius: r,
            mass,
            category,
            zone,
        });

        let spacing = compute_spacing(rng, r, mass);
        r += spacing;
    }

    planets
}

fn has_planet(planets: &Vec<PlanetSpec>, category: Category, zone: Zone) -> bool {
    planets
        .iter()
        .any(|p| p.category == category && p.zone == zone)
}

fn has_category(planets: &Vec<PlanetSpec>, category: Category) -> bool {
    planets.iter().any(|p| p.category == category)
}

fn sample_mass_with_au(rng: &mut impl Rng) -> f32 {
    // Compute cumulative weights
    let total_weight: f32 = MASS_CATEGORIES.iter().map(|c| c.weight).sum();
    let mut roll = rng.gen_range(0.0..total_weight);

    let mut mass = 0.0;
    for cat in MASS_CATEGORIES {
        if roll <= cat.weight {
            mass = rng.gen_range(cat.range.0..cat.range.1);
            break;
        }
        roll -= cat.weight;
    }

    mass
}

fn compute_spacing(rng: &mut impl Rng, orbital_radius: f32, mass: f32) -> f32 {
    let base = rng.gen_range(1.1..1.2);
    let mass_boost = 1.0 + 0.1 * mass.powf(0.25);
    let jitter = rng.gen_range(0.9..1.2);

    orbital_radius * base * mass_boost * jitter
}

fn categorize_zone(orbital_radius: f32) -> Zone {
    match orbital_radius {
        (0.0..0.8) => Zone::Hot,
        (0.8..1.5) => Zone::Habitable,
        _ => Zone::Icy,
    }
}

fn categorize_planet(mass: f32) -> Category {
    match mass {
        (0.0..0.1) => Category::Dwarf,
        (0.1..0.5) => Category::SubEarth,
        (0.5..2.0) => Category::EarthLike,
        (2.0..10.0) => Category::SuperEarth,
        (10.0..30.0) => Category::MiniNeptune,
        (30.0..300.0) => Category::GasGiant,
        _ => Category::SuperGasGiant,
    }
}
