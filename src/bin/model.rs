use std::ffi::CString;
use std::path::Path;
use std::sync::Arc;

use ash::vk::{self, PipelineLayoutCreateInfo};
use nalgebra_glm::{Mat4x4, Vec3};
use vk_rs::base::VulkanBase;
use vk_rs::camera::{Camera, Direction};
use vk_rs::structures::{InputState, RenderResource, RenderState, Vertex};
use vk_rs::util::buffer::{create_index_buffer, create_vertex_buffer};
use vk_rs::util::command_buffer::{create_command_buffers, record_submit_commandbuffer};
use vk_rs::util::descriptor::{
    create_descriptor_set_layout, create_descriptor_sets, create_uniform_buffers,
};
use vk_rs::util::fps_limiter::FPSLimiter;
use vk_rs::util::framebuffer::create_framebuffers;
use vk_rs::util::image::{create_image_view, create_image_views, create_texture_image};
use vk_rs::util::pipeline::{
    create_graphics_pipeline, create_pipeline_layout, create_render_pass, create_shader_module,
    load_spirv,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};
const TEXTURE_PATH: &'static str = "assets/chalet.jpg";
const MODEL_PATH: &'static str = "assets/chalet.obj";

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct UniformBufferObject {
    pub model: Mat4x4,
    pub view: Mat4x4,
    pub proj: Mat4x4,
}

struct VulkanResource {
    //ref
    device: Arc<ash::Device>,
    command_pool: vk::CommandPool,
    resoultion: vk::Extent2D,

    //pipeline
    pub render_pass: vk::RenderPass,
    pub ubo_layout: vk::DescriptorSetLayout,

    pub pipeline_layout: vk::PipelineLayout,
    pub graphics_pipeline: vk::Pipeline,

    //raw data
    pub vertices: Vec<vk_rs::structures::Vertex>,
    pub indices: Vec<u32>,

    //vk buffer and memory
    pub vertex_buffer: vk::Buffer,
    pub index_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer_memory: vk::DeviceMemory,

    //texture
    pub texture_image: vk::Image,
    pub texture_image_view: vk::ImageView,
    pub texture_image_memory: vk::DeviceMemory,

    pub descriptor_sets: Vec<vk::DescriptorSet>,

    pub swapchain_imageviews: Vec<vk::ImageView>,
    pub swapchain_framebuffers: Vec<vk::Framebuffer>,

    pub command_buffers: Vec<vk::CommandBuffer>,

    pub uniform_transform: UniformBufferObject,
    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,

    //util
    pub camera: Camera,
}

impl Drop for VulkanResource {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);

            self.device
                .free_command_buffers(self.command_pool, &self.command_buffers);
            for &framebuffer in self.swapchain_framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }

            for &image_view in self.swapchain_imageviews.iter() {
                self.device.destroy_image_view(image_view, None);
            }

            for i in 0..self.uniform_buffers.len() {
                self.device.destroy_buffer(self.uniform_buffers[i], None);
                self.device
                    .free_memory(self.uniform_buffers_memory[i], None);
            }

            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);

            self.device
                .destroy_image_view(self.texture_image_view, None);
            self.device.destroy_image(self.texture_image, None);
            self.device.free_memory(self.texture_image_memory, None);

            self.device
                .destroy_descriptor_set_layout(self.ubo_layout, None);
        }
    }
}

impl RenderState for VulkanResource {
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

    fn record_command_buffer(&mut self) -> impl Fn() -> () {
        let record=||{
            for (i, &command_buffer) in self.command_buffers.iter().enumerate() {
                let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                unsafe {
                    self.device
                        .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                        .expect("Failed to begin recording Command Buffer at beginning!");
                }
    
                let clear_values = [
                    vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    },
                    vk::ClearValue {
                        // clear value for depth buffer
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    },
                ];
    
                let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                    .render_pass(self.render_pass)
                    .framebuffer(self.swapchain_framebuffers[i])
                    .clear_values(&clear_values)
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.resoultion,
                    });
    
                unsafe {
                    self.device.cmd_begin_render_pass(
                        command_buffer,
                        &render_pass_begin_info,
                        vk::SubpassContents::INLINE,
                    );
    
                    self.device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.graphics_pipeline,
                    );
                    let vertex_buffers = [self.vertex_buffer];
                    let offsets = [0_u64];
                    let descriptor_sets_to_bind = [self.descriptor_sets[i]];
    
                    self.device
                        .cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
                    self.device.cmd_bind_index_buffer(
                        command_buffer,
                        self.index_buffer,
                        0,
                        vk::IndexType::UINT32,
                    );
                    self.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        0,
                        &descriptor_sets_to_bind,
                        &[],
                    );
    
                    self.device
                        .cmd_draw_indexed(command_buffer, self.index_count(), 1, 0, 0, 0);
    
                    self.device.cmd_end_render_pass(command_buffer);
    
                    self.device
                        .end_command_buffer(command_buffer)
                        .expect("Failed to record Command Buffer at Ending!");
                }
    
        }
        };
        record
    }

    fn recreate(&mut self, vk: &VulkanBase) {
        unsafe {
            self.device.destroy_render_pass(self.render_pass, None);
            self.device
                .free_command_buffers(self.command_pool, &self.command_buffers);
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            for &framebuffer in self.swapchain_framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }
        }

        self.render_pass = create_render_pass(
            &vk.instance,
            &self.device,
            vk.physical_device,
            vk.swapchain_format,
        );
        self.command_buffers = create_command_buffers(
            &self.device,
            self.command_pool,
            &self.swapchain_framebuffers,
        );
        (self.graphics_pipeline, self.pipeline_layout) = prepare_graphics_pipeline(
            &self.device,
            self.render_pass,
            vk.swapchain_extent,
            self.ubo_layout,
        );
        self.swapchain_framebuffers = create_framebuffers(
            &self.device,
            self.render_pass,
            &vk.swapchain_imageviews,
            vk.depth_image_view,
            vk.swapchain_extent,
        );
        self.resoultion = vk.swapchain_extent;
        // self.record_command_buffer();
    }
}

impl RenderResource for VulkanResource {
    fn vertex_buffer(&self) -> ash::vk::Buffer {
        self.vertex_buffer
    }

    fn index_buffer(&self) -> ash::vk::Buffer {
        self.index_buffer
    }

    fn vertex_count(&self) -> u32 {
        self.vertices.len() as u32
    }

    fn index_count(&self) -> u32 {
        self.indices.len() as u32
    }

    fn command_buffers(&self) -> &[vk::CommandBuffer] {
        &self.command_buffers
    }

    fn fetch_command_buffer(&self, index: usize) -> &vk::CommandBuffer {
        &self.command_buffers[index]
    }
}

#[derive(Default)]
struct ModelApp {
    window: Option<Arc<Window>>,
    resource: Option<VulkanResource>,
    vk: Option<VulkanBase>,
    timer: Option<FPSLimiter>,

    //helper
    state: InputState,
}

fn prepare(vk: &VulkanBase) -> VulkanResource {
    let (vertices, indices) = vk_rs::util::load_model(&Path::new(MODEL_PATH));

    let (texture_image, texture_image_memory) = create_texture_image(
        &vk.device,
        vk.command_pool,
        vk.graphics_queue,
        &vk.memory_properties,
        Path::new(TEXTURE_PATH),
    );
    let texture_image_view = create_image_view(
        &vk.device,
        texture_image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageAspectFlags::COLOR,
        1,
    );

    let swapchain_imageviews =
        create_image_views(&vk.device, vk.swapchain_format, &vk.swapchain_images);

    let render_pass = create_render_pass(
        &vk.instance,
        &vk.device,
        vk.physical_device,
        vk.swapchain_format,
    );

    let ubo_layout = create_descriptor_set_layout(&vk.device);

    let swapchain_framebuffers = create_framebuffers(
        &vk.device,
        render_pass,
        &swapchain_imageviews,
        vk.depth_image_view,
        vk.swapchain_extent,
    );

    let command_buffers =
        create_command_buffers(&vk.device, vk.command_pool, &swapchain_framebuffers);

    let (graphics_pipeline, pipeline_layout) =
        prepare_graphics_pipeline(&vk.device, render_pass, vk.swapchain_extent, ubo_layout);

    let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
        &vk.device,
        &vk.memory_properties,
        vk.command_pool,
        vk.graphics_queue,
        &vertices,
    );

    let (index_buffer, index_buffer_memory) = create_index_buffer(
        &vk.device,
        &vk.memory_properties,
        vk.command_pool,
        vk.graphics_queue,
        &indices,
    );

    let (uniform_buffers, uniform_buffers_memory) = create_uniform_buffers::<UniformBufferObject>(
        &vk.device,
        &vk.memory_properties,
        vk.swapchain_images.len(),
    );

    let descriptor_sets = create_descriptor_sets::<UniformBufferObject>(
        &vk.device,
        vk.descriptor_pool,
        ubo_layout,
        &uniform_buffers,
        texture_image_view,
        vk.texture_sampler,
        vk.swapchain_images.len(),
    );
    let camera = Camera::new(
        Vec3::new(2.0, 5.0, 2.0),
        -Vec3::new(2.0, 5.0, 2.0),
        Vec3::new(0.0, 0.0, 1.0),
        vk.swapchain_extent.width as f32 / vk.swapchain_extent.height as f32,
        45.0_f32.to_radians(),
    );

    let uniform_transform: UniformBufferObject = UniformBufferObject {
        model: camera.get_model(),
        view: camera.get_view_matrix(),
        proj: camera.get_perspective_projection_matrix(),
    };
    let resoultion = vk.swapchain_extent;
    VulkanResource {
        device: vk.device.clone(),
        command_pool: vk.command_pool,
        render_pass,
        ubo_layout,
        pipeline_layout,
        graphics_pipeline,
        vertices,
        indices,
        vertex_buffer,
        index_buffer,
        vertex_buffer_memory,
        index_buffer_memory,
        texture_image,
        texture_image_view,
        texture_image_memory,
        descriptor_sets,
        swapchain_framebuffers,
        command_buffers,
        camera,
        uniform_transform,
        uniform_buffers,
        uniform_buffers_memory,
        swapchain_imageviews,
        resoultion,
    }
}

fn prepare_graphics_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    swapchain_extent: vk::Extent2D,
    ubo_set_layout: vk::DescriptorSetLayout,
) -> (vk::Pipeline, vk::PipelineLayout) {
    let vertex_code = load_spirv("shader/depth/depth.vert.spv");
    let frag_code = load_spirv("shader/depth/depth.frag.spv");

    let vert_shader_module = create_shader_module(device, vertex_code);
    let frag_shader_module = create_shader_module(device, frag_code);

    let main_function_name = CString::new("main").unwrap(); // the beginning function name in shader code.

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo {
            module: vert_shader_module,
            p_name: main_function_name.as_ptr(),
            stage: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        },
        vk::PipelineShaderStageCreateInfo {
            module: frag_shader_module,
            p_name: main_function_name.as_ptr(),
            stage: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        },
    ];

    let binding_description = Vertex::get_binding_descriptions();
    let attribute_description = Vertex::get_attribute_descriptions();

    let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_attribute_descriptions(&attribute_description)
        .vertex_binding_descriptions(&binding_description);
    let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        ..Default::default()
    };

    let viewports = [vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: swapchain_extent.width as f32,
        height: swapchain_extent.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    }];

    let scissors = [vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: swapchain_extent,
    }];

    let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
        .scissors(&scissors)
        .viewports(&viewports);

    let rasterization_statue_create_info = vk::PipelineRasterizationStateCreateInfo {
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        line_width: 1.0,
        polygon_mode: vk::PolygonMode::FILL,
        ..Default::default()
    };

    let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo {
        rasterization_samples: vk::SampleCountFlags::TYPE_1,
        ..Default::default()
    };

    let stencil_state = vk::StencilOpState {
        fail_op: vk::StencilOp::KEEP,
        pass_op: vk::StencilOp::KEEP,
        depth_fail_op: vk::StencilOp::KEEP,
        compare_op: vk::CompareOp::ALWAYS,
        ..Default::default()
    };

    let depth_state_create_info = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: 1,
        depth_write_enable: 1,
        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
        depth_bounds_test_enable: 0,
        stencil_test_enable: 0,
        front: stencil_state,
        back: stencil_state,
        min_depth_bounds: 0.0,
        max_depth_bounds: 1.0,
        ..Default::default()
    };

    let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
        blend_enable: 0,
        src_color_blend_factor: vk::BlendFactor::ONE,
        dst_color_blend_factor: vk::BlendFactor::ZERO,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ONE,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::RGBA,
    }];

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op(vk::LogicOp::COPY)
        .logic_op_enable(false)
        .attachments(&color_blend_attachment_states)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

    let set_layouts = [ubo_set_layout];
    let pipeline_layout_create_info = PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);

    let pipeline_layout = unsafe {
        device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .expect("Failed to create pipeline layout!")
    };

    let graphic_pipeline_create_infos = [vk::GraphicsPipelineCreateInfo::default()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_state_create_info)
        .input_assembly_state(&vertex_input_assembly_state_info)
        .viewport_state(&viewport_state_info)
        .rasterization_state(&rasterization_statue_create_info)
        .multisample_state(&multisample_state_create_info)
        .depth_stencil_state(&depth_state_create_info)
        .color_blend_state(&color_blend_state)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .base_pipeline_index(-1)];

    let graphics_pipelines = unsafe {
        device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &graphic_pipeline_create_infos,
                None,
            )
            .expect("Failed to create Graphics Pipeline!.")
    };

    unsafe {
        device.destroy_shader_module(vert_shader_module, None);
        device.destroy_shader_module(frag_shader_module, None);
    }

    (graphics_pipelines[0], pipeline_layout)
}

impl ApplicationHandler for ModelApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        window.focus_window();
        self.vk = Some(VulkanBase::new(window.clone()));
        self.window = Some(window);
        self.timer = Some(FPSLimiter::new());
        self.resource = Some(prepare(self.vk.as_ref().unwrap()));
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
                    vk.draw_frame(&self.state, self.resource.as_mut().unwrap(), delta_time);
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
                    let camera = &mut self.resource.as_mut().unwrap().camera;
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

    let mut app = ModelApp::default();

    event_loop.run_app(&mut app).unwrap();
}
