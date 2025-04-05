pub trait WidgetNav
{
    fn next(&mut self);
    fn prev(&mut self);
}

pub trait WidgetNavBounds<WidgetState : WidgetNav> {
    // Because widget bounds are not know before rendering, we need to pass the widget state
    fn ensure_bounds(&self, state: &mut WidgetState);
}