use gfx::gui::{interface::{self, Coordinate, Element, Interface, Panel}};

use crate::window::window::run;

mod window;

fn main() {
    let mut gui_interface = Interface::new();
    let mut panel = Panel::new(Coordinate::new(0.1, 0.1), Coordinate::new(0.2, 0.9));

    let element_refac1 = Element::new(Coordinate::new(0.0, 0.0), Coordinate::new(1.0, 1.0), interface::Color::new(0.0, 1.0, 0.0));
    let element_refac = Element::new(Coordinate::new(0.1, 0.1), Coordinate::new(0.5, 0.5), interface::Color::new(0.0, 0.0, 1.0));

    panel.add_element(element_refac1);
    panel.add_element(element_refac);

    gui_interface.add_panel(panel);

    run(gui_interface).unwrap();
}