use nalgebra_glm::{zero, DVec3};

use crate::astro::{epoch::EphemerisTime, state::State};

pub fn sphere_of_influence(orbital_radius: f64, body_mass: f64, parent_mass: f64) -> f64 {
    orbital_radius * (body_mass / parent_mass).powf(2.0 / 5.0)
}

pub fn find_apoapsis(orbit: &State, current_et: EphemerisTime, mu: f64) -> Result<State, String> {
    let rdotv_at_t = |et: EphemerisTime| -> f64 {
        let state = orbit.propagate(et, mu);
        state.r.dot(&state.v)
    };

    let period = orbit
        .period(mu)
        .ok_or("only periodic orbits have apoapsides")?;

    let dt = EphemerisTime::from_secs(60.0);
    let mut lo = current_et;
    let mut hi = current_et;
    let max_et = current_et + EphemerisTime::from_years(period);

    const TOL: f64 = 1e-6;
    if orbit.ecc(mu) < TOL {
        // orbit is circular, apoapsis isn't defined. Just pick this point
        return Ok(orbit.propagate(current_et + dt, mu));
    }

    // If already past apoapsis (rdotv < 0), march lo forward until rdotv > 0
    // so lo is guaranteed to be on the positive side
    while rdotv_at_t(lo) < 0.0 {
        lo += dt;
        hi = lo;
        if lo > max_et {
            return Err(String::from("apoapsis not found within one period"));
        }
    }

    // now march hi forward until rdotv goes negative
    while rdotv_at_t(hi) > 0.0 {
        hi += dt;
        if hi > max_et {
            return Err(String::from("apoapsis not found within one period"));
        }
    }

    // Binary search for rdotv = 0 crossing (positive -> negative)
    const ITERATIONS: usize = 50;
    for _ in 0..ITERATIONS {
        let mid = lo + (hi - lo) / 2;
        if rdotv_at_t(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    Ok(orbit.propagate(hi, mu))
}

pub fn find_periapsis(orbit: &State, current_et: EphemerisTime, mu: f64) -> State {
    let rdotv_at_t = |et: EphemerisTime| -> f64 {
        let state = orbit.propagate(et, mu);
        state.r.dot(&state.v)
    };

    const TOL: f64 = 1e-6;
    if orbit.ecc(mu) < TOL {
        // orbit is circular, periapsis isn't defined. Just pick this point
        return orbit.propagate(current_et + EphemerisTime::from_secs(60.0), mu);
    }

    if orbit.ecc(mu) >= 1.0 && rdotv_at_t(current_et) > 0.0 {
        // hyperbolic orbit and we're already past the periapsis
        return orbit.propagate(current_et + EphemerisTime::from_secs(60.0), mu);
    }

    // Use period-based step for elliptical, or r/v based step for hyperbolic
    let dt = if let Some(period) = orbit.period(mu) {
        EphemerisTime::from_years(period / 100.0) // 100 steps per orbit
    } else {
        // Hyperbolic - use time to travel one radius at current speed
        let r = orbit.propagate(current_et, mu).r.norm();
        let v = orbit.propagate(current_et, mu).v.norm();
        EphemerisTime::from_years(r / v / 10.0)
    };

    let mut lo = current_et;
    let mut hi = current_et;

    // If already past periapsis (rdotv > 0), march lo forward until rdotv < 0
    while rdotv_at_t(lo) > 0.0 {
        lo += dt;
        hi = lo;
    }

    // Now march hi forward until rdotv goes positive
    while rdotv_at_t(hi) < 0.0 {
        hi += dt;
    }

    // Binary search for the zero crossing
    const ITERATIONS: usize = 50;
    for _ in 0..ITERATIONS {
        let mid = lo + (hi - lo) / 2;
        if rdotv_at_t(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    orbit.propagate(hi, mu)
}

pub fn circularization(orbit: &State, mu: f64) -> (State, f64) {
    let r = orbit.r;
    let v = orbit.v;

    let r_mag = r.norm();

    // the velocity if we were in a circular orbit
    let v_circ_mag = (mu / r_mag).sqrt();

    // convert the scalar velocity to a scalar
    let r_hat = r.normalize();
    let h_hat = r.cross(&v).normalize();
    let t_hat = h_hat.cross(&r_hat);

    let v_circ = t_hat * v_circ_mag;

    // circ dv is just the differnce between v_circ and v

    (
        State {
            r: orbit.r,
            v: v_circ,
            t: orbit.t,
        },
        (v_circ - v).norm(),
    )
}
