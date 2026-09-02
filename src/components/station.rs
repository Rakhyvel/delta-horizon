pub struct Station {
    pub charge_kwh: f32,
    pub capacity_kwh: f32,

    /// bumped whenever a module is added or removed
    pub modules_gen: u32,
}
