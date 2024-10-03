pub mod buffer;
pub mod command_buffer;
pub mod debug;
pub mod descriptor;
pub mod device;
pub mod fps_limiter;
pub mod framebuffer;
pub mod image;
pub mod instance;
pub mod pipeline;
pub mod surface;
pub mod swapchain;
pub mod sync;
pub mod sampler;

use std::ffi::{c_char, CStr};

use ash::vk::{self};

/// Helper function to convert [c_char; SIZE] to string
pub fn vk_to_string(raw_string_array: &[c_char]) -> String {
    // Implementation 1
    //    let end = '\0' as u8;
    //
    //    let mut content: Vec<u8> = vec![];
    //
    //    for ch in raw_string_array.iter() {
    //        let ch = (*ch) as u8;
    //
    //        if ch != end {
    //            content.push(ch);
    //        } else {
    //            break
    //        }
    //    }
    //
    //    String::from_utf8(content)
    //        .expect("Failed to convert vulkan raw string")

    // Implementation 2
    let raw_string = unsafe {
        let pointer = raw_string_array.as_ptr();
        CStr::from_ptr(pointer)
    };

    raw_string
        .to_str()
        .expect("Failed to convert vulkan raw string.")
        .to_owned()
}

pub fn vk_ptr_to_string(raw_string_array: *const i8) -> String {
    // Implementation 1
    //    let end = '\0' as u8;
    //
    //    let mut content: Vec<u8> = vec![];
    //
    //    for ch in raw_string_array.iter() {
    //        let ch = (*ch) as u8;
    //
    //        if ch != end {
    //            content.push(ch);
    //        } else {
    //            break
    //        }
    //    }
    //
    //    String::from_utf8(content)
    //        .expect("Failed to convert vulkan raw string")

    // Implementation 2
    let raw_string = unsafe {
        let pointer = raw_string_array;
        CStr::from_ptr(pointer)
    };

    raw_string
        .to_str()
        .expect("Failed to convert vulkan raw string.")
        .to_owned()
}

pub fn find_memory_type(
    type_filter: u32,
    required_properties: vk::MemoryPropertyFlags,
    mem_properties: &vk::PhysicalDeviceMemoryProperties,
) -> u32 {
    for (i, memory_type) in mem_properties.memory_types.iter().enumerate() {
        //if (type_filter & (1 << i)) > 0 && (memory_type.property_flags & required_properties) == required_properties {
        //    return i as u32
        // }

        // same implementation
        if (type_filter & (1 << i)) > 0 && memory_type.property_flags.contains(required_properties)
        {
            return i as u32;
        }
    }

    panic!("Failed to find suitable memory type!")
}

