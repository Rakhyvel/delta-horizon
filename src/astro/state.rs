use std::f64::consts::PI;

use nalgebra_glm::{vec3, DVec3};

use crate::astro::epoch::EphemerisTime;

#[derive(Debug, Clone, Copy)]
pub struct State {
    pub r: DVec3,
    pub v: DVec3,
    pub t: EphemerisTime,
}

impl State {
    /// Create a state along a circular orbit, at some orbital radius `r`, time `t`, and with the parent grav
    /// param `mu`.
    pub fn circular(r: f64, t: EphemerisTime, mu: f64) -> Self {
        let v_mag = (mu / r).sqrt();

        let r_vec: DVec3 = vec3(r, 0.0, 0.0);
        let v_vec: DVec3 = vec3(0.0, v_mag, 0.0);

        Self {
            r: r_vec,
            v: v_vec,
            t,
        }
    }

    /// Returns the ephemeris at some `t` given some mu
    pub fn propagate(&self, t: EphemerisTime, mu: f64) -> State {
        let dt = (t - self.t).as_years();

        assert!(mu > 0.0); // mu must be positive

        let r0_mag = self.r.norm();
        let v0_mag = self.v.norm();

        assert!(r0_mag > 0.0); // r0_mag must be positive

        let vr0 = self.r.dot(&self.v) / r0_mag; // radial velocity
        let alpha = 2.0 / r0_mag - (v0_mag * v0_mag) / mu; // 1/a (specific energy form)

        // Newton solver for chi (TODO: look into different initial guesses)
        let mut chi = mu.sqrt() * alpha.abs() * dt;

        // newton rhapson, find chi that makes F 0
        const MAX_ITER: usize = 500;
        const TOL: f64 = 1e-6;
        for iter in 0..MAX_ITER {
            let chi2 = chi * chi;
            let z = alpha * chi2;

            let c = stumpff_c(z);
            let s = stumpff_s(z);

            let r0_vr0_over_sqrtmu = r0_mag * vr0 / mu.sqrt();

            let f = r0_vr0_over_sqrtmu * chi2 * c
                + (1.0 - alpha * r0_mag) * chi2 * chi * s
                + r0_mag * chi
                - (mu.sqrt() * dt);

            if f.abs() < TOL {
                break;
            }

            // derivative of F wrt chi
            let df_dchi = r0_vr0_over_sqrtmu * chi * (1.0 - alpha * chi2 * s)
                + (1.0 - alpha * r0_mag) * chi2 * c
                + r0_mag;

            let delta = (f / df_dchi).clamp(-1.0, 1.0);
            const DAMPING: f64 = 0.8; // Tweak if necessary
            chi -= DAMPING * delta;

            if delta.abs() < TOL {
                break;
            }

            if iter == MAX_ITER - 1 {
                panic!("universal kepler equation did not converge");
            }
        }

        let chi2 = chi * chi;

        let z = alpha * chi2;
        let c = stumpff_c(z);
        let s = stumpff_s(z);

        // find f and g
        let f = 1.0 - (chi2 / r0_mag) * c;
        let g = dt - (1.0 / mu.sqrt()) * chi2 * chi * s;

        // position at t
        let r = self.r * f + self.v * g;
        let r_mag = r.norm();

        // derive fdot from position rather than chi for better numerical stability
        let fdot = (mu.sqrt() / (r_mag * r0_mag)) * chi * (z * s - 1.0);
        let gdot = 1.0 - (chi2 / r_mag) * c;

        let v = self.r * fdot + self.v * gdot;

        State { r, v, t }
    }

    /// Returns the period of the orbit is periodic, otherwise returns None
    pub fn period(&self, mu: f64) -> Option<f64> {
        let r = self.r.norm();
        let v2 = self.v.dot(&self.v);
        let a = 1.0 / (2.0 / r - v2 / mu); // semi-major axis from vis-viva

        if a <= 0.0 {
            // a < 0 -> hyperbolic, a = 0 -> parabolic (1/a = 0 means v = escape velocity)
            return None;
        }

        Some(2.0 * PI * (a.powi(3) / mu).sqrt())
    }

    pub fn generate_orbit_vertices(
        &self,
        segments: i32,
        mu: f64,
        _soi_radius: Option<f64>,
    ) -> Vec<f32> {
        // Find period, if periodic, do the loop
        // If not periodic, clip to SOI

        let mut vertices = Vec::with_capacity((segments as usize + 1) * 3);

        if let Some(period) = self.period(mu) {
            let mut et = EphemerisTime::new(0);

            for _ in 0..=segments {
                let new_state = self.propagate(et, mu);

                vertices.push(new_state.r.x as f32);
                vertices.push(new_state.r.y as f32);
                vertices.push(new_state.r.z as f32);

                et += EphemerisTime::from_years(period / (segments as f64));
            }
        } else {
            eprintln!("TODO: Hyperbolic and parabolic orbits");
        }

        vertices
    }
}

fn stumpff_c(z: f64) -> f64 {
    if z > 0.0 {
        let sz = z.sqrt();
        (1.0 - sz.cos()) / z
    } else if z < 0.0 {
        let sz = (-z).sqrt();
        (sz.cosh() - 1.0) / (-z)
    } else {
        0.5
    }
}

fn stumpff_s(z: f64) -> f64 {
    if z > 0.0 {
        let sz = z.sqrt();
        (sz - sz.sin()) / (sz.powi(3))
    } else if z < 0.0 {
        let sz = (-z).sqrt();
        (sz.sinh() - sz) / (sz.powi(3))
    } else {
        1.0 / 6.0
    }
}
