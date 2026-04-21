use std::f64::consts::PI;

use nalgebra_glm::DVec3;

/// Solves lambert's problem using Izzo's algorithm
pub fn lambert(r1: DVec3, r2: DVec3, dt: f64, mu: f64) -> DVec3 {
    let r1_mag = r1.norm();
    let r2_mag = r2.norm();

    // Determine transfer angle (always take propgrade/short way here)
    let cos_dnu = r1.dot(&r2) / (r1_mag * r2_mag);
    let cross = r1.cross(&r2);
    // Prograde if cross.z > 0 (same as orbit direction), else retrograde
    let dnu = if cross.z >= 0.0 {
        cos_dnu.acos()
    } else {
        2.0 * PI - cos_dnu.acos()
    };

    let dm = if dnu < PI { 1.0 } else { -1.0 }; // +1 propgrade short, -1 retrograde

    // Battin's lambda parameter
    let s = (r1_mag + r2_mag + (r2 - r1).norm()) / 2.0; // semi-parameter
    let lam2 = 1.0 - (r2 - r1).norm() / s; // Not quite Izzo's lambda yet!
                                           // Izzo lambda
    let lam = dm * lam2.sqrt();
    let t_norm = dt * (2.0 * mu / s.powi(3)).sqrt(); // normalize time

    // Solve for x via Halley's method on the time-of-flight equation
    // Initial guess (Lancaster and Blanchard)
    let mut x = 0.0_f64; // x in (-1, 1) for elliptic

    for _ in 0..50 {
        let (tof, dtof_dx, d2tof_dx2) = tof_and_derivs(x, lam, t_norm);
        let err = tof - t_norm;
        if err.abs() < 1e-12 {
            break;
        }

        // Halley step
        let dx = err * dtof_dx / (dtof_dx * dtof_dx - 0.5 * err * d2tof_dx2);
        x -= dx;
        x = x.clamp(-0.99, 0.99);
    }

    // Recover gamma, rho, sigma, from x and lambda
    let gamma = (mu * s / 2.0).sqrt();
    let rho = (r1_mag - r2_mag) / (r2 - r1).norm();
    let sigma = (1.0 - rho * rho).sqrt();

    let y = (1.0 - lam * lam + lam * lam * x * x).sqrt();
    let vr1 = gamma * ((lam * y - x) - rho * (lam * y + x)) / r1_mag;
    let vt1 = gamma * sigma * (y + lam * x) / r1_mag;

    // Tangential unit vector at r1
    let r1_hat = r1 / r1_mag;
    // Compute tangential direction, perpendicular to r1 in the orbital plane
    let r2_hat = r2 / r2_mag;
    let t1_hat = (r2_hat - r1_hat * cos_dnu).normalize(); // Gram-Schmidt

    r1_hat * vr1 + t1_hat * vt1
}

fn tof_and_derivs(x: f64, lam: f64, _t_norm: f64) -> (f64, f64, f64) {
    let a = 1.0 / (1.0 - x * x);
    let (e, de_dx, d2e_dx2) = if x.abs() < 1.0 {
        // Elliptic, use Lagrange's expansion
        let alpha = x.acos();
        let beta = 2.0 * (lam * (1.0 - x * x).sqrt()).asin();
        let tof = (alpha - beta - (alpha - beta).sin()) / (1.0 - x * x).powf(1.5);
        // Numerical derivatives (simple finite diff for clarity)
        let h = 1e-7;
        let tof_p = {
            let xp = x + h;
            let ap = 1.0 / (1.0 - xp * xp);
            let _ = ap;
            let alphap = xp.acos();
            let betap = 2.0 * (lam * (1.0 - xp * xp).sqrt()).asin();
            (alphap - betap - (alphap - betap).sin()) / (1.0 - xp * xp).powf(1.5)
        };
        let tof_m = {
            let xm = x - h;
            let alpham = xm.acos();
            let betam = 2.0 * (lam * (1.0 - xm * xm).sqrt()).asin();
            (alpham - betam - (alpham - betam).sin()) / (1.0 - xm * xm).powf(1.5)
        };
        (
            tof,
            (tof_p - tof_m) / (2.0 * h),
            (tof_p - 2.0 * tof + tof_m) / (h * h),
        )
    } else {
        // Hyperbolic branch (x > 1 region, shouldn't hit for short-way Hohman but whatevs)
        let g = (x * x - 1.0).sqrt();
        let tof = (g - lam * x - (x * g - lam * (x * x - 1.0)).ln()) / g.powi(3);
        let h = 1e-7;
        let tof_p = {
            let xp = x + h;
            let gp = (xp * xp - 1.0).sqrt();
            (gp - lam * xp - (xp * gp - lam * (xp * xp - 1.0)).ln()) / gp.powi(3)
        };
        let tof_m = {
            let xm = x - h;
            let gm = (xm * xm - 1.0).sqrt();
            (gm - lam * xm - (xm * gm - lam * (xm * xm - 1.0)).ln()) / gm.powi(3)
        };
        (
            tof,
            (tof_p - tof_m) / (2.0 * h),
            (tof_p - 2.0 * tof + tof_m) / (h * h),
        )
    };
    (e, de_dx, d2e_dx2)
}
