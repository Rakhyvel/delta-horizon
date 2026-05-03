use std::collections::HashMap;

pub struct PartInventory {
    /// Maps part IDs to how many of them there are
    pub parts: HashMap<String, u32>,
}

impl PartInventory {
    pub fn add(&mut self, part_id: &str, count: u32) {
        *self.parts.entry(part_id.to_string()).or_insert(0) += count;
    }

    pub fn take(&mut self, part_id: &str) -> Result<(), String> {
        let current = self.parts.get(part_id).copied().unwrap_or(0);
        if current == 0 {
            return Err(format!("not enough {}", part_id));
        }
        *self.parts.entry(part_id.to_string()).or_insert(0) -= 1;
        Ok(())
    }
}
