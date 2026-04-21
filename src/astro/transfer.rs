use nalgebra_glm::vec3;

use crate::astro::{epoch::EphemerisTime, lambert::standard, state::State};

pub struct TransferInfo {
    pub transfer_state: State,
}

pub fn plan_transfer(init_state: &State, current_et: EphemerisTime, mu: f64) -> TransferInfo {
    let departure_et = current_et + EphemerisTime::from_years(1.0 / 365.0);
    let arrival_et = current_et + EphemerisTime::from_years(2.0 / 365.0);

    let mut depart_state = init_state.propagate(departure_et, mu);
    let arrival_state = init_state.propagate(arrival_et, mu);

    // Get the dv to get from r1 to r2 in dt
    let dv = standard(
        depart_state.r,
        arrival_state.r,
        (arrival_et - departure_et).as_years(),
        mu,
        crate::astro::lambert::TransferKind::Auto,
    );

    depart_state.v = dv.v_init;

    println!("init: {:?}", init_state);
    println!("xfer: {:?}", depart_state);
    println!("xfer: {:?}", arrival_state);

    TransferInfo {
        transfer_state: depart_state,
    }
}
