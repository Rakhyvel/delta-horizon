use std::collections::HashMap;

pub struct PartInventory {
    /// Maps part IDs to how many of them there are
    pub parts: HashMap<u64, u32>,
}

impl PartInventory {
    pub fn add(&mut self, part_id: u64, count: u32) {
        *self.parts.entry(part_id).or_insert(0) += count;
    }

    pub fn quantity(&self, part_id: u64) -> u32 {
        self.parts[&part_id]
    }

    pub fn take(&mut self, part_id: u64) -> Result<(), String> {
        let current = self.parts.get(&part_id).copied().unwrap_or(0);
        if current == 0 {
            return Err(format!("not enough {}", part_id));
        }
        *self.parts.entry(part_id).or_insert(0) -= 1;
        Ok(())
    }
}
