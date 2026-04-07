mod components;
mod scenes;

use std::cell::RefCell;

use apricot::app::run;
use scenes::gameplay::Gameplay;

fn main() -> Result<(), String> {
    // Start Apricot's game loop
    run(
        nalgebra_glm::I32Vec2::new(800, 600),
        "Delta Horizon", // singular, horizon
        apricot::app::AppConfig { mouse_warp: false },
        &|app| RefCell::new(Box::new(Gameplay::new(app))),
    )
}
