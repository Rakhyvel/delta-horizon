use std::f64::consts::PI;

use nalgebra_glm::{quat_angle_axis, quat_rotate_vec3, rotation, vec3, DVec3};

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
        Self::from_kepler(r, 0.0, 0.0, 0.0, 0.0, 0.0, t, mu)
    }

    #[allow(clippy::too_many_arguments)] // TODO: Kepler struct?
    pub fn from_kepler(
        a: f64,
        e: f64,
        i: f64,
        raan: f64,
        arg_peri: f64,
        true_anomaly: f64,
        t: EphemerisTime,
        mu: f64,
    ) -> Self {
        // Position in perifocal frame
        let p = a * (1.0 - e * e);
        let r_p = p / (1.0 + e * true_anomaly.cos());
        let r_pf = vec3(r_p * true_anomaly.cos(), r_p * true_anomaly.sin(), 0.0);

        // Velocity in perifocal frame
        let h = (mu * p).sqrt();
        let v_pf = vec3(
            -mu / h * true_anomaly.sin(),
            mu / h * (e + true_anomaly.cos()),
            0.0,
        );

        // Rotate to inertial frame
        let q_raan = quat_angle_axis(raan, &vec3(0.0, 0.0, 1.0));
        let q_i = quat_angle_axis(i, &vec3(1.0, 0.0, 0.0));
        let q_argp = quat_angle_axis(arg_peri, &vec3(0.0, 0.0, 1.0));
        let q = q_raan * q_i * q_argp;

        let r = quat_rotate_vec3(&q, &r_pf);
        let v = quat_rotate_vec3(&q, &v_pf);

        Self { r, v, t }
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
        let mut chi = if alpha > 0.0 {
            // Elliptic: seed with circular orbit chi
            mu.sqrt() * dt * alpha
        } else {
            // Hyperbolic/parabolic: seed conservatively
            mu.sqrt() * dt / r0_mag
        };

        // newton rhapson, find chi that makes F 0
        const MAX_ITER: usize = 500;
        const TOL: f64 = 1e-8;
        for iter in 0..MAX_ITER {
            let chi2 = chi * chi;
            let z = alpha * chi2;
            assert!(z.is_finite());

            let c = stumpff_c(z);
            if !c.is_finite() {
                println!("{c} {z} {alpha} {chi2}");
            }
            assert!(c.is_finite());
            let s = stumpff_s(z);
            assert!(s.is_finite());

            let r0_vr0_over_sqrtmu = r0_mag * vr0 / mu.sqrt();

            let f = r0_vr0_over_sqrtmu * chi2 * c
                + (1.0 - alpha * r0_mag) * chi2 * chi * s
                + r0_mag * chi
                - (mu.sqrt() * dt);
            assert!(f.is_finite());

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
                eprintln!("universal kepler equation did not converge: {self:?}");
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

    pub fn generate_orbit_vertices(
        &self,
        segments: i32,
        mu: f64,
        soi_radius: Option<f64>,
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
            let soi_radius = soi_radius.expect("must specify a SOI radius for hyperbolic orbits");
            for i in 0..=segments {
                let t = i as f64 / segments as f64;
                let sample_time = self.t + EphemerisTime::from_years(2.0 * t / 365.0);

                let pos = self.propagate(sample_time, mu).r;
                let distance = pos.norm();

                if distance >= soi_radius {
                    break;
                }

                vertices.push(pos.x as f32);
                vertices.push(pos.y as f32);
                vertices.push(pos.z as f32);
            }
        }

        vertices
    }

    pub fn semi_major_axis(&self, mu: f64) -> f64 {
        let r_mag = self.r.norm();
        1.0 / (2.0 / r_mag - self.v.norm_squared() / mu)
    }

    pub fn ecc(&self, mu: f64) -> f64 {
        let r_mag = self.r.norm();

        let v_cross_h = self.v.cross(&(self.r.cross(&self.v)));

        let e_vec = (v_cross_h / mu) - (self.r / r_mag);

        e_vec.norm()
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
