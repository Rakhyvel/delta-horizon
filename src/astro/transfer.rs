use std::f64::consts::PI;

use nalgebra_glm::{vec3, DVec3, DVec4, TVec};

use crate::astro::{
    epoch::EphemerisTime,
    lambert::lambert,
    newton::{newton_target, NLProblem},
    state::State,
};

pub struct BurnTargeter {
    craft_state_t0: State,
    target_state_t0: State,
    current_et: EphemerisTime,
    tof: f64,
    mu: f64,
}

impl NLProblem<3, 3> for BurnTargeter {
    fn resid(&self, controls: &TVec<f64, 3>) -> TVec<f64, 3> {
        let depart_et = self.current_et + EphemerisTime::from_years(1.0 / 365.0);
        let dv = controls;

        let mut new_state = self.craft_state_t0.propagate(depart_et, self.mu);
        new_state.v += dv;

        let tf = depart_et + EphemerisTime::from_years(self.tof);

        println!("ecc: {}", new_state.ecc(self.mu));

        let craft_state_tf = new_state.propagate(tf, self.mu);
        let target_state_tf = self.target_state_t0.propagate(tf, self.mu);

        craft_state_tf.r - target_state_tf.r
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
    let prob = BurnTargeter {
        craft_state_t0: *init_state,
        target_state_t0: *target_body_state,
        current_et,
        tof: 6.0 / 365.0,
        mu,
    };

    let sol: DVec3 = newton_target(&prob, DVec3::new(0.0, 0.0, 0.0), 1000, 3.0, 0.01).unwrap();

    let depart_et = current_et + EphemerisTime::from_years(1.0 / 365.0);
    let dv = sol;

    let mut transfer_state = init_state.propagate(depart_et, mu);
    transfer_state.v += dv;

    TransferInfo {
        transfer_state,
        arrival_et: depart_et + EphemerisTime::from_years(6.0 / 365.0),
    }
}
