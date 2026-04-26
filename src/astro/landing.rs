use nalgebra_glm::{zero, DVec3};

use crate::astro::{
    epoch::EphemerisTime,
    maneuver::{find_apoapsis, find_periapsis},
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

pub fn deorbit_burn(craft_apoapsis: &State, body_radius: f64, mu: f64) -> (State, f64) {
    let r_a = craft_apoapsis.r.norm();
    let a_target = (r_a + body_radius) / 2.0;

    let v_target = (mu * (2.0 / r_a - 1.0 / a_target)).sqrt();
    let v_current = craft_apoapsis.v.norm();

    let dv = v_target - v_current;
    let v_hat = craft_apoapsis.v.normalize();

    (
        State {
            r: craft_apoapsis.r,
            v: craft_apoapsis.v + v_hat * dv,
            t: craft_apoapsis.t,
        },
        dv.abs(),
    )
}

pub fn landing_burn(peri_state: &State) -> (State, f64) {
    let surface_velocity: DVec3 = zero(); // TODO: Update when planets rotate
                                          // surface_velocity = omega.cross(&peri_state.r)
    let cancel_velocity: DVec3 = surface_velocity - peri_state.v;
    let dv = cancel_velocity.norm();

    (
        State {
            r: peri_state.r,
            v: surface_velocity,
            t: peri_state.t,
        },
        dv,
    )
}
