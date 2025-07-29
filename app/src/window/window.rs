use std::sync::Arc;

use gfx::{gui::interface::Interface, RenderState};

use winit::{
    application::ApplicationHandler, dpi::PhysicalPosition, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::PhysicalKey, window::Window
};

pub struct App {
    state: Option<RenderState>,
    interface: Option<Interface>,
    cursor_position: Option<PhysicalPosition<f64>>
}

impl ApplicationHandler<RenderState> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {

        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes()
            .with_maximized(true);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        {
            // If we are not on web we can use pollster to
            // await the
            self.state = Some(pollster::block_on(RenderState::new(window, self.interface.take().expect("Interface should be some by now!"))).unwrap());
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: RenderState) {
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
            }
            WindowEvent::MouseInput { state, button, .. } => match (button, state.is_pressed()) {
                (MouseButton::Left, true) => {
                    self.state.as_mut().unwrap().handle_interact(self.cursor_position.unwrap());
                }
                (MouseButton::Left, false) => {}
                _ => {}
            },
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}

pub fn run(interface: Interface) -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App { 
        state: None, 
        interface: Some(interface), 
        cursor_position: None,
    };
    event_loop.run_app(&mut app)?;

    Ok(())
}