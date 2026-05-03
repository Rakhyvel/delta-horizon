use std::collections::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PartDef {
    pub id: String,
    pub name: String,

    pub dry_mass_kg: f64,
    pub build_time_days: f64,
    pub cost: ResourceCost,

    pub fuel: Option<FuelSpec>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FuelSpec {
    pub max_fuel_mass_kg: f64,
    pub isp: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ResourceCost {
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

pub struct PartRegistry {
    parts: HashMap<String, PartDef>,
}

impl PartRegistry {
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
