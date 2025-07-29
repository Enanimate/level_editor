use gfx::gui::interface::{self, Alignment, Coordinate, Element, HorizontalAlignment, Interface, Panel, VerticalAlignment};

use crate::window::window::run;

mod window;

fn main() {
    let mut gui_interface = Interface::new();
    let mut panel = Panel::new(Coordinate::new(0.0, 0.0), Coordinate::new(0.6, 0.6));

    let element_refac = Element::new(Coordinate::new(0.1, 0.1), Coordinate::new(0.6, 0.6), interface::Color::new(0.0, 0.0, 1.0))
        .with_text(Alignment { horizontal: HorizontalAlignment::Left, vertical: VerticalAlignment::Bottom});

    panel.add_element(element_refac);

    gui_interface.add_panel(panel);

    run(gui_interface).unwrap();
}