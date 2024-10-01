use std::ffi::{self, c_char};

use ash::{ext::debug_utils, vk};
use winit::raw_window_handle::HasDisplayHandle;

pub fn create_instance(entry: &ash::Entry, window: &winit::window::Window) -> ash::Instance {
    unsafe {
        let app_name = ffi::CStr::from_bytes_with_nul_unchecked(b"VulkanTriangle\0");
        let layer_names = [ffi::CStr::from_bytes_with_nul_unchecked(
            b"VK_LAYER_KHRONOS_validation\0",
        )];
        let layers_names_raw: Vec<*const c_char> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let mut extension_names = ash_window::enumerate_required_extensions(
            window
                .display_handle()
                .expect("Failed to get display handle")
                .as_raw(),
        )
        .expect("Failed to enumerate extensions")
        .to_vec();
        extension_names.push(debug_utils::NAME.as_ptr());
        // extension_names.push("VK_EXT_swapchain_maintenance1".as_ptr() as *const c_char);

        let appinfo = vk::ApplicationInfo::default()
            .application_name(app_name)
            .application_version(0)
            .engine_name(app_name)
            .engine_version(0)
            .api_version(vk::make_api_version(0, 1, 0, 0));

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names)
            .flags(vk::InstanceCreateFlags::default());

        let instance = entry
            .create_instance(&instance_create_info, None)
            .expect("Instance creation error");
        instance
    }
}
