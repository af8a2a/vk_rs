use nalgebra_glm::Vec3;

pub enum Direction {
    Forward,
    Backward,
    Left,
    Right,
}

#[derive(Default)]
pub struct Camera {
    pub position: nalgebra_glm::Vec3,
    pub front: nalgebra_glm::Vec3,
    pub up: nalgebra_glm::Vec3,
    pub right: nalgebra_glm::Vec3,
    pub world_up: nalgebra_glm::Vec3,

    fov: f32,
    yaw: f32,
    pitch: f32,

    movement_speed: f32,
    mouse_sensitivity: f32,
    zoom: f32,
}
impl Camera {
    pub fn new(
        position: nalgebra_glm::Vec3,
        front: nalgebra_glm::Vec3,
        up: nalgebra_glm::Vec3,
        fov: f32,
    ) -> Camera {
        Camera {
            position,
            front,
            up,
            right: nalgebra_glm::normalize(&nalgebra_glm::cross(&front, &up)),
            world_up: *Vec3::z_axis(),
            movement_speed: 100.0,
            fov,
            ..Default::default()
        }
    }

    pub fn get_view_matrix(&self) -> nalgebra_glm::Mat4 {
        let view=nalgebra_glm::look_at(&self.position, &(self.position + self.front), &self.up);
        // println!("view matrix: {:?}", view);
        view
    }

    pub fn get_model(&self) -> nalgebra_glm::Mat4 {
        let mat=nalgebra_glm::Mat4x4::identity();
        // println!("model matrix: {:?}", mat);

        mat
    }
    pub fn get_perspective_projection_matrix(&self) -> nalgebra_glm::Mat4 {
        nalgebra_glm::perspective(45.0_f32.to_radians(), self.fov, 0.1, 100.0)
    }

    pub fn get_orthogonal_projection_matrix(&self) -> nalgebra_glm::Mat4 {
        nalgebra_glm::ortho(0.0, 800.0, 0.0, 600.0, 0.1, 100.0)
    }

    pub fn process_move(&mut self, direction: Direction, delta_time: f32) {
        println!("update camera");
        let velocity = self.movement_speed * delta_time;
        match direction {
            Direction::Forward => self.position += self.front * velocity,
            Direction::Backward => self.position -= self.front * velocity,
            Direction::Left => self.position -= self.right * velocity,
            Direction::Right => self.position += self.right * velocity,
        }
    }
}
