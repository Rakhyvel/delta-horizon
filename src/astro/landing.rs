use crate::astro::{
    epoch::EphemerisTime,
    maneuver::{deorbit_burn, find_apoapsis, find_periapsis, landing_burn},
    state::State,
    units::METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
};

#[derive(Clone, Copy)]
pub struct LandingPlan {
    pub deorbit_burn: State,
    pub deorbit_dv: f64,
    pub landing_burn: State,
    pub landing_dv: f64,
}

pub fn plan_landing(
    craft_state: &State,
    body_radius: f64,
    current_et: EphemerisTime,
    mu: f64,
) -> Result<LandingPlan, String> {
    // First deorbit maneuver at apo brings the peri down to body_radius
    let craft_apoapsis = find_apoapsis(craft_state, current_et, mu)?;
    let (deorbit_burn, deorbit_dv) = deorbit_burn(&craft_apoapsis, body_radius, mu);

    // Second cancels all surface-relative velocity at periapsis
    let peri_state = find_periapsis(&deorbit_burn, deorbit_burn.t, mu);
    let (landing_burn, landing_dv) = landing_burn(&peri_state);

    Ok(LandingPlan {
        deorbit_burn,
        deorbit_dv: deorbit_dv * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
        landing_burn,
        landing_dv: landing_dv * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
    })
}
