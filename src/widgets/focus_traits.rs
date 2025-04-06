pub trait WidgetFocusTrait {
    fn focus_on(&mut self, selected: bool);
    fn is_focused(&self) -> bool;
}