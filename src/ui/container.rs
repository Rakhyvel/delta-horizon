use crate::ui::{msg::MsgQueue, widget::Widget};
use apricot::app::App;

pub struct Container<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg> Container<Msg> {
    pub fn new(children: Vec<Box<dyn Widget<Msg>>>) -> Self {
        Self { children }
    }
}

#[macro_export]
macro_rules! container {
    ($($widget:expr),* $(,)?) => {
        Container::new(vec![$(Box::new($widget)),*])
    }
}

impl<Msg: Clone + 'static> Widget<Msg> for Container<Msg> {
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>) {
        for child in self.children.iter_mut() {
            child.as_mut().update(app, msgq);
        }
    }

    fn render(&self, app: &App) {
        for child in self.children.iter() {
            child.as_ref().render(app);
        }
    }
}
