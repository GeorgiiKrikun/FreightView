pub trait WidgetFocusTrait {
    fn focus_on(&mut self, selected: bool);
    #[allow(dead_code)]
    fn is_focused(&self) -> bool;
}