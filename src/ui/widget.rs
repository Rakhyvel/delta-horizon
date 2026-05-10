use apricot::app::App;
use nalgebra_glm::Vec2;

use crate::ui::msg::MsgQueue;

pub trait Widget<Msg: Clone + 'static> {
    /// Runs before update(). Overlay widgets (e.g. open dropdowns) use this to claim clicks
    /// before siblings process them. Default is a no-op.
    fn overlay_update(&mut self, _app: &App, _msgq: &mut MsgQueue<Msg>) {}

    /// Updates the Widget's internal state, and collects any messages into the queue
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>);

    /// Renders the widget to the screen
    fn render(&self, app: &App);

    /// Returns the on-screen size of the widget
    fn size(&self) -> Vec2;

    /// Updates the position of the widget relative to its parent
    fn layout(&mut self, pos: Vec2);
}

pub fn recv_msgs<Msg: Clone + 'static>(app: &App, widget: &mut impl Widget<Msg>) -> MsgQueue<Msg> {
    let mut msgq = MsgQueue::new();
    widget.overlay_update(app, &mut msgq);
    widget.update(app, &mut msgq);
    msgq
}
