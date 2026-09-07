use std::f64::consts::PI;

use nalgebra_glm::DVec3;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::astro::{
    epoch::EphemerisTime,
    lambert::{lambert, TransferKind},
    maneuver::{
        circularization, find_periapsis, find_soi_entry, get_grandparent_state, sphere_of_influence,
    },
    state::State,
    units::{G, METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR},
};

#[derive(Clone, Copy)]
pub enum Arrival {
    /// Capture into a circular orbit
    Capture,
    /// Pass through, no arrival burn
    Flyby,
}

#[allow(unused)]
#[derive(Debug)]
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

pub struct Encounter {
    pub transfer_state: State,
    pub flyby_state: State,
    pub arrival_et: EphemerisTime,
    pub soi_radius: f64,
    pub target_peri: f64,
    pub depart_dv: f64,
}

fn plan_encounter(
    craft_state: &State,
    target_body_state: &State,
    target_body_radius: f64,
    current_et: EphemerisTime,
    parent_mass: f64, // in earth masses
    target_mass: f64, // in earth masses
    objective: TransferObjective,
    arrival_mode: Arrival,
) -> Result<Encounter, String> {
    let mu = G * parent_mass;
    let target_mu = G * target_mass;

    // Start off with just a basic hohmann
    let transfer_a =
        (craft_state.semi_major_axis(mu) + target_body_state.semi_major_axis(mu)) / 2.0;
    let tof_guess = PI * (transfer_a.powi(3) / mu).sqrt();

    // Sweep through the orbit, find cheapest dv transfer
    let craft_period = craft_state
        .period(mu)
        .ok_or("can't transfer while on a hyperbolic trajectory")?;
    let target_period = target_body_state
        .period(mu)
        .ok_or("cant transfer to a hyperbolic target")?;
    let synodic = 1.0 / (1.0 / craft_period - 1.0 / target_period).abs();
    // guard against near-co-orbital targets, synodic would go to infinity
    let sweep = synodic.min(craft_period * 20.0);

    const DEPART_STEPS: usize = 100;
    const TOF_STEPS: usize = 20;
    let tof_min = tof_guess * 0.7;
    let tof_max = tof_guess * 1.5;
    let step = EphemerisTime::from_years(sweep / DEPART_STEPS as f64);

    let soi_radius = sphere_of_influence(
        target_body_state.semi_major_axis(mu),
        target_mass,
        parent_mass,
    );
    let target_peri = (target_body_radius * 1.2).min(soi_radius * 0.5);

    let (dv, depart_et, tof, _) = (0..TOF_STEPS)
        .flat_map(|j| {
            let tof = tof_min + (tof_max - tof_min) * j as f64 / (TOF_STEPS - 1) as f64;
            (1..=DEPART_STEPS).map(move |i| (i, tof))
        })
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|(i, tof)| {
            let et = current_et + step * i as i64;
            let new_craft_state = craft_state.propagate(et, mu).ok()?;
            let target_arrival_et = et + EphemerisTime::from_years(tof);
            let target_future = target_body_state.propagate(target_arrival_et, mu).ok()?;

            let (dv, arrival, _) = [TransferKind::Short, TransferKind::Long]
                .into_iter()
                .filter_map(|k| {
                    aim_for_periapsis(
                        new_craft_state.r,
                        target_future,
                        tof,
                        mu,
                        target_mu,
                        soi_radius,
                        target_peri,
                        0.0,
                        k,
                    )
                    .map(|(v1, v2)| {
                        let dv = v1 - new_craft_state.v;
                        let v_inf = (v2 - target_future.v).norm();
                        let arrival = match arrival_mode {
                            Arrival::Capture => capture_dv(v_inf, target_mu, target_peri),
                            Arrival::Flyby => 0.0,
                        };
                        (dv, arrival, dv.norm() + arrival)
                    })
                })
                .min_by(|(_, _, a), (_, _, b)| a.total_cmp(b))?;

            Some((dv, arrival, et, tof))
        })
        .filter_map(|(dv, circ_dv, et, tof)| {
            let cost = objective.cost(dv.norm() + circ_dv, tof)?;
            Some((dv, et, tof, cost))
        })
        .min_by(|(_, _, _, cost_a), (_, _, _, cost_b)| cost_a.total_cmp(cost_b))
        .ok_or("no feasible transfer found")?;

    let mut transfer_state = craft_state.propagate(depart_et, mu)?;
    transfer_state.v += dv;

    let arrival_et = find_soi_entry(&transfer_state, target_body_state, soi_radius, tof, mu)?;
    let flyby_state = get_flyby_state(&transfer_state, target_body_state, arrival_et, mu)?;

    let depart_dv = dv.norm() * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR;

    Ok(Encounter {
        transfer_state,
        flyby_state,
        arrival_et,
        soi_radius,
        target_peri,
        depart_dv,
    })
}

#[derive(Clone, Copy)]
pub struct TransferPlan {
    pub transfer_state: State,
    pub flyby_state: State,
    pub circ_state: State,
    pub soi_radius: f64,
    pub transfer_dv: f64,
    pub circ_dv: f64,
}

pub fn plan_transfer(
    craft_state: &State,
    target_body_state: &State,
    target_body_radius: f64,
    current_et: EphemerisTime,
    parent_mass: f64, // in earth masses
    target_mass: f64, // in earth masses
    objective: TransferObjective,
) -> Result<TransferPlan, String> {
    let e = plan_encounter(
        craft_state,
        target_body_state,
        target_body_radius,
        current_et,
        parent_mass,
        target_mass,
        objective,
        Arrival::Capture,
    )?;

    let target_mu = G * target_mass;

    let peri_state = find_periapsis(
        &e.flyby_state,
        e.arrival_et + EphemerisTime::from_secs(1.0),
        target_mu,
    )?;
    let (circ_state, circ_dv) = circularization(&peri_state, target_mu);

    println!(
        "target_peri={:.5} achieved={:.5}   soi={:.5}",
        e.target_peri,
        peri_state.r.norm(),
        e.soi_radius,
    );

    Ok(TransferPlan {
        transfer_state: e.transfer_state,
        flyby_state: e.flyby_state,
        circ_state,
        soi_radius: e.soi_radius,
        transfer_dv: e.depart_dv,
        circ_dv: circ_dv * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
    })
}

#[derive(Clone, Copy)]
pub struct FlybyPlan {
    pub transfer_state: State,
    pub flyby_state: State,
    pub exit_state: State,
    pub soi_radius: f64,
    pub transfer_dv: f64,
}

pub fn plan_flyby(
    craft_state: &State,
    target_body_state: &State,
    target_body_radius: f64,
    current_et: EphemerisTime,
    parent_mass: f64, // in earth masses
    target_mass: f64, // in earth masses
    objective: TransferObjective,
) -> Result<FlybyPlan, String> {
    let e = plan_encounter(
        craft_state,
        target_body_state,
        target_body_radius,
        current_et,
        parent_mass,
        target_mass,
        objective,
        Arrival::Flyby,
    )?;

    let mu = G * parent_mass;
    let target_mu = G * target_mass;

    let exit_state = get_grandparent_state(
        &e.flyby_state,
        target_body_state,
        e.soi_radius,
        mu,
        target_mu,
    )?;

    Ok(FlybyPlan {
        transfer_state: e.transfer_state,
        flyby_state: e.flyby_state,
        exit_state,
        soi_radius: e.soi_radius,
        transfer_dv: e.depart_dv,
    })
}

fn aim_for_periapsis(
    r_craft: DVec3,
    target: State,
    tof: f64,
    mu: f64,
    target_mu: f64,
    soi_radius: f64,
    target_peri: f64,
    theta: f64,
    kind: TransferKind,
) -> Option<(DVec3, DVec3)> {
    let mut aim = target.r;
    let mut out = None;

    for _ in 0..3 {
        let (v1, v2) = lambert(r_craft, aim, tof, mu, kind)?;
        let v_inf_vec = v2 - target.v;
        let v_inf = v_inf_vec.norm();

        let b_max = soi_radius * 0.7;
        let b = target_peri * (1.0 + 2.0 * target_mu / (target_peri * v_inf * v_inf)).sqrt();
        let b = b.min(b_max); // clamp to be within the SOI

        // Build the B-plane basis
        let s_hat = v_inf_vec / v_inf;
        let h_ref = target.r.cross(&target.v).normalize();
        let t_hat = s_hat.cross(&h_ref).normalize();
        let r_hat = s_hat.cross(&t_hat);

        // theta selects leading vs trailing, above vs below
        let b_vec = b * (theta.cos() * t_hat + theta.sin() * r_hat);

        // re-solve aiming at the offset point
        let d = (soi_radius * soi_radius - b * b).max(0.0).sqrt();
        aim = target.r + b_vec - s_hat * d;

        out = Some((v1, v2));
    }

    out
}

fn get_flyby_state(
    transfer_orbit: &State,
    target_orbit: &State,
    arrival_et: EphemerisTime,
    mu: f64, // of the common parent
) -> Result<State, String> {
    let craft_state_at_soi = transfer_orbit.propagate(arrival_et, mu)?;
    let target_state_at_soi = target_orbit.propagate(arrival_et, mu)?;

    let r_rel = craft_state_at_soi.r - target_state_at_soi.r; // TODO: Maybe you should be able to subtract states?
    let v_rel = craft_state_at_soi.v - target_state_at_soi.v;

    Ok(State {
        r: r_rel,
        v: v_rel,
        t: arrival_et,
    })
}

fn capture_dv(v_inf: f64, target_mu: f64, r_p: f64) -> f64 {
    (v_inf * v_inf + 2.0 * target_mu / r_p).sqrt() - (target_mu / r_p).sqrt()
}
