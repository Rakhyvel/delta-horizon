use std::f64::consts::PI;

use nalgebra_glm::vec3;

use crate::astro::{epoch::EphemerisTime, lambert::lambert, state::State};

pub struct TransferInfo {
    pub transfer_state: State,
    pub arrival_et: EphemerisTime,
}

pub fn plan_transfer(
    init_state: &State,
    target_body_state: &State,
    current_et: EphemerisTime,
    mu: f64,
) -> TransferInfo {
    // Compute transfer geometry from current states (valid since orbits are circular)
    let r1_mag = init_state.r.norm();
    let r2_mag = target_body_state.r.norm();
    let a_transfer = (r1_mag + r2_mag) / 2.0;
    let transfer_duration_years = PI * (a_transfer.powi(3) / mu).sqrt();

    // Angular velocities (rad/yr)
    let omega_craft = (mu / r1_mag.powi(3)).sqrt();
    let omega_target = (mu / r2_mag.powi(3)).sqrt();

    // Target must be this far ahead of craft at departure (spacecraft travels PI radians)
    let required_phase = PI - omega_target * transfer_duration_years;

    // Current phase angles
    let craft_angle = init_state.r.y.atan2(init_state.r.x);
    let target_angle = target_body_state.r.y.atan2(target_body_state.r.x);
    let current_phase = (target_angle - craft_angle).rem_euclid(2.0 * PI);

    // Time until phase matches (how fast the phase angle is closing/opening)
    let phase_rate = omega_target - omega_craft;
    let phase_error = (required_phase - current_phase).rem_euclid(2.0 * PI);
    let wait_years = phase_error / phase_rate;

    let departure_et = current_et + EphemerisTime::from_years(wait_years);
    let arrival_et = departure_et + EphemerisTime::from_years(transfer_duration_years);

    // Propagate to departure
    let craft_at_departure = init_state.propagate(departure_et, mu);
    let r1 = craft_at_departure.r;
    let r2 = target_body_state.propagate(arrival_et, mu).r;

    // Solve Lambert's problem
    let v_departure = lambert(r1, r2, transfer_duration_years, mu);

    let mut transfer_state = craft_at_departure;
    transfer_state.v = v_departure;

    println!("init:       {:?}", init_state);
    println!("departure:  {:?}", craft_at_departure);
    println!("target:     {:?}", r2);
    println!("xfer:       {:?}", transfer_state);
    println!("dv:         {:?}", transfer_state.v - craft_at_departure.v);

    TransferInfo {
        transfer_state,
        arrival_et,
    }
}
