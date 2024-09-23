use std::ffi::{self, c_char};

use ash::ext::debug_utils;
use ash::{vk, Entry};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::{Window, WindowId};

pub struct VulkanApp {
    pub entry: Entry,
    pub instance: ash::Instance,
}

impl VulkanApp {
    pub fn new(window: &Window) -> Self {
        let entry = Entry::linked();
        unsafe {

            let app_name = ffi::CStr::from_bytes_with_nul_unchecked(b"VulkanTriangle\0");
            let layer_names = [ffi::CStr::from_bytes_with_nul_unchecked(
                b"VK_LAYER_KHRONOS_validation\0",
            )];
            let layers_names_raw: Vec<*const c_char> = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();

            let mut extension_names = ash_window::enumerate_required_extensions(
                window
                    .display_handle()
                    .expect("Failed to get display handle")
                    .as_raw(),
            )
            .expect("Failed to enumerate extensions")
            .to_vec();
            extension_names.push(debug_utils::NAME.as_ptr());

            let appinfo = vk::ApplicationInfo::default()
                .application_name(app_name)
                .application_version(0)
                .engine_name(app_name)
                .engine_version(0)
                .api_version(vk::make_api_version(0, 1, 0, 0));

            let instance_create_info = vk::InstanceCreateInfo::default()
                .application_info(&appinfo)
                .enabled_layer_names(&layers_names_raw)
                .enabled_extension_names(&extension_names)
                .flags(vk::InstanceCreateFlags::default());

            let instance = entry
                .create_instance(&instance_create_info, None)
                .expect("Instance creation error");

            Self { entry, instance }
        }
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}


#[derive(Default)]
struct App {
    window: Option<Window>,
    vk: Option<VulkanApp>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.vk = Some(VulkanApp::new(self.window.as_ref().unwrap()));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                if self.vk.is_some() {
                    println!("create vulkan instance!");
                }
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);

    // ControlFlow::Wait pauses the event loop if no events are available to process.
    // This is ideal for non-game applications that only update in response to user
    // input, and uses significantly less power/CPU time than ControlFlow::Poll.
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();

    event_loop.run_app(&mut app).unwrap();
}
