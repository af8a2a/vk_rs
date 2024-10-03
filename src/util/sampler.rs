use ash::vk;

pub fn create_texture_sampler(device: &ash::Device) -> vk::Sampler {
    let sampler_create_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .mip_lod_bias(0.0)
        .anisotropy_enable(true)
        .max_anisotropy(16.0)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .min_lod(0.0)
        .max_lod(0.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false);

    unsafe {
        device
            .create_sampler(&sampler_create_info, None)
            .expect("Failed to create Sampler!")
    }
}
