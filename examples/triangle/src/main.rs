
use std::{
    error::Error,
    sync::Arc,
    time::Instant,
};

use config::Config;
use renderer::{RenderError, Renderer};
use renderer_settings::RendererSettings;
use vks::{
    Camera,
    Context,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::Key,
    window::{Fullscreen, Window, WindowId},
};

struct App {
    config: Config,
    window: Option<Window>,
    triangle_app: Option<TriangleApp>,
}
impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        let config = Config::default();
        Ok(Self {
            config,
            window: None,
            triangle_app: None,
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Triangle")
                    .with_inner_size(PhysicalSize::new(
                        self.config.resolution.width(),
                        self.config.resolution.height(),
                    )),
            )
            .expect("Failed to create window");

        self.triangle_app = Some(TriangleApp::new(self.config.clone(), &window, true));
        self.window = Some(window);
    }

    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {
        if let Some(app) = self.triangle_app.as_mut() {
            app.new_frame();
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        self.triangle_app
            .as_mut()
            .unwrap()
            .end_frame(self.window.as_ref().unwrap());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
        }

        self.triangle_app
            .as_mut()
            .unwrap()
            .handle_window_event(self.window.as_ref().unwrap(), &event);
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        self.triangle_app
            .as_mut()
            .unwrap()
            .handle_device_event(&event);
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        self.triangle_app.as_mut().unwrap().on_exit();
    }
}

pub struct TriangleApp {
    config: Config,
    context: Arc<Context>,
    renderer: Renderer,
    renderer_settings: RendererSettings,
    camera: Camera,
    time: Instant,
    dirty_swapchain: bool,
}
impl TriangleApp {
    fn new(config: Config, window: &Window, enable_debug: bool) -> Self {
        let context = Arc::new(Context::new(window, enable_debug));
        let renderer_settings = RendererSettings {};
        let renderer = Renderer::create(Arc::clone(&context), &config, renderer_settings.clone());

        Self {
            context,
            renderer,
            camera: Camera::default(),
            time: Instant::now(),
            dirty_swapchain: false,
            renderer_settings,
            config,
        }
    }
    pub fn new_frame(&mut self) {}

    pub fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) {
        match event {
            // Dropped file
            WindowEvent::DroppedFile(path) => {
                // log::debug!("File dropped: {:?}", path);
                // self.loader.load(path.clone());
            }
            // Resizing
            WindowEvent::Resized(_) => {
                self.dirty_swapchain = true;
            }
            // Key events
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Character(c),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if c == "h" {
                    // self.enable_ui = !self.enable_ui;
                }
            }
            _ => (),
        }
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        // self.input_state = self.input_state.handle_device_event(event);
    }

    pub fn end_frame(&mut self, window: &Window) {
        let new_time = Instant::now();
        let delta_s = (new_time - self.time).as_secs_f32();
        self.time = new_time;

        // If swapchain must be recreated wait for windows to not be minimized anymore
        if self.dirty_swapchain {
            let PhysicalSize { width, height } = window.inner_size();
            if width > 0 && height > 0 {
                self.renderer
                    .recreate_swapchain(window.inner_size().into(), false, false);
            } else {
                return;
            }
        }
        self.dirty_swapchain = matches!(
            self.renderer.render(window, self.camera),
            Err(RenderError::DirtySwapchain)
        );

    }

    pub fn on_exit(&mut self) {
        self.renderer.wait_idle_gpu();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new()?;
    event_loop.run_app(&mut app)?;
    println!("hello world");
    Ok(())
}
