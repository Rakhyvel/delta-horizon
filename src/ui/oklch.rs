use nalgebra_glm::{vec4, Vec4};

pub fn oklch(l: f32, c: f32, h_deg: f32, alpha: f32) -> Vec4 {
    let h = h_deg.to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());

    // OKLab to LMS
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_18 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (lc, mc, sc) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    // LMS to linear sRGB
    let r = 4.076_741_7 * lc - 3.307_711_6 * mc + 0.230_969_94 * sc;
    let g = -1.268_438 * lc + 2.609_757_4 * mc - 0.341_319_38 * sc;
    let bl = -0.0041960863 * lc - 0.703_418_6 * mc + 1.707_614_7 * sc;

    vec4(encode(r), encode(g), encode(bl), alpha)
}

fn encode(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}
