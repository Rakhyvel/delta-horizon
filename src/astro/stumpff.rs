const STUMPFF_EPSILON: f64 = 1.0e-8;

pub fn stumpff_c2_c3(z: f64) -> (f64, f64) {
    if z.abs() < STUMPFF_EPSILON {
        let z2 = z * z;
        let z3 = z2 * z;

        let c2 = 0.5 - z / 24.0 + z2 / 720.0 - z3 / 40320.0;
        let c3 = 1.0 / 6.0 - z / 120.0 + z2 / 5040.0 - z3 / 362880.0;

        (c2, c3)
    } else if z > 0.0 {
        let sz = z.sqrt();
        let (sin_sz, cos_sz) = sz.sin_cos();

        let c2 = (1.0 - cos_sz) / z;
        let c3 = (sz - sin_sz) / (z * sz);

        (c2, c3)
    } else {
        let sz = (-z).sqrt();

        let c2 = (1.0 - sz.cosh()) / z;
        let c3 = (sz.sinh() - sz) / ((-z) * sz);

        (c2, c3)
    }
}

pub fn stumpff_c(z: f64) -> f64 {
    stumpff_c2_c3(z).0
}

pub fn stumpff_s(z: f64) -> f64 {
    stumpff_c2_c3(z).1
}
