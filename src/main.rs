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
        nalgebra_glm::I32Vec2::new(1280, 720),
        "Vis-Viva",
        apricot::app::AppConfig { mouse_warp: false },
        &|app| RefCell::new(Box::new(Gameplay::new(app))),
    )
}
