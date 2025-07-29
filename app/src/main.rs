use gfx::gui::interface::{self, Alignment, Coordinate, Element, HorizontalAlignment, Interface, Panel, VerticalAlignment};

use crate::window::window::run;

mod window;

fn main() {
    let mut gui_interface = Interface::new();
    let mut panel = Panel::new(Coordinate::new(0.0, 0.0), Coordinate::new(0.6, 0.6));

    let element = Element::new(Coordinate::new(0.1, 0.1), Coordinate::new(0.6, 0.6), interface::Color::new(0.0, 0.0, 1.0))
        .with_text(Alignment { horizontal: HorizontalAlignment::Right, vertical: VerticalAlignment::Bottom});
    let element_1 = Element::new(Coordinate::new(0.6, 0.6), Coordinate::new(0.9, 0.9), interface::Color::new(0.0, 1.0, 0.0))
        .with_text(Alignment { horizontal: HorizontalAlignment::Left, vertical: VerticalAlignment::Top});

    panel.add_element(element);
    panel.add_element(element_1);

    gui_interface.add_panel(panel);

    run(gui_interface).unwrap();
}