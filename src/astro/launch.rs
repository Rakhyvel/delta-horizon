use nalgebra_glm::{vec3, zero, DVec3};

use crate::astro::{
    epoch::EphemerisTime,
    maneuver::{circularization, find_apoapsis, find_periapsis, sphere_of_influence},
    state::State,
    units::{G, METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR},
};

#[derive(Clone, Copy)]
pub struct LaunchPlan {
    pub launch_burn: State,
    pub launch_dv: f64,
    pub circ_burn: State,
    pub circ_dv: f64,
}

pub fn plan_launch(
    craft_pos: DVec3,
    parent_state: &State,
    parent_body_radius: f64,
    current_et: EphemerisTime,
    grandparent_mass: f64, // in earth masses
    parent_mass: f64,      // in earth masses
) -> Result<LaunchPlan, String> {
    let grandparent_mu = G * grandparent_mass;
    let mu = G * parent_mass;

    let parent_orbital_radius = parent_state.semi_major_axis(grandparent_mu);
    let parent_soi = sphere_of_influence(parent_orbital_radius, parent_mass, grandparent_mass);
    let target_apoapsis = (parent_body_radius + 2.0).min(parent_soi * 0.5);

    // First burn is with eastward, to get a good apoapsis
    let launch_offset = current_et + EphemerisTime::from_secs(5.0); // so our event planner doesn't freak
    let (launch_burn, launch_dv) = launch_burn(
        craft_pos,
        parent_body_radius,
        target_apoapsis,
        launch_offset,
        mu,
    );

    // Second burn circularizes
    let apoapsis = find_apoapsis(
        &launch_burn,
        launch_burn.t + EphemerisTime::from_secs(60.0),
        mu,
    )?;
    let (circ_burn, circ_dv) = circularization(&apoapsis, mu);

    Ok(LaunchPlan {
        launch_burn,
        launch_dv: launch_dv * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
        circ_burn,
        circ_dv: circ_dv * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
    })
}

fn launch_burn(
    craft_pos: DVec3,
    body_radius: f64,
    target_apoapsis: f64,
    current_et: EphemerisTime,
    mu: f64,
) -> (State, f64) {
    let planet_axis: DVec3 = vec3(0.0, 0.0, 1.0);
    let initial_v: DVec3 = zero(); // surface velocity, zero for now

    let east = planet_axis.cross(&craft_pos).normalize(); // eastward at launch site

    let a_target = (body_radius + target_apoapsis) / 2.0;

    let v_target = (mu * (2.0 / body_radius - 1.0 / a_target)).sqrt();
    let launch_dv = v_target - initial_v.norm();

    (
        State {
            r: craft_pos,
            v: initial_v + east * launch_dv,
            t: current_et,
        },
        launch_dv,
    )
}
