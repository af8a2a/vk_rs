use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ash::ext::debug_utils;
use ash::khr::{surface, swapchain};
use ash::{vk, Entry};
use nalgebra_glm::{look_at, perspective, Mat4x4, Vec3};
use vk_rs::camera::{Camera, Direction};
use vk_rs::structures::{QueueFamilyIndices, SurfaceStuff, UniformBufferObject, Vertex};
use vk_rs::util::buffer::{copy_buffer, create_buffer, create_index_buffer, create_vertex_buffer};
use vk_rs::util::command_buffer::{create_command_buffers, create_command_pool};
use vk_rs::util::debug::setup_debug_utils;
use vk_rs::util::descriptor::{
    create_descriptor_pool, create_descriptor_set_layout, create_descriptor_sets,
    create_uniform_buffers,
};
use vk_rs::util::device::{create_logical_device, pick_physical_device};
use vk_rs::util::fps_limiter::FPSLimiter;
use vk_rs::util::framebuffer::create_framebuffers;
use vk_rs::util::image::{
    create_image, create_image_view, create_image_views, create_texture_image,
};
use vk_rs::util::instance::create_instance;
use vk_rs::util::pipeline::{create_graphics_pipeline, create_render_pass};
use vk_rs::util::sampler::create_texture_sampler;
use vk_rs::util::surface::create_surface;
use vk_rs::util::swapchain::create_swapchain;
use vk_rs::util::sync::create_sync_objects;
use vk_rs::util::{find_depth_format, load_model};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, Position};
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey};
use winit::window::{CursorGrabMode, Window, WindowId};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
const TEXTURE_PATH: &'static str = "assets/chalet.jpg";
const MODEL_PATH: &'static str = "assets/chalet.obj";

pub struct VulkanApp {
    pub entry: Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,

    // vulkan stuff
    surface_loader: surface::Instance,
    surface: vk::SurfaceKHR,
    debug_utils_loader: debug_utils::Instance,
    debug_merssager: vk::DebugUtilsMessengerEXT,

    physical_device: vk::PhysicalDevice,
    memory_properties: vk::PhysicalDeviceMemoryProperties,

    queue_family: QueueFamilyIndices,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,

    swapchain_loader: swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_imageviews: Vec<vk::ImageView>,
    swapchain_framebuffers: Vec<vk::Framebuffer>,

    render_pass: vk::RenderPass,
    ubo_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,

    texture_image: vk::Image,
    texture_image_view: vk::ImageView,
    texture_image_memory: vk::DeviceMemory,
    texture_sampler: vk::Sampler,

    depth_image: vk::Image,
    depth_image_view: vk::ImageView,
    depth_image_memory: vk::DeviceMemory,

    vertices: Vec<Vertex>,
    indices: Vec<u32>,

    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,

    uniform_transform: UniformBufferObject,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffers_memory: Vec<vk::DeviceMemory>,

    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,

    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,

    is_framebuffer_resized: bool,

    window: Arc<Window>,
    camera: Camera,
}

impl VulkanApp {
    pub fn new(window: Arc<Window>) -> Self {
        let entry = Entry::linked();
        unsafe {
            // extension_names.push(debug_utils::NAME.as_ptr());
            let device_extensions = vec![swapchain::NAME.as_ptr()];

            let instance = create_instance(&entry, &window);

            let (debug_utils_loader, debug_merssager) = setup_debug_utils(&entry, &instance);

            let surface_stuff = create_surface(&entry, &instance, &window, 800, 600);

            let physical_device =
                pick_physical_device(&instance, &surface_stuff, &device_extensions);
            let physical_device_memory_properties =
                instance.get_physical_device_memory_properties(physical_device);

            let (device, queue_family) =
                create_logical_device(&instance, physical_device, &surface_stuff);

            let graphics_queue = device.get_device_queue(queue_family.graphics_family.unwrap(), 0);
            let present_queue = device.get_device_queue(queue_family.present_family.unwrap(), 0);

            let swapchain_stuff = create_swapchain(
                &instance,
                &device,
                physical_device,
                &window,
                &surface_stuff,
                &queue_family,
            );

            let swapchain_imageviews = create_image_views(
                &device,
                swapchain_stuff.swapchain_format,
                &swapchain_stuff.swapchain_images,
            );

            let render_pass = create_render_pass(
                &instance,
                &device,
                physical_device,
                swapchain_stuff.swapchain_format,
            );
            let ubo_layout = create_descriptor_set_layout(&device);

            let (graphics_pipeline, pipeline_layout) = create_graphics_pipeline(
                &device,
                render_pass,
                swapchain_stuff.swapchain_extent,
                ubo_layout,
            );

            let command_pool = create_command_pool(&device, &queue_family);

            let (texture_image, texture_image_memory) = create_texture_image(
                &device,
                command_pool,
                graphics_queue,
                &physical_device_memory_properties,
                &Path::new(TEXTURE_PATH),
            );
            let texture_image_view = create_image_view(
                &device,
                texture_image,
                vk::Format::R8G8B8A8_SRGB,
                vk::ImageAspectFlags::COLOR,
                1,
            );
            let texture_sampler = create_texture_sampler(&device);

            let (vertices, indices) = load_model(Path::new(MODEL_PATH));

            let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
                &device,
                &physical_device_memory_properties,
                command_pool,
                graphics_queue,
                &vertices,
            );
            let (index_buffer, index_buffer_memory) = create_index_buffer(
                &device,
                &physical_device_memory_properties,
                command_pool,
                graphics_queue,
                &indices,
            );

            let (depth_image, depth_image_view, depth_image_memory) = Self::create_depth_resources(
                &instance,
                &device,
                physical_device,
                command_pool,
                graphics_queue,
                swapchain_stuff.swapchain_extent,
                &physical_device_memory_properties,
                vk::SampleCountFlags::TYPE_1,
            );

            let swapchain_framebuffers = create_framebuffers(
                &device,
                render_pass,
                &swapchain_imageviews,
                depth_image_view,
                swapchain_stuff.swapchain_extent,
            );

            let (uniform_buffers, uniform_buffers_memory) = create_uniform_buffers(
                &device,
                &physical_device_memory_properties,
                swapchain_stuff.swapchain_images.len(),
            );
            let descriptor_pool =
                create_descriptor_pool(&device, swapchain_stuff.swapchain_images.len());
            let descriptor_sets = create_descriptor_sets(
                &device,
                descriptor_pool,
                ubo_layout,
                &uniform_buffers,
                texture_image_view,
                texture_sampler,
                swapchain_stuff.swapchain_images.len(),
            );

            let command_buffers = create_command_buffers(
                &device,
                command_pool,
                graphics_pipeline,
                &swapchain_framebuffers,
                render_pass,
                swapchain_stuff.swapchain_extent,
                vertex_buffer,
                index_buffer,
                pipeline_layout,
                &descriptor_sets,
                indices.len() as u32,
            );
            let sync_ojbects = create_sync_objects(&device, MAX_FRAMES_IN_FLIGHT);
            let camera = Camera::new(
                Vec3::new(2.0, 5.0, 2.0),
                -Vec3::new(2.0, 5.0, 2.0),
                Vec3::new(0.0, 0.0, 1.0),
                swapchain_stuff.swapchain_extent.width as f32
                    / swapchain_stuff.swapchain_extent.height as f32,
                45.0_f32.to_radians(),
            );

            let mut uniform_transform: UniformBufferObject = UniformBufferObject {
                model: camera.get_model(),
                view: camera.get_view_matrix(),
                proj: camera.get_perspective_projection_matrix(),
            };
            // *uniform_transform.proj.index_mut((1, 1)) *= -1.0;

            Self {
                entry,
                instance,
                surface: surface_stuff.surface,
                surface_loader: surface_stuff.surface_loader,
                debug_utils_loader,
                debug_merssager,

                physical_device,
                device,

                queue_family,
                graphics_queue,
                present_queue,

                swapchain_loader: swapchain_stuff.swapchain_loader,
                swapchain: swapchain_stuff.swapchain,
                swapchain_format: swapchain_stuff.swapchain_format,
                swapchain_images: swapchain_stuff.swapchain_images,
                swapchain_extent: swapchain_stuff.swapchain_extent,
                swapchain_imageviews,
                swapchain_framebuffers,

                pipeline_layout,
                render_pass,
                graphics_pipeline,

                command_pool,
                command_buffers,

                image_available_semaphores: sync_ojbects.image_available_semaphores,
                render_finished_semaphores: sync_ojbects.render_finished_semaphores,
                in_flight_fences: sync_ojbects.inflight_fences,
                current_frame: 0,

                is_framebuffer_resized: false,
                window,

                vertex_buffer,
                vertex_buffer_memory,
                index_buffer,
                index_buffer_memory,

                uniform_buffers,
                uniform_buffers_memory,

                descriptor_pool,
                descriptor_sets,
                uniform_transform,
                ubo_layout,

                texture_image,
                texture_image_memory,
                texture_image_view,
                texture_sampler,

                depth_image,
                depth_image_view,
                depth_image_memory,

                vertices,
                indices,

                memory_properties: physical_device_memory_properties,
                camera,
            }
        }
    }
}

impl VulkanApp {
    fn update_input(&mut self, input_state: &InputState, delta_time: f32) {
        if input_state.keyboard_state.contains("w") {
            self.camera.process_move(Direction::Forward, delta_time);
        }

        if input_state.keyboard_state.contains("a") {
            self.camera.process_move(Direction::Left, delta_time);
        }

        if input_state.keyboard_state.contains("s") {
            self.camera.process_move(Direction::Backward, delta_time);
        }

        if input_state.keyboard_state.contains("d") {
            self.camera.process_move(Direction::Right, delta_time);
        }
    }

    fn draw_frame(&mut self, input_state: &InputState, delta_time: f32) {
        self.update_input(input_state, delta_time);
        let wait_fences = [self.in_flight_fences[self.current_frame]];

        unsafe {
            self.device
                .wait_for_fences(&wait_fences, true, u64::MAX)
                .expect("Failed to wait for Fence!");
        }

        let (image_index, _is_sub_optimal) = unsafe {
            let result = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            );
            match result {
                Ok(image_index) => image_index,
                Err(vk_result) => match vk_result {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        self.recreate_swapchain();
                        return;
                    }
                    _ => panic!("Failed to acquire Swap Chain Image!"),
                },
            }
        };

        self.update_uniform_buffer(image_index as usize, delta_time);

        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];

        let binding = [self.command_buffers[image_index as usize]];
        let submit_infos = [vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&binding)
            .signal_semaphores(&signal_semaphores)];

        unsafe {
            self.device
                .reset_fences(&wait_fences)
                .expect("Failed to reset Fence!");

            self.device
                .queue_submit(
                    self.graphics_queue,
                    &submit_infos,
                    self.in_flight_fences[self.current_frame],
                )
                .expect("Failed to execute queue submit.");
        }

        let swapchains = [self.swapchain];

        let binding = [image_index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&binding);

        let result = unsafe {
            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
        };
        let is_resized = match result {
            Ok(_) => self.is_framebuffer_resized,
            Err(vk_result) => match vk_result {
                vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => true,
                _ => panic!("Failed to execute queue present."),
            },
        };
        if is_resized {
            self.is_framebuffer_resized = false;
            self.recreate_swapchain();
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    fn recreate_swapchain(&mut self) {
        // parameters -------------
        let surface_suff = SurfaceStuff {
            surface_loader: self.surface_loader.clone(),
            surface: self.surface,
            screen_width: 800,
            screen_height: 600,
        };
        // ------------------------

        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait device idle!")
        };
        self.cleanup_swapchain();

        let swapchain_stuff = create_swapchain(
            &self.instance,
            &self.device,
            self.physical_device,
            &self.window,
            &surface_suff,
            &self.queue_family,
        );
        self.swapchain_loader = swapchain_stuff.swapchain_loader;
        self.swapchain = swapchain_stuff.swapchain;
        self.swapchain_images = swapchain_stuff.swapchain_images;
        self.swapchain_format = swapchain_stuff.swapchain_format;
        self.swapchain_extent = swapchain_stuff.swapchain_extent;

        self.swapchain_imageviews =
            create_image_views(&self.device, self.swapchain_format, &self.swapchain_images);
        self.render_pass = create_render_pass(
            &self.instance,
            &self.device,
            self.physical_device,
            self.swapchain_format,
        );

        let (graphics_pipeline, pipeline_layout) = create_graphics_pipeline(
            &self.device,
            self.render_pass,
            swapchain_stuff.swapchain_extent,
            self.ubo_layout,
        );
        self.graphics_pipeline = graphics_pipeline;
        self.pipeline_layout = pipeline_layout;

        let depth_resources = Self::create_depth_resources(
            &self.instance,
            &self.device,
            self.physical_device,
            self.command_pool,
            self.graphics_queue,
            self.swapchain_extent,
            &self.memory_properties,
            vk::SampleCountFlags::TYPE_1,
        );
        self.depth_image = depth_resources.0;
        self.depth_image_view = depth_resources.1;
        self.depth_image_memory = depth_resources.2;

        self.swapchain_framebuffers = create_framebuffers(
            &self.device,
            self.render_pass,
            &self.swapchain_imageviews,
            self.depth_image_view,
            self.swapchain_extent,
        );

        self.command_buffers = create_command_buffers(
            &self.device,
            self.command_pool,
            self.graphics_pipeline,
            &self.swapchain_framebuffers,
            self.render_pass,
            self.swapchain_extent,
            self.vertex_buffer,
            self.index_buffer,
            self.pipeline_layout,
            &self.descriptor_sets,
            self.indices.len() as u32,
        );
    }

    fn cleanup_swapchain(&self) {
        unsafe {
            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);
            self.device.free_memory(self.depth_image_memory, None);

            self.device
                .free_command_buffers(self.command_pool, &self.command_buffers);
            for &framebuffer in self.swapchain_framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            for &image_view in self.swapchain_imageviews.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }

    fn update_uniform_buffer(&mut self, current_image: usize, delta_time: f32) {
        self.uniform_transform.view = self.camera.get_view_matrix();
        self.uniform_transform.model = self.camera.get_model();
        let ubos = [self.uniform_transform.clone()];

        let buffer_size = (std::mem::size_of::<UniformBufferObject>() * ubos.len()) as u64;

        unsafe {
            let data_ptr =
                self.device
                    .map_memory(
                        self.uniform_buffers_memory[current_image],
                        0,
                        buffer_size,
                        vk::MemoryMapFlags::empty(),
                    )
                    .expect("Failed to Map Memory") as *mut UniformBufferObject;

            data_ptr.copy_from_nonoverlapping(ubos.as_ptr(), ubos.len());

            self.device
                .unmap_memory(self.uniform_buffers_memory[current_image]);
        }
    }

    fn create_depth_resources(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        _command_pool: vk::CommandPool,
        _submit_queue: vk::Queue,
        swapchain_extent: vk::Extent2D,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        msaa_samples: vk::SampleCountFlags,
    ) -> (vk::Image, vk::ImageView, vk::DeviceMemory) {
        let depth_format = find_depth_format(instance, physical_device);
        let (depth_image, depth_image_memory) = create_image(
            device,
            swapchain_extent.width,
            swapchain_extent.height,
            1,
            msaa_samples,
            depth_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            device_memory_properties,
        );
        let depth_image_view = create_image_view(
            device,
            depth_image,
            depth_format,
            vk::ImageAspectFlags::DEPTH,
            1,
        );

        (depth_image, depth_image_view, depth_image_memory)
    }
}
impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().expect("Device lost!");
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device
                    .destroy_semaphore(self.image_available_semaphores[i], None);
                self.device
                    .destroy_semaphore(self.render_finished_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }

            self.cleanup_swapchain();

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);

            for i in 0..self.uniform_buffers.len() {
                self.device.destroy_buffer(self.uniform_buffers[i], None);
                self.device
                    .free_memory(self.uniform_buffers_memory[i], None);
            }
            self.device
                .destroy_descriptor_set_layout(self.ubo_layout, None);

            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);

            self.device.destroy_image(self.texture_image, None);
            self.device.free_memory(self.texture_image_memory, None);
            self.device.destroy_sampler(self.texture_sampler, None);
            self.device
                .destroy_image_view(self.texture_image_view, None);

            self.device.destroy_command_pool(self.command_pool, None);

            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);

            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_merssager, None);
            self.instance.destroy_instance(None);
        }
    }
}

#[derive(Default)]
struct InputState {
    left_mouse_pressed: bool,
    right_mouse_pressed: bool,
    last_mouse_pos: winit::dpi::PhysicalPosition<f64>,
    keyboard_state: HashSet<String>,
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    vk: Option<VulkanApp>,
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
        self.vk = Some(VulkanApp::new(window.clone()));
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

                    if self.state.keyboard_state.contains("w") {
                        vk.camera.process_move(Direction::Forward, delta_time);
                    }

                    if self.state.keyboard_state.contains("a") {
                        vk.camera.process_move(Direction::Left, delta_time);
                    }

                    if self.state.keyboard_state.contains("s") {
                        vk.camera.process_move(Direction::Backward, delta_time);
                    }

                    if self.state.keyboard_state.contains("d") {
                        vk.camera.process_move(Direction::Right, delta_time);
                    }

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
