
use ash::{
    util::read_spv,
    vk::{self, PipelineLayoutCreateInfo},
};


use super::find_depth_format;

pub fn create_render_pass(
    instance: &ash::Instance,
    device: &ash::Device,
    physcial_device: vk::PhysicalDevice,
    surface_format: vk::Format,
) -> vk::RenderPass {
    let color_attachment = vk::AttachmentDescription {
        format: surface_format,
        flags: vk::AttachmentDescriptionFlags::empty(),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
    };
    let depth_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: find_depth_format(instance, physcial_device),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::DONT_CARE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let color_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };
    let depth_attachment_ref = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let binding = [color_attachment_ref];
    let subpasses = [vk::SubpassDescription::default()
        .color_attachments(&binding)
        .depth_stencil_attachment(&depth_attachment_ref)
        .flags(vk::SubpassDescriptionFlags::empty())
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)];

    let render_pass_attachments = [color_attachment, depth_attachment];

    let subpass_dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        dst_subpass: 0,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        src_access_mask: vk::AccessFlags::empty(),
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
            | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        dependency_flags: vk::DependencyFlags::empty(),
    }];

    let renderpass_create_info = vk::RenderPassCreateInfo::default()
        .flags(vk::RenderPassCreateFlags::empty())
        .attachments(&render_pass_attachments)
        .subpasses(&subpasses)
        .dependencies(&subpass_dependencies);

    unsafe {
        device
            .create_render_pass(&renderpass_create_info, None)
            .expect("Failed to create render pass!")
    }
}

pub fn create_graphics_pipeline(
    device: &ash::Device,
    graphic_pipeline_create_infos: &[vk::GraphicsPipelineCreateInfo],
) -> vk::Pipeline {
    // let mut vertex_spv_file = Cursor::new(&include_bytes!("../../shader/depth/depth.vert.spv")[..]);
    // let mut frag_spv_file = Cursor::new(&include_bytes!("../../shader/depth/depth.frag.spv")[..]);

    // let vertex_code: Vec<u32> =
    //     read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
    // let frag_code = read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");

    // let vert_shader_module = create_shader_module(device, vertex_code);
    // let frag_shader_module = create_shader_module(device, frag_code);

    // let main_function_name = CString::new("main").unwrap(); // the beginning function name in shader code.

    // let shader_stages = [
    //     vk::PipelineShaderStageCreateInfo {
    //         module: vert_shader_module,
    //         p_name: main_function_name.as_ptr(),
    //         stage: vk::ShaderStageFlags::VERTEX,
    //         ..Default::default()
    //     },
    //     vk::PipelineShaderStageCreateInfo {
    //         module: frag_shader_module,
    //         p_name: main_function_name.as_ptr(),
    //         stage: vk::ShaderStageFlags::FRAGMENT,
    //         ..Default::default()
    //     },
    // ];

    // let binding_description = Vertex::get_binding_descriptions();
    // let attribute_description = Vertex::get_attribute_descriptions();

    // let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::default()
    //     .vertex_attribute_descriptions(&attribute_description)
    //     .vertex_binding_descriptions(&binding_description);

    // let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
    //     topology: vk::PrimitiveTopology::TRIANGLE_LIST,
    //     ..Default::default()
    // };

    // let viewports = [vk::Viewport {
    //     x: 0.0,
    //     y: 0.0,
    //     width: swapchain_extent.width as f32,
    //     height: swapchain_extent.height as f32,
    //     min_depth: 0.0,
    //     max_depth: 1.0,
    // }];

    // let scissors = [vk::Rect2D {
    //     offset: vk::Offset2D { x: 0, y: 0 },
    //     extent: swapchain_extent,
    // }];

    // let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
    //     .scissors(&scissors)
    //     .viewports(&viewports);

    // let rasterization_statue_create_info = vk::PipelineRasterizationStateCreateInfo {
    //     front_face: vk::FrontFace::COUNTER_CLOCKWISE,
    //     line_width: 1.0,
    //     polygon_mode: vk::PolygonMode::FILL,
    //     ..Default::default()
    // };

    // let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo {
    //     rasterization_samples: vk::SampleCountFlags::TYPE_1,
    //     ..Default::default()
    // };

    // let stencil_state = vk::StencilOpState {
    //     fail_op: vk::StencilOp::KEEP,
    //     pass_op: vk::StencilOp::KEEP,
    //     depth_fail_op: vk::StencilOp::KEEP,
    //     compare_op: vk::CompareOp::ALWAYS,
    //     ..Default::default()
    // };

    // let depth_state_create_info = vk::PipelineDepthStencilStateCreateInfo {
    //     depth_test_enable: 1,
    //     depth_write_enable: 1,
    //     depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
    //     depth_bounds_test_enable: 0,
    //     stencil_test_enable: 0,
    //     front: stencil_state,
    //     back: stencil_state,
    //     min_depth_bounds: 0.0,
    //     max_depth_bounds: 1.0,
    //     ..Default::default()
    // };

    // let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
    //     blend_enable: 0,
    //     src_color_blend_factor: vk::BlendFactor::ONE,
    //     dst_color_blend_factor: vk::BlendFactor::ZERO,
    //     color_blend_op: vk::BlendOp::ADD,
    //     src_alpha_blend_factor: vk::BlendFactor::ONE,
    //     dst_alpha_blend_factor: vk::BlendFactor::ZERO,
    //     alpha_blend_op: vk::BlendOp::ADD,
    //     color_write_mask: vk::ColorComponentFlags::RGBA,
    // }];

    // let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
    //     .logic_op(vk::LogicOp::COPY)
    //     .logic_op_enable(false)
    //     .attachments(&color_blend_attachment_states)
    //     .blend_constants([0.0, 0.0, 0.0, 0.0]);

    // let set_layouts = [ubo_set_layout];
    // let pipeline_layout_create_info = PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);

    // let pipeline_layout = unsafe {
    //     device
    //         .create_pipeline_layout(&pipeline_layout_create_info, None)
    //         .expect("Failed to create pipeline layout!")
    // };

    // let graphic_pipeline_create_infos = [vk::GraphicsPipelineCreateInfo::default()
    //     .stages(&shader_stages)
    //     .vertex_input_state(&vertex_input_state_create_info)
    //     .input_assembly_state(&vertex_input_assembly_state_info)
    //     .viewport_state(&viewport_state_info)
    //     .rasterization_state(&rasterization_statue_create_info)
    //     .multisample_state(&multisample_state_create_info)
    //     .depth_stencil_state(&depth_state_create_info)
    //     .color_blend_state(&color_blend_state)
    //     .layout(pipeline_layout)
    //     .render_pass(render_pass)
    //     .base_pipeline_index(-1)];

    let graphics_pipelines = unsafe {
        device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &graphic_pipeline_create_infos,
                None,
            )
            .expect("Failed to create Graphics Pipeline!.")
    };

    // unsafe {
    //     device.destroy_shader_module(vert_shader_module, None);
    //     device.destroy_shader_module(frag_shader_module, None);
    // }

    graphics_pipelines[0]
}

pub fn create_shader_module(device: &ash::Device, code: Vec<u32>) -> vk::ShaderModule {
    let shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&code);
    unsafe {
        device
            .create_shader_module(&shader_module_create_info, None)
            .expect("Failed to create Shader Module!")
    }
}

pub fn create_pipeline_layout(
    device: &ash::Device,
    ubo_set_layout: &vk::DescriptorSetLayout,
) -> vk::PipelineLayout {
    let set_layouts = [*ubo_set_layout];
    let pipeline_layout_create_info = PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);

    let pipeline_layout = unsafe {
        device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .expect("Failed to create pipeline layout!")
    };
    pipeline_layout
}

pub fn load_spirv(path: &str) -> Vec<u32> {
    let mut file = std::fs::File::open(path).expect("Failed to open file");
    let spirv_code = read_spv(&mut file).expect("Failed to read vertex shader spv file");
    spirv_code
}

