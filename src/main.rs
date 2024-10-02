use std::sync::Arc;

use ash::ext::debug_utils;
use ash::khr::{surface, swapchain};
use ash::{vk, Entry};
use vk_rs::structures::{QueueFamilyIndices, SurfaceStuff, Vertex, VERTICES_DATA};
use vk_rs::util::buffer::{copy_buffer, create_buffer};
use vk_rs::util::command_buffer::{create_command_buffers, create_command_pool};
use vk_rs::util::debug::setup_debug_utils;
use vk_rs::util::device::{create_logical_device, pick_physical_device};
use vk_rs::util::framebuffer::create_framebuffers;
use vk_rs::util::image::create_image_views;
use vk_rs::util::instance::create_instance;
use vk_rs::util::pipeline::{create_graphics_pipeline, create_render_pass};
use vk_rs::util::surface::create_surface;
use vk_rs::util::swapchain::create_swapchain;
use vk_rs::util::sync::create_sync_objects;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

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
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,

    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,

    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,

    is_framebuffer_resized: bool,

    window: Arc<Window>,
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

            let render_pass = create_render_pass(&device, swapchain_stuff.swapchain_format);

            let (graphics_pipeline, pipeline_layout) =
                create_graphics_pipeline(&device, render_pass, swapchain_stuff.swapchain_extent);

            let swapchain_framebuffers = create_framebuffers(
                &device,
                render_pass,
                &swapchain_imageviews,
                swapchain_stuff.swapchain_extent,
            );
            let command_pool = create_command_pool(&device, &queue_family);
            let (vertex_buffer, vertex_buffer_memory) = VulkanApp::create_vertex_buffer(
                &instance,
                &device,
                physical_device,
                command_pool,
                graphics_queue,
            );
            let command_buffers = create_command_buffers(
                &device,
                command_pool,
                graphics_pipeline,
                &swapchain_framebuffers,
                render_pass,
                swapchain_stuff.swapchain_extent,
                vertex_buffer,
            );
            let sync_ojbects = create_sync_objects(&device, MAX_FRAMES_IN_FLIGHT);

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
            }
        }
    }
}

impl VulkanApp {
    fn draw_frame(&mut self) {
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

        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];

        let submit_infos = [vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&self.command_buffers)
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
        self.render_pass = create_render_pass(&self.device, self.swapchain_format);
        let (graphics_pipeline, pipeline_layout) = create_graphics_pipeline(
            &self.device,
            self.render_pass,
            swapchain_stuff.swapchain_extent,
        );
        self.graphics_pipeline = graphics_pipeline;
        self.pipeline_layout = pipeline_layout;

        self.swapchain_framebuffers = create_framebuffers(
            &self.device,
            self.render_pass,
            &self.swapchain_imageviews,
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
        );
    }

    fn cleanup_swapchain(&self) {
        unsafe {
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

    fn create_vertex_buffer(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let buffer_size = std::mem::size_of_val(&VERTICES_DATA) as vk::DeviceSize;
        let device_memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        let (staging_buffer, staging_buffer_memory) = create_buffer(
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &device_memory_properties,
        );

        unsafe {
            let data_ptr = device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Failed to Map Memory") as *mut Vertex;

            data_ptr.copy_from_nonoverlapping(VERTICES_DATA.as_ptr(), VERTICES_DATA.len());

            device.unmap_memory(staging_buffer_memory);
        }

        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &device_memory_properties,
        );

        copy_buffer(
            device,
            submit_queue,
            command_pool,
            staging_buffer,
            vertex_buffer,
            buffer_size,
        );

        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_buffer_memory, None);
        }

        (vertex_buffer, vertex_buffer_memory)
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
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);

            self.cleanup_swapchain();

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
struct App {
    window: Option<Arc<Window>>,
    vk: Option<VulkanApp>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.vk = Some(VulkanApp::new(window.clone()));
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // println!("extend2d:{:#?}", self.vk.as_ref().unwrap().surface_resolution);

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if self.vk.is_some() {
                    self.vk.as_mut().unwrap().draw_frame();
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

    let mut app = App::default();

    event_loop.run_app(&mut app).unwrap();
}
