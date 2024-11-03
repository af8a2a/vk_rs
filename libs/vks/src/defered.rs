use ash::vk;

use crate::{create_sampler, Context, Image, ImageParameters, Texture};
use std::{collections::HashMap, sync::Arc};

pub const GBUFFER_NORMALS_FORMAT: vk::Format = vk::Format::R16G16B16A16_SFLOAT;
pub const SCENE_COLOR_FORMAT: vk::Format = vk::Format::R32G32B32A32_SFLOAT;


pub struct GBuffer {
    pub scene_color: Texture,
    pub scene_depth: Texture,
    pub gbuffer_normals: Texture,
    pub gbuffer_depth: Texture,
    pub scene_resolve: Option<Texture>,
    pub attachment: HashMap<String, Texture>,
}

impl GBuffer {
    pub fn new(
        context: &Arc<Context>,
        extent: vk::Extent2D,
        depth_format: vk::Format,
        msaa_samples: vk::SampleCountFlags,
    ) -> Self {
        let gbuffer_normals = create_gbuffer_normals(context, extent);
        let gbuffer_depth = create_gbuffer_depth(context, depth_format, extent);
        let scene_color = create_scene_color(context, extent, msaa_samples);
        let scene_depth = create_scene_depth(context, depth_format, extent, msaa_samples);
        let scene_resolve = match msaa_samples {
            vk::SampleCountFlags::TYPE_1 => None,
            _ => Some(create_scene_resolve(context, extent)),
        };

        Self {
            gbuffer_normals,
            gbuffer_depth,
            scene_color,
            scene_depth,
            scene_resolve,
            attachment: HashMap::new(),
        }
    }
}

fn create_gbuffer_normals(context: &Arc<Context>, extent: vk::Extent2D) -> Texture {
    let image = Image::create(
        Arc::clone(context),
        ImageParameters {
            mem_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            extent,
            sample_count: vk::SampleCountFlags::TYPE_1,
            format: GBUFFER_NORMALS_FORMAT,
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            ..Default::default()
        },
    );

    image.transition_image_layout(
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    );

    let view = image.create_view(vk::ImageViewType::TYPE_2D, vk::ImageAspectFlags::COLOR);
    let sampler = Some(create_sampler(
        context,
        vk::Filter::NEAREST,
        vk::Filter::NEAREST,
    ));

    Texture::new(Arc::clone(context), image, view, sampler)
}

fn create_gbuffer_depth(
    context: &Arc<Context>,
    format: vk::Format,
    extent: vk::Extent2D,
) -> Texture {
    let image = Image::create(
        Arc::clone(context),
        ImageParameters {
            mem_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            extent,
            sample_count: vk::SampleCountFlags::TYPE_1,
            format,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            ..Default::default()
        },
    );

    image.transition_image_layout(
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    );

    let view = image.create_view(vk::ImageViewType::TYPE_2D, vk::ImageAspectFlags::DEPTH);

    let sampler = Some(create_sampler(
        context,
        vk::Filter::NEAREST,
        vk::Filter::NEAREST,
    ));

    Texture::new(Arc::clone(context), image, view, sampler)
}

fn create_scene_color(
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

fn create_scene_depth(
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

fn create_scene_resolve(context: &Arc<Context>, extent: vk::Extent2D) -> Texture {
    let image = Image::create(
        Arc::clone(context),
        ImageParameters {
            mem_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            extent,
            format: SCENE_COLOR_FORMAT,
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            ..Default::default()
        },
    );

    image.transition_image_layout(
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    );

    let view = image.create_view(vk::ImageViewType::TYPE_2D, vk::ImageAspectFlags::COLOR);

    let sampler = create_sampler(context, vk::Filter::NEAREST, vk::Filter::NEAREST);

    Texture::new(Arc::clone(context), image, view, Some(sampler))
}
