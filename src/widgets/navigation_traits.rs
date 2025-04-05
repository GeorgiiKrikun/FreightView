// This file defines the Navigation trait for navigating through a list or tree of items.
pub trait Navigation<WidgetState> {
    fn next(state: &mut WidgetState, max: usize);
    fn prev(state: &mut WidgetState);
}