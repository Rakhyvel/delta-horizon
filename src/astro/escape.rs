use crate::astro::{
    epoch::EphemerisTime,
    maneuver::{find_periapsis, find_soi_exit, get_grandparent_state, sphere_of_influence},
    state::State,
    units::{G, METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR},
};

#[derive(Clone, Copy)]
pub struct EscapePlan {
    pub escape_burn: State,
    pub escape_dv: f64,
    pub grandparent_orbit: State,
    pub soi_radius: f64,
}

pub fn plan_escape(
    craft_state: &State,
    parent_state: &State,
    current_et: EphemerisTime,
    grandparent_mass: f64, // in earth masses
    parent_mass: f64,      // in earth masses
) -> Result<EscapePlan, String> {
    let grandparent_mu = G * grandparent_mass;
    let mu = G * parent_mass;

    println!(
        "parent sma: {}",
        parent_state.semi_major_axis(grandparent_mu)
    );
    println!("parent ecc: {}", parent_state.ecc(grandparent_mu));
    println!("grandparent_mu: {}", grandparent_mu);

    let _ = craft_state
        .period(mu)
        .ok_or("can't escape while not on a periodic orbit")?;

    // Second cancels all surface-relative velocity at periapsis
    let peri_state = find_periapsis(craft_state, current_et, mu)?;
    let (escape_burn, escape_dv) = escape_burn(&peri_state, mu);

    let soi_radius = sphere_of_influence(
        parent_state.semi_major_axis(grandparent_mu),
        parent_mass,
        grandparent_mass,
    );

    let grandparent_orbit =
        get_grandparent_state(&escape_burn, parent_state, soi_radius, grandparent_mu, mu);

    Ok(EscapePlan {
        escape_burn,
        escape_dv: escape_dv * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
        grandparent_orbit,
        soi_radius,
    })
}

pub fn escape_burn(peri_state: &State, mu: f64) -> (State, f64) {
    let r = peri_state.r.norm();
    let v_current = peri_state.v.norm();
    let v_escape = (2.0 * mu / r).sqrt();
    let dv = v_escape * 1.01 - v_current; // give it a little nudge

    let v_hat = peri_state.v.normalize();

    (
        State {
            r: peri_state.r,
            v: peri_state.v + v_hat * dv,
            t: peri_state.t,
        },
        dv.abs(),
    )
}
