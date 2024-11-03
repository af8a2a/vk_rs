use std::sync::Arc;

use ash::{vk::{self, RenderingAttachmentInfo, RenderingInfo}, Device};
use egui::TextureId;
use winit::window::Window;

use crate::{
    allocate_command_buffers, cmd_transition_images_layouts, create_sampler, create_scene_color,
    create_scene_depth, create_sync_objects, find_depth_format, in_flight_frames::InFlightFrames,
    Camera, Context, Image, ImageParameters, LayoutTransition, MipsRange, Swapchain,
    SwapchainSupportDetails, Texture, HDR_SURFACE_FORMAT,
};

pub enum RenderError {
    DirtySwapchain,
}

pub struct VulkanExampleBase {
    pub context: Arc<Context>,
    pub swapchain: Swapchain,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub in_flight_frames: InFlightFrames,
    pub depth_format: vk::Format,
    pub msaa_samples: vk::SampleCountFlags,
    pub scene_color: Texture,
    pub scene_depth: Texture,
}

impl VulkanExampleBase {
    pub fn new(window: &Window,enable_debug: bool) -> Self {
        let context = Arc::new(Context::new(window, enable_debug));
        let swapchain_support_details = SwapchainSupportDetails::new(
            context.physical_device(),
            context.surface(),
            context.surface_khr(),
        );
        // let resolution = [800, 600];
        let depth_format = find_depth_format(&context);
        let msaa_samples = vk::SampleCountFlags::TYPE_4;
        window.inner_size();
        let swapchain = Swapchain::create(
            Arc::clone(&context),
            swapchain_support_details,
            window.inner_size().into(),
            Some(vk::SurfaceFormatKHR {
                format: vk::Format::R16G16B16A16_SFLOAT,
                color_space: vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT,
            }),
            true,
        );

        let command_buffers = allocate_command_buffers(&context, swapchain.image_count());

        let in_flight_frames = create_sync_objects(&context);
        let scene_color = create_scene_color(&context, swapchain.properties().extent, msaa_samples);
        let scene_depth = create_scene_depth(
            &context,
            depth_format,
            swapchain.properties().extent,
            msaa_samples,
        );

        Self {
            context,
            swapchain,
            command_buffers,
            in_flight_frames,
            depth_format,
            msaa_samples,
            scene_color,
            scene_depth,
        }
    }
    pub fn destroy_swapchain(&mut self) {
        unsafe {
            self.context
                .device()
                .free_command_buffers(self.context.general_command_pool(), &self.command_buffers);
        }
        self.swapchain.destroy();
    }
    pub fn on_new_swapchain(&mut self) {
        let swapchain_properties = self.swapchain.properties();
        self.scene_color = create_scene_color(
            &self.context,
            swapchain_properties.extent,
            self.msaa_samples,
        );
        self.scene_depth = create_scene_depth(
            &self.context,
            self.depth_format,
            swapchain_properties.extent,
            self.msaa_samples,
        );
    }

    pub fn wait_idle_gpu(&self) {
        unsafe { self.context.device().device_wait_idle().unwrap() };
    }

    pub fn recreate_swapchain(&mut self, dimensions: [u32; 2], vsync: bool, hdr: bool) {
        tracing::debug!("Recreating swapchain.");
        tracing::debug!("extent: {:?}", dimensions);

        self.wait_idle_gpu();

        self.destroy_swapchain();

        let swapchain_support_details = SwapchainSupportDetails::new(
            self.context.physical_device(),
            self.context.surface(),
            self.context.surface_khr(),
        );

        self.swapchain = Swapchain::create(
            Arc::clone(&self.context),
            swapchain_support_details,
            dimensions,
            hdr.then_some(HDR_SURFACE_FORMAT),
            vsync,
        );

        self.on_new_swapchain();

        self.command_buffers =
            allocate_command_buffers(&self.context, self.swapchain.image_count());
    }

}
