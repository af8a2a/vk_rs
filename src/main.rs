use std::sync::Arc;

use vk_rs::base::VulkanBase;
use vk_rs::structures::InputState;
use vk_rs::util::fps_limiter::FPSLimiter;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};


#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    vk: Option<VulkanBase>,
    timer: Option<FPSLimiter>,

    //helper
    state: InputState,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        // window.set_cursor_visible(false);
        // window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
        window.focus_window();
        self.vk = Some(VulkanBase::new(window.clone()));
        self.window = Some(window);
        self.timer = Some(FPSLimiter::new());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // println!("extend2d:{:#?}", self.vk.as_ref().unwrap().surface_resolution);
        let timer = self.timer.as_mut().unwrap();
        let delta_time = timer.delta_time();

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if self.vk.is_some() {
                    let vk = self.vk.as_mut().unwrap();
                    vk.draw_frame(&self.state, delta_time);
                }
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                if let MouseButton::Left = button {
                    self.state.left_mouse_pressed = state.is_pressed();
                }
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                if self.state.left_mouse_pressed {
                    let camera = &mut self.vk.as_mut().unwrap().camera;
                    let (xoffset, yoffset) = (
                        (position.x - self.state.last_mouse_pos.x),
                        position.y - self.state.last_mouse_pos.y,
                    );
                    camera.process_mouse(xoffset as f32, yoffset as f32);
                }
                self.state.last_mouse_pos = position;
            }
            WindowEvent::KeyboardInput { event, .. } => match event.state {
                ElementState::Pressed => {
                    if let Key::Character(ch) = event.logical_key.as_ref() {
                        self.state.keyboard_state.insert(ch.to_lowercase());
                    }
                }
                ElementState::Released => {
                    if let Key::Character(ch) = event.logical_key.as_ref() {
                        self.state.keyboard_state.remove(ch);
                    }
                }
            },
            _ => (),
        }
        timer.tick_frame();
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();

    event_loop.run_app(&mut app).unwrap();
}
