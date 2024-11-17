use std::sync::Arc;

use ash::{vk, Device};
use egui::TextureId;

use crate::Context;

pub const MAX_FRAMES_IN_FLIGHT: u32 = 2;

pub struct InFlightFrames {
    context: Arc<Context>,
    sync_objects: Vec<SyncObjects>,
    pub gui_textures_to_free: Vec<TextureId>,
    current_frame: usize,
}

impl InFlightFrames {
    pub fn new(context: Arc<Context>, sync_objects: Vec<SyncObjects>) -> Self {
        Self {
            context,
            sync_objects,
            gui_textures_to_free: Vec::new(),
            current_frame: 0,
        }
    }
}

impl Drop for InFlightFrames {
    fn drop(&mut self) {
        self.sync_objects
            .iter()
            .for_each(|o| o.destroy(self.context.device()));
    }
}

impl Iterator for InFlightFrames {
    type Item = SyncObjects;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.sync_objects[self.current_frame];

        self.current_frame = (self.current_frame + 1) % self.sync_objects.len();

        Some(next)
    }
}


#[derive(Clone, Copy)]
pub struct SyncObjects {
   pub image_available_semaphore: vk::Semaphore,
   pub render_finished_semaphore: vk::Semaphore,
   pub fence: vk::Fence,
}

impl SyncObjects {
    fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_semaphore(self.image_available_semaphore, None);
            device.destroy_semaphore(self.render_finished_semaphore, None);
            device.destroy_fence(self.fence, None);
        }
    }
}
