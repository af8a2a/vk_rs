use std::{path::Path, sync::Arc};

use ash::vk;

use crate::util::{
    buffer::create_buffer,
    command_buffer::record_single_time_submit_commandbuffer,
    image::{create_image, create_image_view},
};

pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_levels: u32,
    pub array_layers: u32,
    pub format: vk::Format,
    pub bit_count: u32,
}

pub struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub name: String,
    pub device: Arc<ash::Device>,
    pub info: ImageInfo,
}

impl Texture {
    pub fn cleanup(&self) {
        unsafe {
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }

    pub fn new(
        device: Arc<ash::Device>,
        image: vk::Image,
        memory: vk::DeviceMemory,
        name: &str,
        info: ImageInfo,
    ) -> Self {
        Self {
            image,
            memory,
            name: name.to_string(),
            device,
            info,
        }
    }
    pub fn load_from_path(
        device: &Arc<ash::Device>,
        command_pool: vk::CommandPool,
        submit_queue: vk::Queue,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        image_path: &Path,
        name: &str,
    ) -> Self {
        let mut image_object = image::open(image_path).unwrap(); // this function is slow in debug mode.
        image_object = image_object.flipv();
        let (image_width, image_height) = (image_object.width(), image_object.height());
        let image_size =
            (std::mem::size_of::<u8>() as u32 * image_width * image_height * 4) as vk::DeviceSize;
        let image_data = image_object.to_rgba8();

        if image_size <= 0 {
            panic!("Failed to load texture image!")
        }

        let (staging_buffer, staging_buffer_memory) = create_buffer(
            device,
            image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            device_memory_properties,
        );

        unsafe {
            let data_ptr = device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    image_size,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Failed to Map Memory") as *mut u8;

            data_ptr.copy_from_nonoverlapping(image_data.as_ptr(), image_data.len());

            device.unmap_memory(staging_buffer_memory);
        }

        let (texture_image, texture_image_memory, info) = create_image(
            device,
            image_width,
            image_height,
            1,
            vk::SampleCountFlags::TYPE_1,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            device_memory_properties,
        );

        transition_image_layout(
            device,
            command_pool,
            submit_queue,
            texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            1,
        );

        copy_buffer_to_image(
            device,
            command_pool,
            submit_queue,
            staging_buffer,
            texture_image,
            image_width,
            image_height,
        );

        transition_image_layout(
            device,
            command_pool,
            submit_queue,
            texture_image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            1,
        );

        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_buffer_memory, None);
        }
        Self {
            image: texture_image,
            memory: texture_image_memory,
            name: name.to_string(),
            device: device.clone(),
            info,
        }
    }

    pub fn create_srv(&self) -> vk::ImageView {
        create_image_view(
            &self.device,
            self.image,
            self.info.format,
            vk::ImageAspectFlags::COLOR,
            self.info.mip_levels,
        )
    }

    pub fn create_dsv(&self) -> vk::ImageView {
        let flag = match self.info.format {
            vk::Format::D16_UNORM_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D32_SFLOAT_S8_UINT => {
                vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
            }
            _ => vk::ImageAspectFlags::DEPTH,
        };
        create_image_view(
            &self.device,
            self.image,
            self.info.format,
            flag,
            self.info.mip_levels,
        )
    }
}


fn transition_image_layout(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    submit_queue: vk::Queue,
    image: vk::Image,
    _format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    mip_levels: u32,
) {
    record_single_time_submit_commandbuffer(
        device,
        command_pool,
        submit_queue,
        |device: &ash::Device, command_buffer: vk::CommandBuffer| {
            let src_access_mask;
            let dst_access_mask;
            let source_stage;
            let destination_stage;

            if old_layout == vk::ImageLayout::UNDEFINED
                && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
            {
                src_access_mask = vk::AccessFlags::empty();
                dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
                destination_stage = vk::PipelineStageFlags::TRANSFER;
            } else if old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
                && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            {
                src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                dst_access_mask = vk::AccessFlags::SHADER_READ;
                source_stage = vk::PipelineStageFlags::TRANSFER;
                destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
            } else if old_layout == vk::ImageLayout::UNDEFINED
                && new_layout == vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
            {
                src_access_mask = vk::AccessFlags::empty();
                dst_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_READ
                    | vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
                source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
                destination_stage = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
            } else {
                panic!("Unsupported layout transition!")
            }

            let image_barriers = [vk::ImageMemoryBarrier::default()
                .src_access_mask(src_access_mask)
                .dst_access_mask(dst_access_mask)
                .old_layout(old_layout)
                .new_layout(new_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: mip_levels,
                    base_array_layer: 0,
                    layer_count: 1,
                })];

            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    source_stage,
                    destination_stage,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &image_barriers,
                );
            }
        },
    );
}

pub fn copy_buffer_to_image(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    submit_queue: vk::Queue,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
) {
    record_single_time_submit_commandbuffer(
        device,
        command_pool,
        submit_queue,
        |device: &ash::Device, command_buffer| {
            let buffer_image_regions = [vk::BufferImageCopy {
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                buffer_offset: 0,
                buffer_image_height: 0,
                buffer_row_length: 0,
                image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            }];

            unsafe {
                device.cmd_copy_buffer_to_image(
                    command_buffer,
                    buffer,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &buffer_image_regions,
                );
            }
        },
    );
}

pub fn load_cubemap(
    device: Arc<ash::Device>,
    command_pool: vk::CommandPool,
    submit_queue: vk::Queue,
    device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
    dir_path: &Path,
) -> Vec<Texture> {
    let dir = std::fs::read_dir(dir_path).expect("Failed to read directory");
    let mut images = Vec::new();
    for entry in dir {
        let path = entry.unwrap().path();
        let image = Texture::load_from_path(
            &device,
            command_pool,
            submit_queue,
            device_memory_properties,
            &path,
            path.to_str().expect("Failed to convert path to string!"),
        );
        images.push(image);
    }
    images
}
