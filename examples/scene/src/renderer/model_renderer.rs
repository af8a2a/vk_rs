use std::sync::Arc;

use ash::vk;
use gltf_model::{Model, ModelStagingResources, MAX_JOINTS_PER_MESH};
use math::cgmath::Matrix4;
use vks::{Buffer, Context, PreLoadedResource};

type JointsBuffer = [Matrix4<f32>; MAX_JOINTS_PER_MESH];

pub fn load_assets(
    context: Arc<Context>,
    path: impl AsRef<std::path::Path>,
) -> PreLoadedResource<Model, ModelStagingResources> {
    let device = context.device();

    // Create command buffer
    let command_buffer = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.general_command_pool())
            .level(vk::CommandBufferLevel::SECONDARY)
            .command_buffer_count(1);

        unsafe { device.allocate_command_buffers(&allocate_info).unwrap()[0] }
    };

    // Begin recording command buffer
    {
        let inheritance_info = vk::CommandBufferInheritanceInfo::default();
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .inheritance_info(&inheritance_info)
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .unwrap()
        };
    }

    let model = Model::create_from_file(context.clone(), command_buffer, path).unwrap();
    unsafe { device.end_command_buffer(command_buffer).unwrap() };

    model
}

pub struct ModelRender {
    context: Arc<Context>,
    model: Box<Model>,
    transform_ubos: Vec<Buffer>,
    skin_ubos: Vec<Buffer>,
    skin_matrices: Vec<Vec<JointsBuffer>>,
    materials_ubo: Buffer,
    light_ubos: Vec<Buffer>,
}
