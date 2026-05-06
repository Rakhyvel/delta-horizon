use std::collections::HashMap;

use crate::components::craft::{Payload, Stage};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PartDef {
    pub id: String,
    pub name: String,

    pub dry_mass_kg: f64,
    pub build_time_days: f64,
    pub cost: ResourceCost,

    pub fuel: Option<FuelSpec>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct FuelSpec {
    pub max_fuel_mass_kg: f64,
    pub isp: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ResourceCost {
    #[serde(default)]
    pub funds: u32,
    #[serde(default)]
    pub iron: f64,
    #[serde(default)]
    pub aluminum: f64,
    #[serde(default)]
    pub silicon: f64,
    #[serde(default)]
    pub copper: f64,
    #[serde(default)]
    pub water: f64,
}

#[derive(serde::Deserialize)]
struct PartFile {
    parts: Vec<PartDef>,
}

#[derive(Clone)]
pub struct PartRegistry {
    parts: HashMap<String, PartDef>,
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
                    parts.insert(part.id.clone(), part);
                }
            }
        }

        Self { parts }
    }

    pub fn get(&self, id: &str) -> Option<&PartDef> {
        self.parts.get(id)
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
            isp: self.fuel.unwrap().isp,
        }
    }

    pub fn instantiate_payload(&self) -> Payload {
        Payload {
            name: self.name.clone(),
            dry_mass: self.dry_mass_kg,
        }
    }
}
