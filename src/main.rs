mod astro;
mod components;
mod generation;
mod scenes;
mod ui;

use std::cell::RefCell;

use apricot::app::run;
use scenes::gameplay::Gameplay;

fn main() -> Result<(), String> {
    // Start Apricot's game loop
    run(
        nalgebra_glm::I32Vec2::new(960, 720),
        "Delta Horizon", // singular, 1 horizon
        // delta horizon is already taken, here are some other good names that are taken
        // - vis-viva <= holy moly
        // - aphelion
        // - ephemeris
        // - aurelion
        // - pericynthion <= not taken but hard to spell
        // - apolune
        apricot::app::AppConfig { mouse_warp: false },
        &|app| RefCell::new(Box::new(Gameplay::new(app))),
    )
}
