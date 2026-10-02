//! Minimal fixture demonstrating the two trait-relationship cases:
//!
//! 1. Concrete impl: `impl IUi for ConsoleUi` -> `ConsoleUi ..|> IUi`
//! 2. Generic bound on a field: `struct Blinky<UiG: IUi> { ui: UiG }`
//!    -> `Blinky ..|> IUi` (the regression target; currently missing).
//!
//! `Blinky` depends on `IUi` ONLY through the generic parameter `UiG`, never
//! naming `IUi` in a field type or method signature, so a naive analyzer that
//! only follows concrete field types reports no relationship.

mod application;
mod application_layer;
mod device_layer;

use application::Manager;

fn main() {
    let mut manager = Manager::new();
    manager.run();
}