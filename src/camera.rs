use nalgebra_glm::{quat_rotate_vec3, Vec3};

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

    aspect: f32,
    fov: f32,

    movement_speed: f32,
    mouse_sensitivity: f32,
}
impl Camera {
    pub fn new(
        position: nalgebra_glm::Vec3,
        front: nalgebra_glm::Vec3,
        up: nalgebra_glm::Vec3,
        aspect: f32,
        fov: f32,
    ) -> Camera {
        Camera {
            position,
            front,
            up,
            right: nalgebra_glm::normalize(&nalgebra_glm::cross(&front, &up)),
            world_up: *Vec3::z_axis(),
            movement_speed: 1.0,
            aspect,
            fov,
            mouse_sensitivity: 0.05,
            ..Default::default()
        }
    }

    pub fn get_view_matrix(&self) -> nalgebra_glm::Mat4 {
        let view = nalgebra_glm::look_at(&self.position, &(self.position + self.front), &self.up);
        view
    }

    pub fn get_model(&self) -> nalgebra_glm::Mat4 {
        let mat = nalgebra_glm::Mat4x4::identity();
        // println!("model matrix: {:?}", mat);

        mat
    }
    pub fn get_perspective_projection_matrix(&self) -> nalgebra_glm::Mat4 {
        let mut proj=nalgebra_glm::perspective(self.aspect, self.fov, 0.1, 100.0);
        *proj.index_mut((1, 1)) *= -1.0;
        proj
    }

    pub fn get_orthogonal_projection_matrix(&self) -> nalgebra_glm::Mat4 {
        nalgebra_glm::ortho(0.0, 800.0, 0.0, 600.0, 0.1, 100.0)
    }

    pub fn process_move(&mut self, direction: Direction, delta_time: f32) {
        let velocity = self.movement_speed * delta_time;
        match direction {
            Direction::Forward => self.position += self.front * velocity,
            Direction::Backward => self.position -= self.front * velocity,
            Direction::Left => self.position -= self.right * velocity,
            Direction::Right => self.position += self.right * velocity,
        }
    }
    pub fn process_mouse(&mut self, xoffset: f32, yoffset: f32) {
        let dx = -(xoffset * self.mouse_sensitivity).to_radians();
        let dy = -(yoffset * self.mouse_sensitivity).to_radians();

        self.translate_quaternion(dx, dy);
    }

    fn translate_quaternion(&mut self, dx: f32, dy: f32) {
        let q_yaw = nalgebra_glm::quat_angle_axis(dx, &Vec3::z_axis());
        let q_pitch = nalgebra_glm::quat_angle_axis(dy, &(self.right));
        self.front = quat_rotate_vec3(&(q_yaw * q_pitch), &self.front);
        self.right = quat_rotate_vec3(&(q_yaw * q_pitch), &self.right);
        self.up = -self.front.cross(&self.right);
    }

}
