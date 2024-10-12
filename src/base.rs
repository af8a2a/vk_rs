use std::sync::Arc;

use crate::structures::texture::Texture;
use crate::structures::{InputState, RenderResource, RenderState};
use crate::structures::{QueueFamilyIndices, SurfaceStuff};
use crate::util::command_buffer::create_command_pool;
use crate::util::debug::setup_debug_utils;
use crate::util::descriptor::create_descriptor_pool;
use crate::util::device::{create_logical_device, pick_physical_device};
use crate::util::find_depth_format;
use crate::util::image::{create_image, create_image_view, create_image_views};
use crate::util::instance::create_instance;
use crate::util::sampler::create_texture_sampler;
use crate::util::surface::create_surface;
use crate::util::swapchain::create_swapchain;
use crate::util::sync::create_sync_objects;
use ash::ext::debug_utils;
use ash::khr::{surface, swapchain};
use ash::{vk, Entry};
use winit::window::Window;
pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct VulkanBase {
    pub entry: Entry,
    pub instance: ash::Instance,
    pub device: Arc<ash::Device>,

    // vulkan stuff
    pub surface_loader: surface::Instance,
    pub surface: vk::SurfaceKHR,
    pub debug_utils_loader: debug_utils::Instance,
    pub debug_merssager: vk::DebugUtilsMessengerEXT,

    pub physical_device: vk::PhysicalDevice,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,

    pub queue_family: QueueFamilyIndices,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,

    pub swapchain_loader: swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain_imageviews: Vec<vk::ImageView>,

    pub texture_sampler: vk::Sampler,

    // pub depth_image: vk::Image,
    pub depth_image_view: vk::ImageView,
    // pub depth_image_memory: vk::DeviceMemory,
    pub depth_texture: Texture,

    pub command_pool: vk::CommandPool,
    pub descriptor_pool: vk::DescriptorPool,

    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub current_frame: usize,

    pub is_framebuffer_resized: bool,

    pub window: Arc<Window>,
}

impl VulkanBase {
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

            let command_pool = create_command_pool(&device, &queue_family);
            let texture_sampler = create_texture_sampler(&device);

            let depth_texture = Self::create_depth_resources(
                &instance,
                &device,
                physical_device,
                command_pool,
                graphics_queue,
                swapchain_stuff.swapchain_extent,
                &physical_device_memory_properties,
                vk::SampleCountFlags::TYPE_1,
            );
            let depth_image_view = depth_texture.create_dsv();

            let descriptor_pool =
                create_descriptor_pool(&device, swapchain_stuff.swapchain_images.len());

            let sync_ojbects = create_sync_objects(&device, MAX_FRAMES_IN_FLIGHT);
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
                command_pool,

                image_available_semaphores: sync_ojbects.image_available_semaphores,
                render_finished_semaphores: sync_ojbects.render_finished_semaphores,
                in_flight_fences: sync_ojbects.inflight_fences,
                current_frame: 0,

                is_framebuffer_resized: false,
                window,

                descriptor_pool,
                texture_sampler,

                // depth_image,
                depth_image_view,
                // depth_image_memory,
                memory_properties: physical_device_memory_properties,
                depth_texture,
            }
        }
    }
}

impl VulkanBase {
    pub fn draw_frame<T: RenderResource + RenderState>(
        &mut self,
        input_state: &InputState,
        resource: &mut T,
        delta_time: f32,
    ) {
        resource.update_input(input_state, delta_time);
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

        resource.update_uniform_buffer(image_index as usize, delta_time);

        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];

        unsafe {
            self.device
                .wait_for_fences(&self.in_flight_fences, true, u64::MAX)
                .expect("Wait for fence failed.");
        }

        resource.record_command_buffer(self.swapchain_extent);

        let binding = [*resource.fetch_command_buffer(image_index as usize)];
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
            resource.recreate(&self);
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

        self.depth_texture = Self::create_depth_resources(
            &self.instance,
            &self.device,
            self.physical_device,
            self.command_pool,
            self.graphics_queue,
            self.swapchain_extent,
            &self.memory_properties,
            vk::SampleCountFlags::TYPE_1,
        );
        // self.depth_image = depth_resources.0;
        self.depth_image_view = self.depth_texture.create_dsv();
        // self.depth_image_memory = depth_resources.2;
    }

    fn cleanup_swapchain(&self) {
        unsafe {
            self.device.destroy_image_view(self.depth_image_view, None);
            self.depth_texture.cleanup();
            // self.device.destroy_image(self.depth_image, None);
            // self.device.free_memory(self.depth_image_memory, None);

            for &image_view in self.swapchain_imageviews.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }

    fn create_depth_resources(
        instance: &ash::Instance,
        device: &Arc<ash::Device>,
        physical_device: vk::PhysicalDevice,
        _command_pool: vk::CommandPool,
        _submit_queue: vk::Queue,
        swapchain_extent: vk::Extent2D,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        msaa_samples: vk::SampleCountFlags,
    ) -> Texture {
        let depth_format = find_depth_format(instance, physical_device);
        let (depth_image, depth_image_memory, info) = create_image(
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
        let texture = Texture::new(
            device.clone(),
            depth_image,
            depth_image_memory,
            "Depth",
            info,
        );
        texture
    }
}
impl Drop for VulkanBase {
    fn drop(&mut self) {
        unsafe {
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

            self.device.destroy_sampler(self.texture_sampler, None);

            self.device.destroy_command_pool(self.command_pool, None);

            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);

            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_merssager, None);
            self.instance.destroy_instance(None);
        }
    }
}
