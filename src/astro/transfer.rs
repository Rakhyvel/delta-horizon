use std::f64::consts::PI;

use nalgebra_glm::vec3;

use crate::astro::{epoch::EphemerisTime, lambert::lambert, state::State};

pub struct BurnTargeter {
    craft_state_t0: State,
    target_state_t0: State,
    mu: f64,
}

impl BurnTargeter {
    fn miss(&self, x: [f64; 4]) -> [f64; 3] {
        let depart_et = EphemerisTime::from_years(x[0]);
        let dv = vec3(x[1], x[2], x[3]);

        let mut craft = self.craft_state_t0.propagate(depart_et, self.mu);
        craft.v += dv;

        // TODO: Get pos defects at closest approach
        [0.0, 0.0, 0.0]
    }
}

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
    // Compute transfer geometry from current states
    let r1_mag = init_state.r.norm();
    let r2_mag = target_body_state.r.norm();
    let a1 = 1.0 / (2.0 / r1_mag - init_state.v.norm_squared() / mu);
    let a2 = 1.0 / (2.0 / r2_mag - target_body_state.v.norm_squared() / mu);
    let a_transfer = (a1 + a2) / 2.0;
    let transfer_duration_years = PI * (a_transfer.powi(3) / mu).sqrt();

    // Angular velocities (rad/yr)
    let omega_craft = (mu / a1.powi(3)).sqrt();
    let omega_target = (mu / a2.powi(3)).sqrt();

    // Target must be this far ahead of craft at departure (spacecraft travels PI radians)
    let required_phase = (PI - omega_target * transfer_duration_years).rem_euclid(2.0 * PI);
    assert!(
        required_phase > 0.0 && required_phase < PI,
        "required phase {required_phase} out of range, transfer geometry is wrong"
    );

    // Current phase angles
    let craft_angle = init_state.r.y.atan2(init_state.r.x);
    let target_angle = target_body_state.r.y.atan2(target_body_state.r.x);
    let phase_rate = omega_craft - omega_target;
    let synodic_period = 2.0 * PI / phase_rate;

    let wait_years = (-PI + omega_target * transfer_duration_years - (craft_angle - target_angle))
        .rem_euclid(2.0 * PI)
        / phase_rate;
    let wait_years = if wait_years < 1.0 / 365.0 {
        wait_years + synodic_period
    } else {
        wait_years
    };

    assert!(wait_years > 0.0, "departure_et has gotta be in the future");
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

    println!("r1_mag:                  {}", r1_mag);
    println!("r2_mag:                  {}", r2_mag);
    println!("a_transfer:              {}", a_transfer);
    println!("transfer_duration_years: {}", transfer_duration_years);
    println!("omega_craft:             {}", omega_craft);
    println!("omega_target:            {}", omega_target);
    println!("phase_rate:              {}", phase_rate);
    println!("synodic_period:          {}", synodic_period);
    println!("craft_angle:             {}", craft_angle);
    println!("target_angle:            {}", target_angle);
    println!("wait_years:              {}", wait_years);
    println!("craft_at_departure:      {:?}", craft_at_departure);
    println!("target_at_arrival:       {:?}", r2);
    println!("|r1|:                    {}", r1.norm());
    println!("|r2|:                    {}", r2.norm());
    println!("circular v at r1:        {}", (mu / r1_mag).sqrt());
    println!("lambert v:               {:?}", v_departure);
    println!("|lambert v|:             {}", v_departure.norm());

    TransferInfo {
        transfer_state,
        arrival_et,
    }
}
