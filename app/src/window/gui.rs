use std::sync::{Arc, Mutex};

use gfx::{definitions::{GuiEvent, GuiState}, gui::interface::{Color, Coordinate, Element, Interface, Panel}, RenderState};
use winit::{application::ApplicationHandler, dpi::PhysicalPosition, event::{MouseButton, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, window::Window};

pub struct EditorApp {
    layout: GuiState,
    interface: Arc<Mutex<Interface>>,
    render_state: Option<gfx::RenderState>,
    cursor_position: Option<PhysicalPosition<f64>>,
    window_ref: Option<Arc<Window>>,
}

impl EditorApp {
    pub fn new() -> anyhow::Result<()> {
        let mut app = EditorApp {
            layout: GuiState::ProjectView,
            interface: Arc::new(Mutex::new(Interface::new())),
            render_state: None,
            cursor_position: None,
            window_ref: None,
        };

        env_logger::init();

        let event_loop = EventLoop::with_user_event().build()?;

        event_loop.run_app(&mut app)?;

        Ok(())
    }

    fn rebuild_interface(&mut self) {
        println!("Rebuilding interface for layout: {:?}", self.layout);

        let new_interface_data = match self.layout {
            GuiState::ProjectView => Self::build_project_view_interface(),
            GuiState::FileExplorer => Self::build_file_explorer_interface(),
        };

        if let Some(rs) = self.render_state.as_mut() {
            let mut interface_guard = self.interface.lock().unwrap();
            *interface_guard = new_interface_data;

            interface_guard.init_gpu_buffers(&rs.device, &rs.queue, rs.size, &rs.config);

            interface_guard.update_vertices_and_queue_text(rs.size, &rs.queue, &rs.device);
        } else {
            log::warn!("Attempted to rebuild interface but render_state was None. Cannot initialize GPU buffers.");
            let mut interface_guard = self.interface.lock().unwrap();
            *interface_guard = new_interface_data;
        }
    }

    fn build_project_view_interface() -> Interface {
        let mut interface = Interface::new();
        let mut panel = Panel::new(Coordinate::new(0.0, 0.0), Coordinate::new(0.03, 1.0));
        
        let element1 = Element::new(Coordinate::new(0.0, 0.0), Coordinate::new(1.0, 0.05), Color::new(0.0, 0.0, 1.0))
            .with_fn(|| Some(GuiEvent::ChangeLayoutToFileExplorer));

        panel.add_element(element1);

        interface.add_panel(panel);
        interface
    }

    fn build_file_explorer_interface() -> Interface {
        let mut interface = Interface::new();
        let mut panel = Panel::new(Coordinate::new(0.4, 0.4), Coordinate::new(0.6, 0.6));
        let element = Element::new(Coordinate::new(0.0, 0.0), Coordinate::new(1.0, 1.0), Color::new(1.0, 0.0, 0.0));

        panel.add_element(element);
        interface.add_panel(panel);

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
        let mut needs_layout_change: Option<GuiState> = None;
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
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left && state.is_pressed() {
                    if let Some(cursor_pos) = self.cursor_position {
                        let gui_event = {
                            let mut interface_guard = self.interface.lock().unwrap();
                            interface_guard.handle_interaction(cursor_pos, current_window_size)
                        };

                        if let Some(event) = gui_event {
                            println!("Received GUI event: {:?}", event);
                            match event {
                                GuiEvent::ChangeLayoutToFileExplorer => {
                                    if self.layout != GuiState::FileExplorer {
                                        needs_layout_change = Some(GuiState::FileExplorer);
                                    }
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

        if needs_redraw {
            if let Some(window_arc) = self.window_ref.as_ref() {
                window_arc.request_redraw();
            }
        }
    }
}