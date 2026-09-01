use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::components::craft::{Payload, Stage};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PartDef {
    pub id: String,
    pub name: String,

    pub dry_mass_kg: f64,
    pub fuel: Option<FuelSpec>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct FuelSpec {
    pub max_fuel_mass_kg: f64,
    pub isp: f64,
    pub thrust_kn: f64,
}

#[derive(serde::Deserialize)]
struct PartFile {
    parts: Vec<PartDef>,
}

#[derive(Clone)]
pub struct PartRegistry {
    parts: HashMap<u64, PartDef>,
}

impl PartRegistry {
    pub fn new() -> Self {
        Self {
            parts: HashMap::new(),
        }
    }

    pub fn load_from_dir(path: &str) -> Self {
        let mut parts = HashMap::new();

        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                let text = std::fs::read_to_string(&path).unwrap();
                let file: PartFile = toml::from_str(&text).unwrap();
                for part in file.parts {
                    let mut hasher = DefaultHasher::new();
                    part.id.hash(&mut hasher);
                    parts.insert(hasher.finish(), part);
                }
            }
        }

        Self { parts }
    }

    pub fn get(&self, id: u64) -> Option<&PartDef> {
        self.parts.get(&id)
    }

    pub fn all(&self) -> impl Iterator<Item = &PartDef> {
        self.parts.values()
    }
}

impl PartDef {
    pub fn instantiate_stage(&self) -> Stage {
        Stage {
            name: self.name.clone(),
            dry_mass: self.dry_mass_kg,
            fuel_mass: self.fuel.unwrap().max_fuel_mass_kg, // starts full
            max_fuel_mass: self.fuel.unwrap().max_fuel_mass_kg,
            thrust_kn: self.fuel.unwrap().thrust_kn,
            isp: self.fuel.unwrap().isp,
        }
    }

    pub fn instantiate_payload(&self) -> Payload {
        Payload {
            name: self.name.clone(),
            dry_mass: self.dry_mass_kg,
        }
    }

    pub fn id_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        hasher.finish()
    }
}
