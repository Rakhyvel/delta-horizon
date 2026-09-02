use hecs::World;

pub struct Binding(Box<dyn Fn(&World)>);

impl Binding {
    pub fn new(f: impl Fn(&World) + 'static) -> Self {
        Self(Box::new(f))
    }

    pub fn sync(&self, world: &World) {
        (self.0)(world)
    }
}
