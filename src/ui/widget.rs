use apricot::app::App;

use crate::ui::msg::MsgQueue;

pub trait Widget<Msg: Clone + 'static> {
    fn update(&mut self, app: &App, msgq: &mut MsgQueue<Msg>);
    fn render(&self, app: &App);
}

pub fn recv_msgs<Msg: Clone + 'static>(app: &App, widget: &mut impl Widget<Msg>) -> MsgQueue<Msg> {
    let mut msgq = MsgQueue::new();
    widget.update(app, &mut msgq);
    msgq
}
