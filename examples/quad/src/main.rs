use std::{error::Error, ffi::CString, io::Cursor, sync::Arc, time::Instant};

use ash::{
    util::read_spv,
    vk::{self, Extent2D, PipelineLayoutCreateInfo, RenderingAttachmentInfo, RenderingInfo},
    Device,
};
use tracing::{debug, info, Level};
use vks::{
    allocate_command_buffers, cmd_transition_images_layouts, create_device_local_buffer_with_data,
    create_pipeline, Buffer, Camera, Context, Descriptors, LayoutTransition, MipsRange,
    PipelineParameters, RenderError, ShaderParameters, Swapchain, SwapchainSupportDetails, Texture,
    Vertex, VulkanExampleBase, WindowApp,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::Key,
    window::{Fullscreen, Window, WindowId},
};
pub const HDR_SURFACE_FORMAT: vk::SurfaceFormatKHR = vk::SurfaceFormatKHR {
    format: vk::Format::R16G16B16A16_SFLOAT,
    color_space: vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT,
};

struct App {
    window: Option<Window>,
    triangle_app: Option<TriangleApp>,
}
impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            window: None,
            triangle_app: None,
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Triangle")
                    .with_inner_size(PhysicalSize::new(800, 600)),
            )
            .expect("Failed to create window");

        self.triangle_app = Some(TriangleApp::new(&window, true));
        self.window = Some(window);
    }

    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {
        if let Some(app) = self.triangle_app.as_mut() {
            app.new_frame();
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        self.triangle_app
            .as_mut()
            .unwrap()
            .end_frame(self.window.as_ref().unwrap());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
        }

        self.triangle_app
            .as_mut()
            .unwrap()
            .handle_window_event(self.window.as_ref().unwrap(), &event);
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        self.triangle_app
            .as_mut()
            .unwrap()
            .handle_device_event(&event);
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        self.triangle_app.as_mut().unwrap().on_exit();
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct QuadVertex {
    position: [f32; 2],
    coords: [f32; 2],
}

impl Vertex for QuadVertex {
    fn get_bindings_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<QuadVertex>() as _,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn get_attributes_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 8,
            },
        ]
    }
}

struct QuadModel {
    vertices: Buffer,
    indices: Buffer,
}

impl QuadModel {
    fn new(context: &Arc<Context>) -> Self {
        let indices: [u32; 6] = [0, 1, 2, 2, 3, 0];
        let indices = create_device_local_buffer_with_data::<u8, _>(
            context,
            vk::BufferUsageFlags::INDEX_BUFFER,
            &indices,
        );
        let vertices: [f32; 16] = [
            -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 0.0,
        ];
        let vertices = create_device_local_buffer_with_data::<u8, _>(
            context,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            &vertices,
        );

        Self { vertices, indices }
    }
}

pub struct TriangleApp {
    base: VulkanExampleBase,
    model: QuadModel,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,

    camera: Camera,
    time: Instant,
    dirty_swapchain: bool,
}

fn prepare_pipeline(context: &Arc<Context>) -> (vk::Pipeline, vk::PipelineLayout) {
    let device = context.device();
    let layout = {
        let layout_info = vk::PipelineLayoutCreateInfo::default();

        unsafe { device.create_pipeline_layout(&layout_info, None).unwrap() }
    };

    let pipeline = {
        let viewport_info = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let multisampling_info = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .blend_enable(false)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)];

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(false)
            .front(Default::default())
            .back(Default::default());

        create_pipeline::<QuadVertex>(
            context,
            PipelineParameters {
                vertex_shader_params: ShaderParameters::new("quad"),
                fragment_shader_params: ShaderParameters::new("quad"),
                multisampling_info: &multisampling_info,
                viewport_info: &viewport_info,
                rasterizer_info: &rasterizer_info,
                dynamic_state_info: Some(&dynamic_state_info),
                depth_stencil_info: Some(&depth_stencil_info),
                color_blend_attachments: &color_blend_attachments,
                color_attachment_formats: &[vk::Format::R16G16B16A16_SFLOAT],
                depth_attachment_format: None,
                layout,
                parent: None,
                allow_derivatives: false,
            },
        )
    };

    (pipeline, layout)
}

pub fn create_shader_module(device: &ash::Device, code: Vec<u32>) -> vk::ShaderModule {
    let shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&code);
    unsafe {
        device
            .create_shader_module(&shader_module_create_info, None)
            .expect("Failed to create Shader Module!")
    }
}

impl TriangleApp {
    fn new(window: &Window, enable_debug: bool) -> Self {
        let base = VulkanExampleBase::new(window,enable_debug);
        let context = &base.context;
        let model = QuadModel::new(context);

        let (pipeline, pipeline_layout) = prepare_pipeline(context);
        Self {
            model,
            camera: Camera::default(),
            time: Instant::now(),
            dirty_swapchain: false,
            pipeline_layout,
            pipeline,
            base,
        }
    }
}

impl WindowApp for TriangleApp {
    fn new_frame(&mut self) {}

    fn handle_window_event(&mut self, _window: &Window, event: &WindowEvent) {
        match event {
            // Dropped file
            WindowEvent::DroppedFile(_) => {
                // tracing::debug!("File dropped: {:?}", path);
                // self.loader.load(path.clone());
            }
            // Resizing
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                tracing::debug!("resize {:?}", (width, height));

                self.dirty_swapchain = true;
            }
            // Key events
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Character(c),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if c == "h" {
                    // self.enable_ui = !self.enable_ui;
                }
            }
            _ => (),
        }
    }

    fn handle_device_event(&mut self, event: &DeviceEvent) {
        // self.input_state = self.input_state.handle_device_event(event);
    }

    fn recreate_swapchain(&mut self, dimensions: [u32; 2], vsync: bool, hdr: bool) {
        tracing::debug!("Recreating swapchain.");

        self.base.context.graphics_queue_wait_idle();

        unsafe {
            self.base.context.device().free_command_buffers(
                self.base.context.general_command_pool(),
                &self.base.command_buffers,
            )
        };

        let swapchain_support_details = SwapchainSupportDetails::new(
            self.base.context.physical_device(),
            self.base.context.surface(),
            self.base.context.surface_khr(),
        );

        self.base.swapchain = Swapchain::create(
            Arc::clone(&self.base.context),
            swapchain_support_details,
            dimensions,
            hdr.then_some(HDR_SURFACE_FORMAT),
            vsync,
        );

        self.base.on_new_swapchain();
        self.base.command_buffers =
            allocate_command_buffers(&self.base.context, self.base.swapchain.image_count());
    }

    fn end_frame(&mut self, window: &Window) {
        let new_time = Instant::now();
        let delta_s = (new_time - self.time).as_secs_f32();
        self.time = new_time;

        // If swapchain must be recreated wait for windows to not be minimized anymore
        if self.dirty_swapchain {
            let PhysicalSize { width, height } = window.inner_size();
            if width > 0 && height > 0 {
                self.base
                    .recreate_swapchain(window.inner_size().into(), false, true);
            } else {
                return;
            }
        }
        self.dirty_swapchain = matches!(
            self.render(window, self.camera),
            Err(RenderError::DirtySwapchain)
        );
    }

    fn on_exit(&mut self) {
        self.base.wait_idle_gpu();
    }

    fn render(&mut self, window: &Window, camera: Camera) -> Result<(), RenderError> {
        tracing::trace!("Drawing frame.");
        let sync_objects = self.base.in_flight_frames.next().unwrap();
        let image_available_semaphore = sync_objects.image_available_semaphore;
        let render_finished_semaphore = sync_objects.render_finished_semaphore;
        let in_flight_fence = sync_objects.fence;
        let wait_fences = [in_flight_fence];

        unsafe {
            self.base
                .context
                .device()
                .wait_for_fences(&wait_fences, true, u64::MAX)
                .unwrap()
        };

        let result =
            self.base
                .swapchain
                .acquire_next_image(None, Some(image_available_semaphore), None);
        let image_index = match result {
            Ok((image_index, _)) => image_index,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                return Err(RenderError::DirtySwapchain);
            }
            Err(error) => panic!("Error while acquiring next image. Cause: {}", error),
        };

        unsafe {
            self.base
                .context
                .device()
                .reset_fences(&wait_fences)
                .unwrap()
        };

        // record_command_buffer
        {
            let command_buffer = self.base.command_buffers[image_index as usize];
            let frame_index = image_index as _;

            unsafe {
                self.base
                    .context
                    .device()
                    .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                    .unwrap();
            }

            // begin command buffer
            {
                let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);
                unsafe {
                    self.base
                        .context
                        .device()
                        .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                        .unwrap()
                };
            }

            self.cmd_draw(command_buffer, frame_index);

            // End command buffer
            unsafe {
                self.base
                    .context
                    .device()
                    .end_command_buffer(command_buffer)
                    .unwrap()
            };
        }

        // Submit command buffer
        {
            let wait_semaphore_submit_info = vk::SemaphoreSubmitInfo::default()
                .semaphore(image_available_semaphore)
                .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT);

            let signal_semaphore_submit_info = vk::SemaphoreSubmitInfo::default()
                .semaphore(render_finished_semaphore)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS);

            let cmd_buffer_submit_info = vk::CommandBufferSubmitInfo::default()
                .command_buffer(self.base.command_buffers[image_index as usize]);

            let submit_info = vk::SubmitInfo2::default()
                .command_buffer_infos(std::slice::from_ref(&cmd_buffer_submit_info))
                .wait_semaphore_infos(std::slice::from_ref(&wait_semaphore_submit_info))
                .signal_semaphore_infos(std::slice::from_ref(&signal_semaphore_submit_info));

            unsafe {
                self.base
                    .context
                    .synchronization2()
                    .queue_submit2(
                        self.base.context.graphics_compute_queue(),
                        std::slice::from_ref(&submit_info),
                        in_flight_fence,
                    )
                    .unwrap()
            };
        }

        let swapchains = [self.base.swapchain.swapchain_khr()];
        let images_indices = [image_index];

        {
            let signal_semaphores = [render_finished_semaphore];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&images_indices);

            match self.base.swapchain.present(&present_info) {
                Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    return Err(RenderError::DirtySwapchain)
                }
                Err(error) => panic!("Failed to present queue. Cause: {}", error),
                _ => {}
            }
        }

        Ok(())
    }

    fn cmd_draw(&mut self, command_buffer: vk::CommandBuffer, frame_index: usize) {
        // Prepare attachments and inputs for lighting pass
        let transitions = vec![
            LayoutTransition {
                image: &self.base.scene_color.image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                mips_range: MipsRange::All,
            },
            LayoutTransition {
                image: &self.base.scene_depth.image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                mips_range: MipsRange::All,
            },
        ];
        cmd_transition_images_layouts(command_buffer, &transitions);
        let (image, image_view) = (
            &self.base.swapchain.images()[frame_index],
            &self.base.swapchain.image_views()[frame_index],
        );
        // Scene Pass
        {
            // let extent = vk::Extent2D {
            //     width: self.base.scene_color.image.extent.width,
            //     height: self.base.scene_color.image.extent.height,
            // };
            let extent = vk::Extent2D {
                width: image.extent.width,
                height: image.extent.height,
            };

            unsafe {
                self.base.context.device().cmd_set_viewport(
                    command_buffer,
                    0,
                    &[vk::Viewport {
                        width: extent.width as _,
                        height: extent.height as _,
                        max_depth: 1.0,
                        ..Default::default()
                    }],
                );
                self.base.context.device().cmd_set_scissor(
                    command_buffer,
                    0,
                    &[vk::Rect2D {
                        extent,
                        ..Default::default()
                    }],
                )
            }

            {
                let color_attachment_info = RenderingAttachmentInfo::default()
                    .clear_value(vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [1.0, 0.0, 0.0, 1.0],
                        },
                    })
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .image_view(*image_view)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE);

                let depth_attachment_info = RenderingAttachmentInfo::default()
                    .clear_value(vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    })
                    .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .image_view(self.base.scene_depth.view)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE);

                let rendering_info = RenderingInfo::default()
                    .color_attachments(std::slice::from_ref(&color_attachment_info))
                    .depth_attachment(&depth_attachment_info)
                    .layer_count(1)
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent,
                    });
                unsafe {
                    self.base
                        .context
                        .dynamic_rendering()
                        .cmd_begin_rendering(command_buffer, &rendering_info)
                };
            }
            let device = self.base.context.device();

            // Bind skybox pipeline
            unsafe {
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline,
                )
            };

            unsafe {
                device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[self.model.vertices.buffer],
                    &[0],
                );
            }

            unsafe {
                device.cmd_bind_index_buffer(
                    command_buffer,
                    self.model.indices.buffer,
                    0,
                    vk::IndexType::UINT32,
                );
            }

            // Draw skybox
            unsafe { device.cmd_draw_indexed(command_buffer, 36, 1, 0, 0, 0) };

            unsafe {
                self.base
                    .context
                    .dynamic_rendering()
                    .cmd_end_rendering(command_buffer)
            };
        }
        // Transition swapchain image for presentation
        {
            self.base.swapchain.images()[frame_index].cmd_transition_image_layout(
                command_buffer,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            );
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // builds the subscriber.
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    debug!("Hello, world!");
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
