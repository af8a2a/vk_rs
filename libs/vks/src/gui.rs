use crate::camera::Camera;
use crate::{DEFAULT_FOV, DEFAULT_FPS_MOVE_SPEED, DEFAULT_Z_FAR, DEFAULT_Z_NEAR};
use egui::{ClippedPrimitive, Context, TexturesDelta, Ui, ViewportId, Widget};
use egui_winit::State as EguiWinit;
use math::cgmath::Deg;
use winit::event::WindowEvent;
use winit::window::Window as WinitWindow;

const SSAO_KERNEL_SIZES: [u32; 4] = [16, 32, 64, 128];
fn get_kernel_size_index(size: u32) -> usize {
    SSAO_KERNEL_SIZES
        .iter()
        .position(|&v| v == size)
        .unwrap_or_else(|| {
            panic!(
                "Illegal kernel size {:?}. Should be one of {:?}",
                size, SSAO_KERNEL_SIZES
            )
        })
}

pub struct RenderData {
    pub pixels_per_point: f32,
    pub textures_delta: TexturesDelta,
    pub clipped_primitives: Vec<ClippedPrimitive>,
}

pub struct Gui {
    egui: Context,
    egui_winit: EguiWinit,
    camera: Option<Camera>,
    state: State,
}

pub struct  RendererSetting {}

impl Gui {
    pub fn new(window: &WinitWindow, renderer_settings: Option<RendererSetting>) -> Self {
        let (egui, egui_winit) = init_egui(window);

        Self {
            egui,
            egui_winit,
            camera: None,
            state: State{},
        }
    }

    pub fn handle_event(&mut self, window: &WinitWindow, event: &WindowEvent) {
        let _ = self.egui_winit.on_window_event(window, event);
    }

    pub fn render(&mut self, window: &WinitWindow) -> RenderData {
        let raw_input = self.egui_winit.take_egui_input(window);

        let previous_state = self.state;

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = self.egui.run(raw_input, |ctx: &Context| {
            egui::Window::new("Menu ('H' to toggle)")
                .default_open(false)
                .show(ctx, |ui| {
                    build_renderer_settings_window(ui, &mut self.state);
                    ui.separator();
                    build_camera_details_window(ui, &mut self.state, self.camera);
                    ui.separator();
                    build_animation_player_window(ui, &mut self.state);
                });
        });

        // self.state.check_renderer_settings_changed(&previous_state);

        // self.state.hovered = self.egui.is_pointer_over_area();

        self.egui_winit
            .handle_platform_output(window, platform_output);

        let clipped_primitives = self.egui.tessellate(shapes, pixels_per_point);

        RenderData {
            pixels_per_point,
            textures_delta,
            clipped_primitives,
        }
    }

    pub fn set_camera(&mut self, camera: Option<Camera>) {
        self.camera = camera;
    }

    // pub fn get_selected_animation(&self) -> usize {
    //     self.state.selected_animation
    // }

    // pub fn is_infinite_animation_checked(&self) -> bool {
    //     self.state.infinite_animation
    // }

    // pub fn should_toggle_animation(&self) -> bool {
    //     self.state.toggle_animation
    // }

    // pub fn should_stop_animation(&self) -> bool {
    //     self.state.stop_animation
    // }

    // pub fn should_reset_animation(&self) -> bool {
    //     self.state.reset_animation
    // }

    // pub fn get_animation_speed(&self) -> f32 {
    //     self.state.animation_speed
    // }

    // pub fn camera_mode(&self) -> CameraMode {
    //     self.state.camera_mode
    // }

    // pub fn camera_fov(&self) -> Deg<f32> {
    //     Deg(self.state.camera_fov)
    // }

    // pub fn camera_z_near(&self) -> f32 {
    //     self.state.camera_z_near
    // }

    // pub fn camera_z_far(&self) -> f32 {
    //     self.state.camera_z_far
    // }

    // pub fn camera_move_speed(&self) -> f32 {
    //     self.state.camera_move_speed
    // }

    // pub fn should_reset_camera(&self) -> bool {
    //     self.state.reset_camera
    // }

    // pub fn get_new_renderer_settings(&self) -> Option<RendererSettings> {
    //     if self.state.renderer_settings_changed {
    //         Some(RendererSettings {
    //             hdr_enabled: self.state.hdr_enabled,
    //             emissive_intensity: self.state.emissive_intensity,
    //             ssao_enabled: self.state.ssao_enabled,
    //             ssao_kernel_size: SSAO_KERNEL_SIZES[self.state.ssao_kernel_size_index],
    //             ssao_radius: self.state.ssao_radius,
    //             ssao_strength: self.state.ssao_strength,
    //             bloom_strength: self.state.bloom_strength as f32 / 100f32,
    //         })
    //     } else {
    //         None
    //     }
    // }

    // pub fn is_hovered(&self) -> bool {
    //     self.state.hovered
    // }
}

fn init_egui(window: &WinitWindow) -> (Context, EguiWinit) {
    let egui = Context::default();
    let egui_winit = EguiWinit::new(egui.clone(), ViewportId::ROOT, &window, None, None, None);

    (egui, egui_winit)
}

fn build_animation_player_window(ui: &mut Ui, state: &mut State) {
    egui::CollapsingHeader::new("Animation player")
        .default_open(false)
        .show(ui, |ui| {});
}

fn build_camera_details_window(ui: &mut Ui, state: &mut State, camera: Option<Camera>) {
    // egui::CollapsingHeader::new("Camera")
    //     .default_open(false)
    //     .show(ui, |ui| {
    //         if let Some(camera) = camera {
    //             ui.horizontal(|ui| {
    //                 ui.radio_value(&mut state.camera_mode, CameraMode::Orbital, "Orbital");
    //                 ui.radio_value(&mut state.camera_mode, CameraMode::Fps, "Fps");
    //             });

    //             if let CameraMode::Fps = state.camera_mode {
    //                 ui.add(
    //                     egui::Slider::new(&mut state.camera_move_speed, 1.0..=10.0)
    //                         .text("Move speed"),
    //                 );
    //             }

    //             ui.add(egui::Slider::new(&mut state.camera_fov, 30.0..=90.0).text("FOV"));
    //             ui.add(
    //                 egui::Slider::new(&mut state.camera_z_near, 0.01..=10.0)
    //                     .text("Near plane")
    //                     .logarithmic(true)
    //                     .max_decimals(2),
    //             );
    //             ui.add(
    //                 egui::Slider::new(&mut state.camera_z_far, 10.0..=1000.0)
    //                     .text("Far plane")
    //                     .logarithmic(true),
    //             );

    //             let p = camera.position();
    //             let t = camera.target();
    //             ui.label(format!("Position: {:.3}, {:.3}, {:.3}", p.x, p.y, p.z));
    //             ui.label(format!("Target: {:.3}, {:.3}, {:.3}", t.x, t.y, t.z));

    //             state.reset_camera = ui.button("Reset").clicked();
    //             if state.reset_camera {
    //                 state.camera_fov = DEFAULT_FOV;
    //                 state.camera_z_near = DEFAULT_Z_NEAR;
    //                 state.camera_z_far = DEFAULT_Z_FAR;
    //                 state.camera_move_speed = DEFAULT_FPS_MOVE_SPEED;
    //             }
    //         }
    //     });
}

fn build_renderer_settings_window(ui: &mut Ui, state: &mut State) {
    egui::CollapsingHeader::new("Renderer settings")
        .default_open(true)
        .show(ui, |ui| {
            // {
            //     ui.heading("Settings");
            //     ui.separator();

            //     ui.add_enabled_ui(state.hdr_enabled.is_some(), |ui| {
            //         if let Some(hdr_enabled) = state.hdr_enabled.as_mut() {
            //             ui.checkbox(hdr_enabled, "Enable HDR");
            //         }
            //     });

            //     ui.add(
            //         egui::Slider::new(&mut state.emissive_intensity, 1.0..=200.0)
            //             .text("Emissive intensity")
            //             .integer(),
            //     );
            //     ui.add(
            //         egui::Slider::new(&mut state.bloom_strength, 0..=10)
            //             .text("Bloom strength")
            //             .integer(),
            //     );

            //     ui.checkbox(&mut state.ssao_enabled, "Enable SSAO");
            //     if state.ssao_enabled {
            //         egui::ComboBox::from_label("SSAO Kernel").show_index(
            //             ui,
            //             &mut state.ssao_kernel_size_index,
            //             SSAO_KERNEL_SIZES.len(),
            //             |i| SSAO_KERNEL_SIZES[i].to_string(),
            //         );
            //         ui.add(
            //             egui::Slider::new(&mut state.ssao_radius, 0.01..=1.0).text("SSAO Radius"),
            //         );
            //         ui.add(
            //             egui::Slider::new(&mut state.ssao_strength, 0.5..=5.0)
            //                 .text("SSAO Strength"),
            //         );
            //     }
            // }

            {
                ui.heading("Post Processing");
                ui.separator();

                // let tone_map_modes = ToneMapMode::all();
                // egui::ComboBox::from_label("Tone map mode").show_index(
                //     ui,
                //     &mut state.selected_tone_map_mode,
                //     tone_map_modes.len(),
                //     |i| format!("{:?}", tone_map_modes[i]),
                // );
            }

            {
                ui.heading("Debug");
                ui.separator();

                // let output_modes = OutputMode::all();
                // egui::ComboBox::from_label("Output mode").show_index(
                //     ui,
                //     &mut state.selected_output_mode,
                //     output_modes.len(),
                //     |i| format!("{:?}", output_modes[i]),
                // );
            }
        });
}


#[derive(Clone, Copy)]
struct State;

// #[derive(Clone, Copy)]
// struct State {
//     selected_animation: usize,
//     infinite_animation: bool,
//     reset_animation: bool,
//     toggle_animation: bool,
//     stop_animation: bool,
//     animation_speed: f32,

//     camera_mode: CameraMode,
//     camera_move_speed: f32,
//     camera_fov: f32,
//     camera_z_near: f32,
//     camera_z_far: f32,
//     reset_camera: bool,

//     hdr_enabled: Option<bool>,
//     selected_output_mode: usize,
//     selected_tone_map_mode: usize,
//     emissive_intensity: f32,
//     ssao_enabled: bool,
//     ssao_radius: f32,
//     ssao_strength: f32,
//     ssao_kernel_size_index: usize,
//     bloom_strength: u32,
//     renderer_settings_changed: bool,

//     hovered: bool,
// }

// impl State {
//     fn new(renderer_settings: RendererSettings) -> Self {
//         Self {
//             hdr_enabled: renderer_settings.hdr_enabled,
//             selected_output_mode: renderer_settings.output_mode as _,
//             selected_tone_map_mode: renderer_settings.tone_map_mode as _,
//             emissive_intensity: renderer_settings.emissive_intensity,
//             ssao_enabled: renderer_settings.ssao_enabled,
//             ssao_radius: renderer_settings.ssao_radius,
//             ssao_strength: renderer_settings.ssao_strength,
//             ssao_kernel_size_index: get_kernel_size_index(renderer_settings.ssao_kernel_size),
//             ..Default::default()
//         }
//     }

//     fn reset(&self) -> Self {
//         Self {
//             hdr_enabled: self.hdr_enabled,
//             selected_output_mode: self.selected_output_mode,
//             selected_tone_map_mode: self.selected_tone_map_mode,
//             emissive_intensity: self.emissive_intensity,
//             ssao_radius: self.ssao_radius,
//             ssao_strength: self.ssao_strength,
//             ssao_kernel_size_index: self.ssao_kernel_size_index,
//             ssao_enabled: self.ssao_enabled,
//             camera_mode: self.camera_mode,
//             ..Default::default()
//         }
//     }

//     fn check_renderer_settings_changed(&mut self, other: &Self) {
//         self.renderer_settings_changed = self.hdr_enabled != other.hdr_enabled
//             || self.selected_output_mode != other.selected_output_mode
//             || self.selected_tone_map_mode != other.selected_tone_map_mode
//             || self.emissive_intensity != other.emissive_intensity
//             || self.ssao_enabled != other.ssao_enabled
//             || self.ssao_radius != other.ssao_radius
//             || self.ssao_strength != other.ssao_strength
//             || self.ssao_kernel_size_index != other.ssao_kernel_size_index
//             || self.bloom_strength != other.bloom_strength;
//     }
// }

// impl Default for State {
//     fn default() -> Self {
//         Self {
//             selected_animation: 0,
//             infinite_animation: true,
//             reset_animation: false,
//             toggle_animation: false,
//             stop_animation: false,
//             animation_speed: 1.0,

//             camera_mode: CameraMode::Orbital,
//             camera_move_speed: DEFAULT_FPS_MOVE_SPEED,
//             camera_fov: DEFAULT_FOV,
//             camera_z_near: DEFAULT_Z_NEAR,
//             camera_z_far: DEFAULT_Z_FAR,
//             reset_camera: false,

//             hdr_enabled: None,
//             selected_output_mode: 0,
//             selected_tone_map_mode: 0,
//             emissive_intensity: 1.0,
//             ssao_enabled: true,
//             ssao_radius: 0.15,
//             ssao_strength: 1.0,
//             ssao_kernel_size_index: 1,
//             bloom_strength: (DEFAULT_BLOOM_STRENGTH * 100f32) as _,
//             renderer_settings_changed: false,

//             hovered: false,
//         }
//     }
// }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode {
    Orbital,
    Fps,
}
