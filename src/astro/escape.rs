use crate::astro::{
    epoch::EphemerisTime,
    maneuver::{find_periapsis, sphere_of_influence},
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

fn find_soi_exit(orbit: &State, soi: f64, mu: f64) -> EphemerisTime {
    // First find a rough bracket by marching forward
    let dt_coarse = EphemerisTime::from_years(1.0 / 365.0); // 1 day steps
    let mut t = orbit.t + EphemerisTime::from_secs(60.0);

    // March until we're outside the SOI
    loop {
        t += dt_coarse;
        let pos = orbit.propagate(t, mu).unwrap().r;
        if pos.norm() >= soi {
            break;
        }
        // Safety limit - 10 years
        if t > orbit.t + EphemerisTime::from_years(10.0) {
            panic!("SOI exit not found within 10 years");
        }
    }

    // Binary search to refine
    let mut lo = t - dt_coarse;
    let mut hi = t;

    const ITERATIONS: usize = 50;
    for _ in 0..ITERATIONS {
        let mid = lo + (hi - lo) / 2;
        let pos = orbit.propagate(mid, mu).unwrap().r;
        if pos.norm() < soi {
            lo = mid; // inside SOI, search later
        } else {
            hi = mid; // outisde SOI, search earlier
        }
    }

    hi
}

fn get_grandparent_state(
    escape_burn: &State,
    parent_state: &State,
    soi: f64,
    grandparent_mu: f64,
    mu: f64,
) -> State {
    let soi_exit_et = find_soi_exit(escape_burn, soi, mu);

    // Craft state at SOI exit in parent-relative frame
    let craft_at_exit = escape_burn.propagate(soi_exit_et, mu).unwrap();

    // Parent state at SOI exit in grandparent frame
    let parent_at_exit = parent_state.propagate(soi_exit_et, grandparent_mu).unwrap();

    // Recontextualize to grandparent frame
    State {
        r: craft_at_exit.r + parent_at_exit.r,
        v: craft_at_exit.v + parent_at_exit.v,
        t: soi_exit_et,
    }
}
