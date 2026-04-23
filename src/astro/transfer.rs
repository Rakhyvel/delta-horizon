use std::f64::consts::PI;

use nalgebra_glm::{vec3, DVec3, DVec4, TVec};

use crate::{
    astro::{
        epoch::EphemerisTime,
        lambert::lambert,
        newton::{newton_target, NLProblem},
        state::State,
    },
    generation::solar_system_gen::G,
};

pub struct BurnTargeter {
    craft_state_t0: State,
    target_state_t0: State,
    current_et: EphemerisTime,
    tof: f64,
    depart_et: EphemerisTime,
    mu: f64,
}

impl NLProblem<3, 3> for BurnTargeter {
    fn resid(&self, controls: &TVec<f64, 3>) -> TVec<f64, 3> {
        let dv = controls;

        let mut new_state = self.craft_state_t0.propagate(self.depart_et, self.mu);
        new_state.v += dv;

        let tf = self.depart_et + EphemerisTime::from_years(self.tof);

        println!("ecc: {}", new_state.ecc(self.mu));

        let craft_state_tf = new_state.propagate(tf, self.mu);
        let target_state_tf = self.target_state_t0.propagate(tf, self.mu);

        craft_state_tf.r - target_state_tf.r
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
    pub soi_radius: f64,
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

    // Start off with just a basic hohmann
    let transfer_a = (init_state.semi_major_axis(mu) + target_body_state.semi_major_axis(mu)) / 2.0;
    let tof_guess = PI * (transfer_a.powi(3) / mu).sqrt();

    // Sweep through the orbit, find cheapest dv transfer
    let craft_period = init_state.period(mu).unwrap();
    let target_period = target_body_state.period(mu).unwrap();
    let full_period = craft_period.max(target_period);

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

            let new_init_state = init_state.propagate(et, mu);
            let new_target_state =
                target_body_state.propagate(et + EphemerisTime::from_years(tof), mu);

            let departure_velocity = lambert(new_init_state.r, new_target_state.r, tof, mu);
            let dv = departure_velocity - new_init_state.v;
            (dv, et, tof)
        })
        .filter_map(|(dv, et, tof)| {
            let cost = objective.cost(dv.norm(), tof)?;
            Some((dv, et, tof, cost))
        })
        .min_by(|(_, _, _, cost_a), (_, _, _, cost_b)| cost_a.partial_cmp(cost_b).unwrap())
        .expect("no feasible transfer found");

    const EARTH_RADIUS_KM: f64 = 6371.0;
    const YEAR_S: f64 = 31_557_600.0;

    let dv_mag = best_dv.norm();
    let dv_mag_kms: f64 = dv_mag * EARTH_RADIUS_KM / YEAR_S;
    println!(
        "best xfer is {} ({} km/s) at {:?}",
        dv_mag, dv_mag_kms, best_et
    );

    let depart_et = best_et;
    let dv = best_dv;

    let mut transfer_state = init_state.propagate(depart_et, mu);
    transfer_state.v += dv;

    let soi_radius = sphere_of_influence(
        target_body_state.semi_major_axis(mu),
        target_mass,
        parent_mass,
    );

    let arrival_et = find_soi_entry(&transfer_state, target_body_state, soi_radius, best_tof, mu);
    let flyby_state = get_flyby_state(&transfer_state, target_body_state, arrival_et, mu);

    TransferInfo {
        transfer_state,
        flyby_state: transfer_state,
        soi_radius,
    }
}

fn find_soi_entry(
    transfer_orbit: &State,
    target_orbit: &State,
    target_soi: f64, // earth radii
    tof: f64,        // in years
    mu: f64,         // of the common parent
) -> EphemerisTime {
    let n_samples = 10_000;

    // Sweep through transfer orbit backward, find the first instance where the craft is NOT in the SOI
    for i in 0..n_samples {
        let t = 1.0 - (i as f64 / n_samples as f64);
        let sample_time = transfer_orbit.t + EphemerisTime::from_years(t * tof);

        let craft_pos = transfer_orbit.propagate(sample_time, mu).r;
        let target_pos = target_orbit.propagate(sample_time, mu).r;
        let distance = (craft_pos - target_pos).norm();
        println!("{}", distance);

        if distance >= target_soi {
            return sample_time;
        }
    }

    panic!("SOI entry not found!");
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
