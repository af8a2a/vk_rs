use ash::{
    util::Align,
    vk::{self, DeviceSize},
};
use std::{ffi::c_void, mem::size_of, sync::Arc};
use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
    keyboard::Key,
    window::Window,
};

use crate::{
    in_flight_frames::{InFlightFrames, SyncObjects},
    Camera, Context, Image, ImageParameters, RenderError, Texture,
};

pub const SCENE_COLOR_FORMAT: vk::Format = vk::Format::R32G32B32A32_SFLOAT;

/// Utility function that copy the content of a slice at the position of a given pointer.
pub unsafe fn mem_copy<T: Copy>(ptr: *mut c_void, data: &[T]) {
    let elem_size = size_of::<T>() as DeviceSize;
    let size = data.len() as DeviceSize * elem_size;
    let mut align = Align::new(ptr, elem_size, size);
    align.copy_from_slice(data);
}

/// Utility function that copy the content of a slice at the position of a given pointer and pad elements to respect the requested alignment.
pub unsafe fn mem_copy_aligned<T: Copy>(ptr: *mut c_void, alignment: DeviceSize, data: &[T]) {
    let size = data.len() as DeviceSize * alignment;
    let mut align = Align::new(ptr, alignment, size);
    align.copy_from_slice(data);
}

pub fn create_sampler(
    context: &Arc<Context>,
    min_filter: vk::Filter,
    mag_filter: vk::Filter,
) -> vk::Sampler {
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(mag_filter)
        .min_filter(min_filter)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .anisotropy_enable(false)
        .max_anisotropy(0.0)
        .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(1.0);

    unsafe {
        context
            .device()
            .create_sampler(&sampler_info, None)
            .expect("Failed to create sampler")
    }
}

pub fn allocate_command_buffers(context: &Context, count: usize) -> Vec<vk::CommandBuffer> {
    let allocate_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(context.general_command_pool())
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(count as _);

    unsafe {
        context
            .device()
            .allocate_command_buffers(&allocate_info)
            .unwrap()
    }
}

pub fn create_sync_objects(context: &Arc<Context>) -> InFlightFrames {
    let device = context.device();
    let mut sync_objects_vec = Vec::new();
    for _ in 0..2 {
        let image_available_semaphore = {
            let semaphore_info = vk::SemaphoreCreateInfo::default();
            unsafe { device.create_semaphore(&semaphore_info, None).unwrap() }
        };

        let render_finished_semaphore = {
            let semaphore_info = vk::SemaphoreCreateInfo::default();
            unsafe { device.create_semaphore(&semaphore_info, None).unwrap() }
        };

        let in_flight_fence = {
            let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
            unsafe { device.create_fence(&fence_info, None).unwrap() }
        };

        let sync_objects = SyncObjects {
            image_available_semaphore,
            render_finished_semaphore,
            fence: in_flight_fence,
        };
        sync_objects_vec.push(sync_objects)
    }

    InFlightFrames::new(Arc::clone(context), sync_objects_vec)
}

pub fn find_depth_format(context: &Context) -> vk::Format {
    let candidates = vec![
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];
    context
        .find_supported_format(
            &candidates,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
        .expect("Failed to find a supported depth format")
}

pub fn create_scene_color(
    context: &Arc<Context>,
    extent: vk::Extent2D,
    msaa_samples: vk::SampleCountFlags,
) -> Texture {
    let image_usage = match msaa_samples {
        vk::SampleCountFlags::TYPE_1 => {
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED
        }
        _ => vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
    };
    let image = Image::create(
        Arc::clone(context),
        ImageParameters {
            mem_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            extent,
            sample_count: msaa_samples,
            format: SCENE_COLOR_FORMAT,
            usage: image_usage,
            ..Default::default()
        },
    );

    image.transition_image_layout(
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    );

    let view = image.create_view(vk::ImageViewType::TYPE_2D, vk::ImageAspectFlags::COLOR);

    let sampler = match msaa_samples {
        vk::SampleCountFlags::TYPE_1 => Some(create_sampler(
            context,
            vk::Filter::NEAREST,
            vk::Filter::NEAREST,
        )),
        _ => None,
    };

    Texture::new(Arc::clone(context), image, view, sampler)
}

pub fn create_scene_depth(
    context: &Arc<Context>,
    format: vk::Format,
    extent: vk::Extent2D,
    msaa_samples: vk::SampleCountFlags,
) -> Texture {
    let image_usage = match msaa_samples {
        vk::SampleCountFlags::TYPE_1 => vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        _ => {
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT
        }
    };
    let image = Image::create(
        Arc::clone(context),
        ImageParameters {
            mem_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            extent,
            sample_count: msaa_samples,
            format,
            usage: image_usage,
            ..Default::default()
        },
    );

    image.transition_image_layout(
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    );

    let view = image.create_view(vk::ImageViewType::TYPE_2D, vk::ImageAspectFlags::DEPTH);

    let sampler = match msaa_samples {
        vk::SampleCountFlags::TYPE_1 => Some(create_sampler(
            context,
            vk::Filter::NEAREST,
            vk::Filter::NEAREST,
        )),
        _ => None,
    };

    Texture::new(Arc::clone(context), image, view, sampler)
}

pub trait WindowApp {
    fn new_frame(&mut self);
    fn end_frame(&mut self, window: &Window);
    fn handle_window_event(&mut self, _window: &Window, event: &WindowEvent);
    fn handle_device_event(&mut self, event: &DeviceEvent);
    fn recreate_swapchain(&mut self, dimensions: [u32; 2], vsync: bool, hdr: bool);
    fn on_exit(&mut self) {}
    fn render(&mut self, window: &Window, camera: Camera) -> Result<(), RenderError>;
    fn cmd_draw(&mut self, command_buffer: vk::CommandBuffer, frame_index: usize);
}
