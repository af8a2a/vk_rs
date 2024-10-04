use ash::vk;

pub fn create_framebuffers(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    image_views: &Vec<vk::ImageView>,
    depth_image_view: vk::ImageView,
    swapchain_extent: vk::Extent2D,
) -> Vec<vk::Framebuffer> {
    let mut framebuffers = vec![];

    for &image_view in image_views.iter() {
        let attachments = [image_view, depth_image_view];

        let framebuffer_create_info = vk::FramebufferCreateInfo::default()
            .attachments(&attachments)
            .render_pass(render_pass)
            .width(swapchain_extent.width)
            .height(swapchain_extent.height)
            .layers(1)
            .flags(vk::FramebufferCreateFlags::empty());

        let framebuffer = unsafe {
            device
                .create_framebuffer(&framebuffer_create_info, None)
                .expect("Failed to create Framebuffer!")
        };

        framebuffers.push(framebuffer);
    }

    framebuffers
}
