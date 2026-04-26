use std::f64::consts::PI;

use nalgebra_glm::DVec3;

#[allow(unused)]
use crate::astro::{epoch::EphemerisTime, state::State};

const LAMBERT_EPSILON: f64 = 1e-4; // General epsilon

/// Solve using Vallado's algorithm
pub fn lambert(r1: DVec3, r2: DVec3, tof: f64, mu: f64) -> DVec3 {
    assert!(tof > 0.0, "tof was non-positive");
    assert!(mu > 0.0, "grav param was non-positive");

    let r1_mag = r1.norm();
    let r2_mag = r2.norm();

    let cos_dnu = r1.dot(&r2) / (r1_mag * r2_mag);
    let a = (r1_mag * r2_mag * (1.0 + cos_dnu)).sqrt(); // always take the short way
    assert!(a > LAMBERT_EPSILON, "transfer angle too close to 0 or 180");

    const TOL: f64 = 1e-4; // Time epsilon

    let mut phi_upper = 4.0 * PI * PI;
    let mut phi_lower = -4.0 * PI * PI;
    let mut phi = 0.0;
    let mut c2 = 0.5_f64;
    let mut c3 = 1.0_f64 / 6.0;
    let mut cur_tof;
    let mut y = 0.0;
    for _ in 0..1000 {
        y = r1_mag + r2_mag + a * (phi * c3 - 1.0) / c2.sqrt();

        if a > 0.0 && y < 0.0 {
            // nudge phi until y is positive
            for _ in 0..500 {
                phi += 0.1;
                y = r1_mag + r2_mag + a * (phi * c3 - 1.0) / c2.sqrt();
                if y >= 0.0 {
                    break;
                }
            }
            assert!(y >= 0.0, "lambert: could not find reasonable phi");
        }

        cur_tof = ((y / c2).sqrt().powi(3) * c3 + a * y.sqrt()) / mu.sqrt();

        if (cur_tof - tof).abs() < TOL {
            break;
        }

        if cur_tof < tof {
            phi_lower = phi;
        } else {
            phi_upper = phi;
        }
        phi = (phi_upper + phi_lower) / 2.0;

        (c2, c3) = stumpff_c2_c3(phi);
    }

    let f = 1.0 - y / r1_mag;
    let g = a * (y / mu).sqrt();

    (r2 - f * r1) / g
}

fn stumpff_c2_c3(phi: f64) -> (f64, f64) {
    if phi > LAMBERT_EPSILON {
        let sp = phi.sqrt();
        let (s, c) = sp.sin_cos();
        ((1.0 - c) / phi, (sp - s) / phi.powi(3).sqrt())
    } else if phi < -LAMBERT_EPSILON {
        let sp = (-phi).sqrt();
        (
            (1.0 - sp.cosh()) / phi,
            (sp.sinh() - sp) / (-phi).powi(3).sqrt(),
        )
    } else {
        (0.5, 1.0 / 6.0)
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

    let depart_state = init_state.propagate(departure_et, mu).unwrap();
    let arrival_state = init_state.propagate(arrival_et, mu).unwrap();

    let v_lambert = lambert(depart_state.r, arrival_state.r, tof, mu);

    let err = (v_lambert - depart_state.v).norm();
    println!("lambert v: {:?}", v_lambert);
    println!("true    v: {:?}", depart_state.v);
    println!("error:     {err:.2e}");
    assert!(err < 1e-6, "lambert velocity error too large: {err}");
}
