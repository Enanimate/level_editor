use std::{fs, io, sync::{Arc, Mutex}};

use gfx::{definitions::{GuiEvent, GuiMenuState, GuiPageState, InteractionStyle}, gui::interface::{Alignment, Coordinate, Element, HorizontalAlignment, Interface, Panel, VerticalAlignment}, RenderState};
use winit::{application::ApplicationHandler, dpi::PhysicalPosition, event::{MouseButton, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, window::Window};

use crate::UiAtlas;

pub struct EditorApp {
    layout: GuiPageState,
    interface: Arc<Mutex<Interface>>,
    atlas: Option<UiAtlas>,
    render_state: Option<gfx::RenderState>,
    cursor_position: Option<PhysicalPosition<f64>>,
    window_ref: Option<Arc<Window>>,
    menu_open: (bool, Option<GuiMenuState>),
    last_hovered_element_index: Option<(usize, usize)>,
}

impl EditorApp {
    pub fn new(atlas: UiAtlas) -> anyhow::Result<()> {
        let mut app = EditorApp {
            layout: GuiPageState::ProjectView,
            interface: Arc::new(Mutex::new(Interface::new(atlas.clone()))),
            atlas: Some(atlas),
            render_state: None,
            cursor_position: None,
            window_ref: None,
            menu_open: (false, None),
            last_hovered_element_index: None,
        };

        env_logger::init();

        let event_loop = EventLoop::with_user_event().build()?;

        event_loop.run_app(&mut app)?;

        Ok(())
    }

    fn rebuild_interface(&mut self) {
        println!("Rebuilding interface for layout: {:?}", self.layout);
        let atlas = self.atlas.clone().unwrap();

        let page_interface_data = match self.layout {
            GuiPageState::ProjectView => Self::build_project_view_interface(atlas),
            GuiPageState::FileExplorer => Self::build_file_explorer_interface(atlas),
        };

        let modified_interface_data = match self.menu_open {
            (true, Some(GuiMenuState::SettingsMenu)) => Self::display_settings_menu(page_interface_data),
            _ => page_interface_data
        };

        if let Some(rs) = self.render_state.as_mut() {
            let mut interface_guard = self.interface.lock().unwrap();
            *interface_guard = modified_interface_data;

            interface_guard.init_gpu_buffers(&rs.device, &rs.queue, rs.size, &rs.config);

            interface_guard.update_vertices_and_queue_text(rs.size, &rs.queue, &rs.device);
        } else {
            log::warn!("Attempted to rebuild interface but render_state was None. Cannot initialize GPU buffers.");
            let mut interface_guard = self.interface.lock().unwrap();
            *interface_guard = modified_interface_data;
        }
    }

    fn build_project_view_interface(atlas: UiAtlas) -> Interface {
        let mut interface = Interface::new(atlas);
        let mut header = Panel::new(Coordinate::new(0.0, 0.0), Coordinate::new(1.0, 0.02))
            .with_color("#0d1117");
        
        let element1 = Element::new(Coordinate::new(0.0, 0.0), Coordinate::new(0.025, 1.0), "solid")
            .with_color("#0d1117")
            .with_text(Alignment { vertical: VerticalAlignment::Center, horizontal: HorizontalAlignment::Center }, "File", 0.7)
            .with_fn(|| Some(GuiEvent::Highlight), InteractionStyle::OnHover);

        header.add_element(element1);

        interface.add_panel(header);
        interface
    }

    fn build_file_explorer_interface(atlas: UiAtlas) -> Interface {
        let entries = fs::read_dir(r".\projects").unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>().unwrap();

        let mut panel = Panel::new(Coordinate::new(0.2, 0.1), Coordinate::new(0.8, 0.9));
        let mut last_coordinate = Coordinate::new(0.0, 0.0);
        for file in entries {
            println!("{} {}", last_coordinate.x, last_coordinate.y);
            let element = Element::new(Coordinate::new(0.0, last_coordinate.y), Coordinate::new(1.0, last_coordinate.y + 0.2), "")
                .with_text(Alignment { vertical: VerticalAlignment::Center, horizontal: HorizontalAlignment::Center}, file.file_name().unwrap().to_str().unwrap(), 0.5);
            panel.add_element(element);
            last_coordinate.y = last_coordinate.y + 0.2
        }
        
        let mut interface = Interface::new(atlas);

        let mut header = Panel::new(Coordinate::new(0.0, 0.0), Coordinate::new(1.0, 0.02))
            .with_color("#0d1117");
        
        let element1 = Element::new(Coordinate::new(0.0, 0.0), Coordinate::new(0.025, 1.0), "solid")
            .with_color("#0d1117")
            .with_text(Alignment { vertical: VerticalAlignment::Center, horizontal: HorizontalAlignment::Center }, "Test", 0.7)
            .with_fn(|| Some(GuiEvent::ChangeLayoutToProjectView), InteractionStyle::OnClick);

        header.add_element(element1);

        interface.add_panel(header);

        interface.add_panel(panel);

        interface
    }

    fn display_settings_menu(mut interface: Interface) -> Interface {
        let element = Element::new(Coordinate::new(0.0, 0.0), Coordinate::new(1.0, 1.0), "solid")
            .with_color("#ff0000ff");
        let mut settings_panel = Panel::new(Coordinate::new(0.4, 0.4), Coordinate::new(0.6, 0.6));
        settings_panel.add_element(element);
        interface.add_panel(settings_panel);
        interface
    }
}

impl ApplicationHandler<RenderState> for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.render_state.is_none() {
            let window_attributes = Window::default_attributes().with_maximized(true);
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window_ref = Some(window.clone());
            

            let interface_arc = Arc::clone(&self.interface);

            self.render_state = Some(pollster::block_on(RenderState::new(window, interface_arc)).unwrap());

            self.rebuild_interface();

            if let Some(rs) = self.render_state.as_mut() {
                let mut interface_guard = self.interface.lock().unwrap();
                interface_guard.init_gpu_buffers(&rs.device, &rs.queue, rs.size, &rs.config);
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: RenderState) {
        self.render_state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let mut needs_layout_change: Option<GuiPageState> = None;
        let mut needs_menu_change: Option<(bool, Option<GuiMenuState>)> = None;
        let mut needs_redraw = false;

        let current_window_size = if let Some(rs) = self.render_state.as_ref() {
            rs.window.inner_size()
        } else {
            log::warn!("Window event received before render_state is initialized.");
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(rs) = self.render_state.as_mut() {
                    rs.resize(size.width, size.height);
                }
                needs_redraw = true;
            }
            WindowEvent::RedrawRequested => {
                if let Some(rs) = self.render_state.as_mut() {
                    match rs.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            rs.resize(current_window_size.width, current_window_size.height);
                        }
                        Err(e) => {
                            log::error!("Unable to render {}", e);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
                let mut needs_state_update = false;

                let mut interface_guard = self.interface.lock().unwrap();

                let current_hovered = interface_guard.handle_interaction(position, current_window_size, InteractionStyle::OnHover);

                let current_index= if let Some((_, index)) = current_hovered {
                    Some(index)
                } else {
                    None
                };

                if self.last_hovered_element_index != current_index {
                    if let Some((panel_idx, element_idx)) = self.last_hovered_element_index {
                        if panel_idx < interface_guard.panels.len() && element_idx < interface_guard.panels[panel_idx].elements.len() {
                            let element = &mut interface_guard.panels[panel_idx].elements[element_idx];
                            element.color = element.original_color.clone();
                        }
                    }

                    if let Some((_event, (panel_idx, element_idx))) = current_hovered {
                        let element = &mut interface_guard.panels[panel_idx].elements[element_idx];
                        element.with_temp_color("#999999ff");
                    }

                    self.last_hovered_element_index = current_index;
                    needs_state_update = true;
                }

                if needs_state_update {
                    if let Some(rs) = self.render_state.as_mut() {
                        interface_guard.update_vertices_and_queue_text(rs.size, &rs.queue, &rs.device);
                        needs_redraw = true;
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left && state.is_pressed() {
                    if let Some(cursor_pos) = self.cursor_position {
                        let gui_event = {
                            let mut interface_guard = self.interface.lock().unwrap();
                            interface_guard.handle_interaction(cursor_pos, current_window_size, InteractionStyle::OnClick)
                        };

                        if let Some((event, _index)) = gui_event {
                            println!("Received GUI event: {:?}", event);
                            match event {
                                GuiEvent::ChangeLayoutToFileExplorer => {
                                    if self.layout != GuiPageState::FileExplorer {
                                        needs_layout_change = Some(GuiPageState::FileExplorer);
                                    }
                                }
                                GuiEvent::ChangeLayoutToProjectView => {
                                    if self.layout != GuiPageState::ProjectView {
                                        needs_layout_change = Some(GuiPageState::ProjectView);
                                    }
                                }
                                GuiEvent::DisplaySettingsMenu => {
                                    if self.menu_open != (true, Some(GuiMenuState::SettingsMenu)) {
                                        needs_menu_change = Some((true, Some(GuiMenuState::SettingsMenu)));
                                    }
                                }
                                GuiEvent::Highlight => {

                                }
                            }
                            needs_redraw = true;
                        }
                    } else {
                        log::warn!("Mouse click detected but cursor position is None.")
                    }
                }
            }
            _ => {}
        }

        if let Some(new_layout) = needs_layout_change {
            self.render_state.as_mut().unwrap().gui_state = new_layout.clone();
            self.layout = new_layout;
            self.rebuild_interface();
            needs_redraw = true;
        }

        if let Some(menu_opened) = needs_menu_change {
            self.menu_open = menu_opened;
            self.rebuild_interface();
            needs_redraw = true;
        }

        if needs_redraw {
            if let Some(window_arc) = self.window_ref.as_ref() {
                window_arc.request_redraw();
            }
        }
    }
}