use nalgebra_glm::DVec3;

use crate::astro::{
    departure::{best_branch, sweep_window, TransferObjective},
    epoch::EphemerisTime,
    lambert::{lambert, TransferKind},
    maneuver::{
        capture_dv, circularization, find_periapsis, find_soi_entry, get_grandparent_state,
        impact_parameter, sphere_of_influence,
    },
    porkchop::{Cell, Porkchop},
    state::State,
    units::{G, METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR},
};

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
    let mu = G * parent_mass;
    let target_mu = G * target_mass;
    let w = sweep_window(craft_state, target_body_state, mu)?;

    let soi_radius = sphere_of_influence(
        target_body_state.semi_major_axis(mu),
        target_mass,
        parent_mass,
    );
    let target_peri = (target_body_radius * 1.2).min(soi_radius * 0.5);

    let chop = Porkchop::compute(
        current_et,
        w.sweep,
        w.tof_min,
        w.tof_max,
        100,
        20,
        |et, tof| {
            let craft = craft_state.propagate(et, mu).ok()?;
            let target = target_body_state
                .propagate(et + EphemerisTime::from_years(tof), mu)
                .ok()?;

            best_branch(|k| {
                let (v1, v2) = aim_for_periapsis(
                    craft.r,
                    target,
                    tof,
                    mu,
                    target_mu,
                    soi_radius,
                    target_peri,
                    0.0,
                    k,
                )?;
                let depart_dv = v1 - craft.v;
                let arrival_dv = capture_dv((v2 - target.v).norm(), target_mu, target_peri);
                Some(Cell {
                    total: depart_dv.norm() + arrival_dv,
                    depart_dv,
                    arrival_dv,
                })
            })
        },
    );
    let (i, j, cell) = chop.best(&objective).ok_or("no feasible transfer found")?;
    let depart_et = chop.depart_at(i);
    let tof = chop.tof_at(j);

    let mut transfer_state = craft_state.propagate(depart_et, mu)?;
    transfer_state.v += cell.depart_dv;

    let arrival_et = find_soi_entry(&transfer_state, target_body_state, soi_radius, tof, mu)?;
    let flyby_state = get_flyby_state(&transfer_state, target_body_state, arrival_et, mu)?;

    let peri_state = find_periapsis(
        &flyby_state,
        arrival_et + EphemerisTime::from_secs(1.0),
        target_mu,
    )?;
    let (circ_state, circ_dv) = circularization(&peri_state, target_mu);

    Ok(TransferPlan {
        transfer_state,
        flyby_state,
        circ_state,
        soi_radius,
        transfer_dv: cell.depart_dv.norm() * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
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
    let mu = G * parent_mass;
    let target_mu = G * target_mass;
    let w = sweep_window(craft_state, target_body_state, mu)?;

    let soi_radius = sphere_of_influence(
        target_body_state.semi_major_axis(mu),
        target_mass,
        parent_mass,
    );
    let target_peri = (target_body_radius * 1.2).min(soi_radius * 0.5);

    let chop = Porkchop::compute(
        current_et,
        w.sweep,
        w.tof_min,
        w.tof_max,
        100,
        20,
        |et, tof| {
            let craft = craft_state.propagate(et, mu).ok()?;
            let target = target_body_state
                .propagate(et + EphemerisTime::from_years(tof), mu)
                .ok()?;

            best_branch(|k| {
                let (v1, _) = aim_for_periapsis(
                    craft.r,
                    target,
                    tof,
                    mu,
                    target_mu,
                    soi_radius,
                    target_peri,
                    0.0,
                    k,
                )?;
                let depart_dv = v1 - craft.v;
                let arrival_dv = 0.0;
                Some(Cell {
                    total: depart_dv.norm() + arrival_dv,
                    depart_dv,
                    arrival_dv,
                })
            })
        },
    );
    let (i, j, cell) = chop.best(&objective).ok_or("no feasible transfer found")?;
    let depart_et = chop.depart_at(i);
    let tof = chop.tof_at(j);

    let mut transfer_state = craft_state.propagate(depart_et, mu)?;
    transfer_state.v += cell.depart_dv;

    let arrival_et = find_soi_entry(&transfer_state, target_body_state, soi_radius, tof, mu)?;
    let flyby_state = get_flyby_state(&transfer_state, target_body_state, arrival_et, mu)?;

    let exit_state =
        get_grandparent_state(&flyby_state, target_body_state, soi_radius, mu, target_mu)?;

    Ok(FlybyPlan {
        transfer_state,
        flyby_state,
        exit_state,
        soi_radius,
        transfer_dv: cell.depart_dv.norm() * METERS_PER_SECOND_PER_EARTH_RADII_PER_YEAR,
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
    let mut aim = target.r; // we target the body directly on the first pass
    let mut out = None;

    // 3 iterations seems to be good enough from playtesting, could maybe use some test cases
    for _ in 0..3 {
        let (v1, v2) = lambert(r_craft, aim, tof, mu, kind)?;
        let v_inf_vec = v2 - target.v;
        let v_inf = v_inf_vec.norm();

        let b_max = soi_radius * 0.7;
        let b = impact_parameter(target_peri, v_inf, target_mu).min(b_max); // clamp to be within the SOI

        // Build the B-plane basis
        let s_hat = v_inf_vec / v_inf;
        let h_ref = target.r.cross(&target.v).normalize();
        let t_hat = s_hat.cross(&h_ref).normalize();
        let r_hat = s_hat.cross(&t_hat);

        // theta selects leading vs trailing, above vs below
        let b_vec = b * (theta.cos() * t_hat + theta.sin() * r_hat);

        // nudge the aim point toward our desired flyby, resolve
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
