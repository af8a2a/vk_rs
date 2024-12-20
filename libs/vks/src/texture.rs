use super::{buffer::*, context::*, image::*, util::*};
use ash::vk;
use std::{mem::size_of_val, sync::Arc};

pub struct Texture {
    context: Arc<Context>,
    pub image: Image,
    pub view: vk::ImageView,
    pub sampler: Option<vk::Sampler>,
}

#[derive(Copy, Clone, Debug)]
pub struct SamplerParameters {
    pub mag_filter: vk::Filter,
    pub min_filter: vk::Filter,
    pub anisotropy_enabled: bool,
    pub max_anisotropy: f32,
}

impl Default for SamplerParameters {
    fn default() -> Self {
        Self {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            anisotropy_enabled: false,
            max_anisotropy: 0.0,
        }
    }
}

impl Texture {
    pub fn new(
        context: Arc<Context>,
        image: Image,
        view: vk::ImageView,
        sampler: Option<vk::Sampler>,
    ) -> Self {
        Texture {
            context,
            image,
            view,
            sampler,
        }
    }

    pub fn from_rgba(
        context: &Arc<Context>,
        width: u32,
        height: u32,
        data: &[u8],
        linear: bool,
    ) -> Self {
        let (texture, _) = context.execute_one_time_commands(|command_buffer| {
            Self::cmd_from_rgba(context, command_buffer, width, height, data, linear)
        });
        texture
    }

    pub fn cmd_from_rgba(
        context: &Arc<Context>,
        command_buffer: vk::CommandBuffer,
        width: u32,
        height: u32,
        data: &[u8],
        linear: bool,
    ) -> (Self, Buffer) {
        let max_mip_levels = ((width.min(height) as f32).log2().floor() + 1.0) as u32;
        let extent = vk::Extent2D { width, height };
        let image_size = size_of_val(data) as vk::DeviceSize;
        let device = context.device();

        let mut buffer = Buffer::create(
            Arc::clone(context),
            image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        unsafe {
            let ptr = buffer.map_memory();
            mem_copy(ptr, data);
        }

        let format = if linear {
            vk::Format::R8G8B8A8_UNORM
        } else {
            vk::Format::R8G8B8A8_SRGB
        };

        let image = Image::create(
            Arc::clone(context),
            ImageParameters {
                mem_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
                extent,
                format,
                mip_levels: max_mip_levels,
                usage: vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED,
                ..Default::default()
            },
        );

        // Transition the image layout and copy the buffer into the image
        // and transition the layout again to be readable from fragment shader.
        {
            image.cmd_transition_image_layout(
                command_buffer,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            image.cmd_copy_buffer(command_buffer, &buffer, extent);

            image.cmd_generate_mipmaps(command_buffer, extent);
        }

        let image_view = image.create_view(vk::ImageViewType::TYPE_2D, vk::ImageAspectFlags::COLOR);

        let sampler = {
            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT)
                .anisotropy_enable(true)
                .max_anisotropy(16.0)
                .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                .unnormalized_coordinates(false)
                .compare_enable(false)
                .compare_op(vk::CompareOp::ALWAYS)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .mip_lod_bias(0.0)
                .min_lod(0.0)
                .max_lod(max_mip_levels as _);

            unsafe {
                device
                    .create_sampler(&sampler_info, None)
                    .expect("Failed to create sampler")
            }
        };

        let texture = Texture::new(Arc::clone(context), image, image_view, Some(sampler));

        (texture, buffer)
    }

    pub fn from_rgba_32(
        context: &Arc<Context>,
        width: u32,
        height: u32,
        with_mipmaps: bool,
        data: &[f32],
        sampler_parameters: Option<SamplerParameters>,
    ) -> Self {
        let max_mip_levels = if with_mipmaps {
            ((width.min(height) as f32).log2().floor() + 1.0) as u32
        } else {
            1
        };
        let extent = vk::Extent2D { width, height };
        let image_size = size_of_val(data) as vk::DeviceSize;
        let device = context.device();

        let mut buffer = Buffer::create(
            Arc::clone(context),
            image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        unsafe {
            let ptr = buffer.map_memory();
            mem_copy(ptr, data);
        }

        let usage = if with_mipmaps {
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED
        } else {
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED
        };

        let image = Image::create(
            Arc::clone(context),
            ImageParameters {
                mem_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
                extent,
                format: vk::Format::R32G32B32A32_SFLOAT,
                mip_levels: max_mip_levels,
                usage,
                ..Default::default()
            },
        );

        // Transition the image layout and copy the buffer into the image
        // and transition the layout again to be readable from fragment shader.
        {
            image.transition_image_layout(
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            image.copy_buffer(&buffer, extent);

            if with_mipmaps {
                image.generate_mipmaps(extent);
            } else {
                image.transition_image_layout(
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                );
            }
        }

        let image_view = image.create_view(vk::ImageViewType::TYPE_2D, vk::ImageAspectFlags::COLOR);

        let sampler = {
            let params = sampler_parameters.unwrap_or_default();
            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(params.mag_filter)
                .min_filter(params.min_filter)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT)
                .anisotropy_enable(params.anisotropy_enabled)
                .max_anisotropy(params.max_anisotropy)
                .border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK)
                .unnormalized_coordinates(false)
                .compare_enable(false)
                .compare_op(vk::CompareOp::ALWAYS)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .mip_lod_bias(0.0)
                .min_lod(0.0)
                .max_lod(max_mip_levels as _);

            unsafe {
                device
                    .create_sampler(&sampler_info, None)
                    .expect("Failed to create sampler")
            }
        };

        Texture::new(Arc::clone(context), image, image_view, Some(sampler))
    }

    pub fn create_renderable_cubemap(
        context: &Arc<Context>,
        size: u32,
        mip_levels: u32,
        format: vk::Format,
    ) -> Self {
        let extent = vk::Extent2D {
            width: size,
            height: size,
        };

        let device = context.device();

        let image = Image::create(
            Arc::clone(context),
            ImageParameters {
                mem_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
                extent,
                format,
                layers: 6,
                mip_levels,
                usage: vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::COLOR_ATTACHMENT,
                create_flags: vk::ImageCreateFlags::CUBE_COMPATIBLE,
                ..Default::default()
            },
        );

        image.transition_image_layout(
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        let image_view = image.create_view(vk::ImageViewType::CUBE, vk::ImageAspectFlags::COLOR);

        let sampler = {
            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
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
                .max_lod(mip_levels as _);

            unsafe {
                device
                    .create_sampler(&sampler_info, None)
                    .expect("Failed to create sampler")
            }
        };

        Texture::new(Arc::clone(context), image, image_view, Some(sampler))
    }

    pub fn create_renderable_texture(
        context: &Arc<Context>,
        width: u32,
        height: u32,
        format: vk::Format,
    ) -> Self {
        let extent = vk::Extent2D { width, height };

        let device = context.device();

        let image = Image::create(
            Arc::clone(context),
            ImageParameters {
                mem_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
                extent,
                format,
                usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::COLOR_ATTACHMENT,
                ..Default::default()
            },
        );

        image.transition_image_layout(
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        let image_view = image.create_view(vk::ImageViewType::TYPE_2D, vk::ImageAspectFlags::COLOR);

        let sampler = {
            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
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
                device
                    .create_sampler(&sampler_info, None)
                    .expect("Failed to create sampler")
            }
        };

        Texture::new(Arc::clone(context), image, image_view, Some(sampler))
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            if let Some(sampler) = self.sampler.take() {
                self.context.device().destroy_sampler(sampler, None);
            }
            self.context.device().destroy_image_view(self.view, None);
        }
    }
}
