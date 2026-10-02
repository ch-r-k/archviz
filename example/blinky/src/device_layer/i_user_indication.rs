/// Abstraction the application depends on.
pub trait IUi {
    fn render(&mut self);
}