use std::f64::consts::PI;

use nalgebra_glm::DVec3;

use crate::astro::{epoch::EphemerisTime, state::State};

const TAU: f64 = 2.0 * PI;
const LAMBERT_EPSILON: f64 = 1e-4; // General epsilon
const LAMBERT_EPSILON_TIME: f64 = 1e-4; // Time epsilon
const LAMBERT_EPSILON_RAD: f64 = (5e-5 / 180.0) * PI; // 0.00005 degrees

/// Define the transfer kind for a Lambert
pub enum TransferKind {
    Auto,
    ShortWay,
    LongWay,
    NRevs(u8),
}

#[derive(Debug)]
pub struct LambertSolution {
    pub v_init: DVec3,
    pub v_final: DVec3,
    pub phi: f64,
}

/// Solves the Lambert boundary problem using a standard secant method.
/// Given the initial and final radii, a time of flight, and a gravitational parameters, it returns the needed initial and final velocities
/// along with φ which is the square of the difference in eccentric anomaly. Note that the direction of motion
/// is computed directly in this function to simplify the generation of Pork chop plots.
pub fn standard(
    r_init: DVec3,
    r_final: DVec3,
    tof: f64,
    gm: f64,
    kind: TransferKind,
) -> LambertSolution {
    let r_init_norm = r_init.norm();
    let r_final_norm = r_final.norm();

    let cos_dnu = r_init.dot(&r_final) / (r_init_norm * r_final_norm);

    let dm = match kind {
        TransferKind::Auto => {
            let mut dnu = r_final[1].atan2(r_final[0]) - r_init[1].atan2(r_final[1]);
            if dnu > TAU {
                dnu -= TAU;
            } else if dnu < 0.0 {
                dnu += TAU;
            }

            if dnu > std::f64::consts::PI {
                -1.0
            } else {
                1.0
            }
        }
        TransferKind::ShortWay => 1.0,
        TransferKind::LongWay => -1.0,
        _ => panic!("multi rev not supported"),
    };

    // Compute the direction of motion
    let nu_init = r_init[1].atan2(r_init[0]);
    let nu_final = r_final[1].atan2(r_final[0]);

    let a = dm * (r_init_norm * r_final_norm * (1.0 + cos_dnu)).sqrt();

    if nu_final - nu_init < LAMBERT_EPSILON_RAD && a.abs() < LAMBERT_EPSILON {
        panic!("targets too close");
    }

    // Define the search space (note that we do not support multirevs in this algorithm)
    let mut phi_upper = 4.0 * PI.powi(2);
    let mut phi_lower = -4.0 * PI.powi(2);
    let mut phi = 0.0; // ??!?

    // Initial guesses for c2 and c3
    let mut c2: f64 = 1.0 / 2.0;
    let mut c3: f64 = 1.0 / 6.0;
    let mut iter: usize = 0;
    let mut cur_tof: f64 = 0.0;
    let mut y = 0.0;

    while (cur_tof - tof).abs() > LAMBERT_EPSILON_TIME {
        if iter > 1000 {
            return panic!("Lambert solver failed after {} iterations", 1000);
        }
        iter += 1;

        y = r_init_norm + r_final_norm + a * (phi * c3 - 1.0) / c2.sqrt();
        if a > 0.0 && y < 0.0 {
            // Try to increase phi
            for _ in 0..500 {
                phi += 0.1;
                // Recompute y
                y = r_init_norm + r_final_norm + a * (phi * c3 - 1.0) / c2.sqrt();
                if y >= 0.0 {
                    break;
                }
            }
            if y < 0.0 {
                // If y is still negative, then our attempts have failed.
                panic!("y still negative");
            }
        }

        let chi = (y / c2).sqrt();
        // Compute the current time of flight
        cur_tof = (chi.powi(3) * c3 + a * y.sqrt()) / gm.sqrt();
        // Update the next TOF we should use
        if cur_tof < tof {
            phi_lower = phi;
        } else {
            phi_upper = phi;
        }

        // Compute the next phi
        phi = (phi_upper + phi_lower) / 2.0;

        // Update c2 and c3
        if phi > LAMBERT_EPSILON {
            let sqrt_phi = phi.sqrt();
            let (s_sphi, c_sphi) = sqrt_phi.sin_cos();
            c2 = (1.0 - c_sphi) / phi;
            c3 = (sqrt_phi - s_sphi) / phi.powi(3).sqrt();
        } else if phi < -LAMBERT_EPSILON {
            let sqrt_phi = (-phi).sqrt();
            c2 = (1.0 - sqrt_phi.cosh()) / phi;
            c3 = (sqrt_phi.sinh() - sqrt_phi) / (-phi).powi(3).sqrt();
        } else {
            // Reset c2 and c3 and try again
            c2 = 0.5;
            c3 = 1.0 / 6.0;
        }
    }

    // Time of flight matches!

    let f = 1.0 - y / r_init_norm;
    let g_dot = 1.0 - y / r_final_norm;
    let g = a * (y / gm).sqrt();

    // Compute velocities
    LambertSolution {
        v_init: (r_final - f * r_init) / g,
        v_final: (1.0 / g) * (g_dot * r_final - r_init),
        phi,
    }
}

#[test]
fn test_lambert_recovers_velocity() {
    let mu = 1.0;
    let r = 2.0;
    let init_state = State {
        r: DVec3::new(r, 0.0, 0.0),
        v: DVec3::new(0.0, (mu / r).sqrt(), 0.0),
        t: EphemerisTime::new(0),
    };

    // Use a quarter period so r1 and r2 are 90 degrees apart
    let period = 2.0 * PI * (r.powi(3) / mu).sqrt();
    let departure_et = EphemerisTime::new(0);
    let arrival_et = EphemerisTime::from_years(period / 4.0);
    let tof = (arrival_et - departure_et).as_years();

    let depart_state = init_state.propagate(departure_et, mu);
    let arrival_state = init_state.propagate(arrival_et, mu);

    let v_lambert = standard(depart_state.r, arrival_state.r, tof, mu, TransferKind::Auto);

    let err = (v_lambert.v_init - depart_state.v).norm();
    println!("lambert v: {:?}", v_lambert);
    println!("true    v: {:?}", depart_state.v);
    println!("error:     {err:.2e}");
    assert!(err < 1e-6, "lambert velocity error too large: {err}");
}
