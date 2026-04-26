use std::f64::consts::PI;

use nalgebra_glm::{DVec1, TVec};

use crate::{
    astro::{
        epoch::EphemerisTime,
        lambert::lambert,
        newton::{newton_target, NLProblem},
        state::State,
    },
    generation::solar_system_gen::G,
};

pub const EARTH_RADIUS_KM: f64 = 6371.0;
pub const EARTH_RADIUS_M: f64 = EARTH_RADIUS_KM * 1000.0;
pub const YEAR_S: f64 = 31_557_600.0;

pub struct BurnTargeter {
    transfer_state: State,
    target_state: State,
    parent_mu: f64,
    target_mu: f64,
    soi_radius: f64,
    tof: f64,
    target_peri: f64,
}

impl NLProblem<1, 1> for BurnTargeter {
    fn resid(&self, controls: &TVec<f64, 1>) -> TVec<f64, 1> {
        let mut transfer_state = self.transfer_state;
        transfer_state.v *= controls.x;

        let arrival_et = find_soi_entry(
            &transfer_state,
            &self.target_state,
            self.soi_radius,
            self.tof,
            self.parent_mu,
        );
        let flyby_state = get_flyby_state(
            &transfer_state,
            &self.target_state,
            arrival_et,
            self.parent_mu,
        );

        let peri_state = find_periapsis(&flyby_state, self.target_mu);

        let peri = peri_state.r.norm();

        TVec::<f64, 1>::new(peri - self.target_peri)
    }
}

pub enum TransferObjective {
    /// minimize total delta-v
    MinFuel,
    /// minimize tof, subject to a max delta-v budget
    MinTime { max_dv: f64 },
    /// weighed combination, alpha * dv + (1 - alpha) * tof
    Balanced { dv_weight: f64, tof_weight: f64 },
}

impl TransferObjective {
    /// return the cost given dv and tof, if feasible
    fn cost(&self, dv: f64, tof: f64) -> Option<f64> {
        match self {
            TransferObjective::MinFuel => Some(dv),
            TransferObjective::MinTime { max_dv } => {
                if dv <= *max_dv {
                    Some(tof)
                } else {
                    None
                }
            }
            TransferObjective::Balanced {
                dv_weight,
                tof_weight,
            } => Some(*dv_weight * dv + tof_weight * tof),
        }
    }
}

pub struct TransferInfo {
    pub transfer_state: State,
    pub flyby_state: State,
    pub circ_state: State,
    pub soi_radius: f64,
    pub transfer_dv: f64,
    pub circ_dv: f64,
}

pub fn plan_transfer(
    init_state: &State,
    target_body_state: &State,
    current_et: EphemerisTime,
    parent_mass: f64, // in earth masses
    target_mass: f64, // in earth masses
    objective: TransferObjective,
) -> TransferInfo {
    let mu = G * parent_mass;
    let target_mu = G * target_mass;

    // Start off with just a basic hohmann
    let transfer_a = (init_state.semi_major_axis(mu) + target_body_state.semi_major_axis(mu)) / 2.0;
    let tof_guess = PI * (transfer_a.powi(3) / mu).sqrt();

    // Sweep through the orbit, find cheapest dv transfer
    let craft_period = init_state.period(mu).unwrap();
    let target_period = target_body_state
        .period(mu)
        .expect("planets typically arent in hyperbolic orbits");
    let full_period = craft_period.min(target_period);

    const DEPART_STEPS: usize = 100;
    const TOF_STEPS: usize = 20;
    let tof_min = tof_guess * 0.5;
    let tof_max = tof_guess * 2.0;
    let step = EphemerisTime::from_years(full_period / DEPART_STEPS as f64);

    let (best_dv, best_et, best_tof, _) = (0..TOF_STEPS)
        .flat_map(|j| {
            let tof = tof_min + (tof_max - tof_min) * j as f64 / TOF_STEPS as f64;
            (1..=DEPART_STEPS).map(move |i| (i, tof))
        })
        .map(|(i, tof)| {
            let et = current_et + step * i as i64;

            let new_craft_state = init_state.propagate(et, mu);

            // Lag the target position by our desired flyby periapsis
            // little hack that lets us easily get reasonable prograde flyby periapsis without too much complexity
            const DESIRED_TARGET_PE: f64 = 2.0;
            let target_arrival_et = et + EphemerisTime::from_years(tof);
            let target_future = target_body_state.propagate(target_arrival_et, mu);
            let delta_t = DESIRED_TARGET_PE / target_future.v.norm();
            let lagged_et = target_arrival_et - EphemerisTime::from_years(delta_t);
            let new_target_state = target_future.propagate(lagged_et, mu);

            let departure_velocity = lambert(new_craft_state.r, new_target_state.r, tof, mu);
            let dv = departure_velocity - new_craft_state.v;
            (dv, et, tof)
        })
        .filter_map(|(dv, et, tof)| {
            let cost = objective.cost(dv.norm(), tof)?;
            Some((dv, et, tof, cost))
        })
        .min_by(|(_, _, _, cost_a), (_, _, _, cost_b)| cost_a.partial_cmp(cost_b).unwrap())
        .expect("no feasible transfer found");

    let xfer_dv = best_dv.norm();

    let depart_et = best_et;
    let dv = best_dv;

    let mut transfer_state = init_state.propagate(depart_et, mu);
    transfer_state.v += dv;

    let soi_radius = sphere_of_influence(
        target_body_state.semi_major_axis(mu),
        target_mass,
        parent_mass,
    );

    let prob = BurnTargeter {
        transfer_state,
        target_state: *target_body_state,
        parent_mu: mu,
        target_mu,
        soi_radius,
        tof: best_tof,
        target_peri: 2.0,
    };
    let res = newton_target(&prob, DVec1::new(1.0), 100, 0.5, 1.0);

    if let Ok(refined_v) = res {
        transfer_state.v *= refined_v;
    } else {
        println!("WARNING: couldn't refine the periapsis")
    }

    let arrival_et = find_soi_entry(&transfer_state, target_body_state, soi_radius, best_tof, mu);
    let flyby_state = get_flyby_state(&transfer_state, target_body_state, arrival_et, mu);

    let peri_state = find_periapsis(&flyby_state, target_mu);
    let (circ_state, circ_dv) = circularization(&peri_state, target_mu);

    TransferInfo {
        transfer_state,
        flyby_state,
        circ_state,
        soi_radius,
        transfer_dv: xfer_dv * EARTH_RADIUS_M / YEAR_S,
        circ_dv: circ_dv * EARTH_RADIUS_M / YEAR_S,
    }
}

fn find_soi_entry(
    transfer_orbit: &State,
    target_orbit: &State,
    target_soi: f64,
    tof: f64,
    mu: f64,
) -> EphemerisTime {
    let distance_at_t = |t: f64| -> f64 {
        let sample_time = transfer_orbit.t + EphemerisTime::from_years(t * tof);
        let craft_pos = transfer_orbit.propagate(sample_time, mu).r;
        let target_pos = target_orbit.propagate(sample_time, mu).r;
        (craft_pos - target_pos).norm()
    };

    // Binary search between 0 and 1 (normalized departure and periapsis)
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;

    const ITERATIONS: usize = 50;
    for _ in 0..ITERATIONS {
        let mid = (lo + hi) / 2.0;
        if distance_at_t(mid) < target_soi {
            hi = mid; // inside SOI, search earlier
        } else {
            lo = mid; // outside SOI, search later
        }
    }

    transfer_orbit.t + EphemerisTime::from_years(hi * tof)
}

fn get_flyby_state(
    transfer_orbit: &State,
    target_orbit: &State,
    arrival_et: EphemerisTime,
    mu: f64, // of the common parent
) -> State {
    let craft_state_at_soi = transfer_orbit.propagate(arrival_et, mu);
    let target_state_at_soi = target_orbit.propagate(arrival_et, mu);

    let r_rel = craft_state_at_soi.r - target_state_at_soi.r; // TODO: Maybe you should be able to subtract states?
    let v_rel = craft_state_at_soi.v - target_state_at_soi.v;

    State {
        r: r_rel,
        v: v_rel,
        t: arrival_et,
    }
}

pub fn sphere_of_influence(orbital_radius: f64, body_mass: f64, parent_mass: f64) -> f64 {
    orbital_radius * (body_mass / parent_mass).powf(2.0 / 5.0)
}

fn find_periapsis(flyby_orbit: &State, mu: f64) -> State {
    let rdotv_at_t = |et: EphemerisTime| -> f64 {
        let state = flyby_orbit.propagate(et, mu);
        state.r.dot(&state.v)
    };

    // Find a bracket where rdotv changes sign (negative -> positive)
    // Periapsis is where rdotv = 0
    let dt = EphemerisTime::from_secs(60.0);
    let mut lo = flyby_orbit.t;
    let mut hi = flyby_orbit.t;

    // March forward until we bracket the zero crossing
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

    flyby_orbit.propagate(lo, mu)
}

fn circularization(peri_state: &State, mu: f64) -> (State, f64) {
    let r = peri_state.r;
    let v = peri_state.v;

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
            r: peri_state.r,
            v: v_circ,
            t: peri_state.t,
        },
        (v_circ - v).norm(),
    )
}
