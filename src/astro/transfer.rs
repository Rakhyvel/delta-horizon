use nalgebra_glm::vec3;

use crate::astro::{epoch::EphemerisTime, state::State};

pub struct TransferInfo {
    pub transfer_state: State,
}

pub fn plan_transfer(init_state: &State, current_et: EphemerisTime, mu: f64) -> TransferInfo {
    let departure_et = current_et + EphemerisTime::from_years(1.0 / 365.0);
    let mut transfer_state = init_state.propagate(departure_et, mu);
    transfer_state.v += vec3(0.0, 10000.0, 10000.0);

    println!("init: {:?}", init_state);
    println!("xfer: {:?}", transfer_state);

    TransferInfo { transfer_state }
}
