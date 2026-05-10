use nalgebra_glm::vec4;
use nalgebra_glm::Vec4;

pub struct Style {
    // Backgrounds
    pub bg_primary: Vec4,
    pub bg_secondary: Vec4,
    pub bg_hover: Vec4,
    pub bg_danger: Vec4,

    // Text
    pub text_primary: Vec4,
    pub text_secondary: Vec4,
    pub text_disabled: Vec4,

    // Button states
    pub btn_active_bg: Vec4,
    pub btn_active_border: Vec4,
    pub btn_inactive_bg: Vec4,
    pub btn_inactive_border: Vec4,
    pub btn_hover_bg: Vec4,
    pub btn_accent_bg: Vec4,
    pub btn_accent_border: Vec4,
    pub btn_accent_hover_bg: Vec4,

    // Borders
    pub border_primary: Vec4,
    pub border_accent: Vec4,

    // Accents
    pub accent: Vec4,
    pub accent_hover: Vec4,

    // Status colors
    pub positive: Vec4, // green, good delta-v etc
    pub warning: Vec4,  // yellow
    pub negative: Vec4, // red, low fuel etc
}

lazy_static::lazy_static! {
pub static ref STYLE: Style = Style {
    bg_primary: vec4(3.17, 12.6, 19.6, 216.0) / 255.0,
    bg_secondary: vec4(1.0, 0.0, 1.0, 1.0),
    bg_hover: vec4(19.0, 30.0, 40.0, 220.0) / 255.0,
    bg_danger: vec4(1.0, 0.0, 1.0, 1.0),

    text_primary: vec4(1.00, 1.00, 1.00, 1.00),
    text_secondary: vec4(1.0, 0.0, 1.0, 1.0),
    text_disabled: vec4(0.6, 0.6, 0.6, 0.5),

    border_primary: vec4(36.0, 49.0, 62.0, 255.0) / 255.0,
    border_accent: vec4(91.0, 160.0, 229.0, 255.0) / 255.0,

    btn_active_bg:       vec4(3.17, 12.6, 19.6, 216.0) / 255.0,
    btn_active_border:   vec4(36.0, 49.0, 62.0, 255.0) / 255.0,
    btn_inactive_bg:     vec4(12.0, 20.0, 28.0, 200.0) / 255.0,
    btn_inactive_border: vec4(28.0, 38.0, 48.0, 180.0) / 255.0,
    btn_hover_bg:        vec4(19.0, 30.0, 40.0, 220.0) / 255.0,
    btn_accent_bg:       vec4(20.0, 60.0, 100.0, 220.0) / 255.0,
    btn_accent_border:   vec4(91.0, 160.0, 229.0, 255.0) / 255.0,
    btn_accent_hover_bg: vec4(35.0, 80.0, 130.0, 240.0) / 255.0,

    accent: vec4(14.0, 135.0, 217.0, 216.0) / 255.0,
    accent_hover: vec4(1.0, 0.0, 1.0, 1.0),

    positive: vec4(0.20, 0.75, 0.40, 1.00),
    warning: vec4(0.90, 0.70, 0.10, 1.00),
    negative: vec4(0.85, 0.25, 0.20, 1.00),
};
}
