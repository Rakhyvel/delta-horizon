pub struct StationModule {
    pub slot: u32,
}

pub struct SolarPanel {
    pub rated_kw: f32,
}

impl SolarPanel {
    pub fn output_kw(&self) -> f32 {
        self.rated_kw
    }
}
