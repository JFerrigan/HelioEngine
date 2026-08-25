use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalized(self) -> Self {
        let length = self.length();
        if length <= f32::EPSILON {
            Self::ZERO
        } else {
            self / length
        }
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalized(),
        }
    }

    pub fn point_at(self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub yaw_radians: f32,
    pub pitch_radians: f32,
    pub roll_radians: f32,
    pub fov_y_radians: f32,
    pub max_distance: f32,
}

impl Camera {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw_radians: 0.0,
            pitch_radians: 0.0,
            roll_radians: 0.0,
            fov_y_radians: 60.0_f32.to_radians(),
            max_distance: 96.0,
        }
    }

    pub fn looking_at(mut self, yaw_radians: f32, pitch_radians: f32) -> Self {
        self.yaw_radians = yaw_radians;
        self.pitch_radians = pitch_radians;
        self
    }

    pub fn with_fov_y(mut self, fov_y_radians: f32) -> Self {
        self.fov_y_radians = fov_y_radians;
        self
    }

    pub fn with_max_distance(mut self, max_distance: f32) -> Self {
        self.max_distance = max_distance;
        self
    }

    pub fn with_roll(mut self, roll_radians: f32) -> Self {
        self.roll_radians = roll_radians;
        self
    }

    pub fn ray_for_cell(self, x: usize, y: usize, width: usize, height: usize) -> Ray {
        let width = width.max(1) as f32;
        let height = height.max(1) as f32;
        let aspect = width / height;
        let tan_half_fov = (self.fov_y_radians * 0.5).tan();
        let sensor_x = ((((x as f32 + 0.5) / width) * 2.0) - 1.0) * aspect * tan_half_fov;
        let sensor_y = (1.0 - (((y as f32 + 0.5) / height) * 2.0)) * tan_half_fov;

        let forward = self.forward();
        let right = self.right();
        let up = self.up();
        Ray::new(self.position, forward + right * sensor_x + up * sensor_y)
    }

    pub fn forward(self) -> Vec3 {
        let yaw_sin = self.yaw_radians.sin();
        let yaw_cos = self.yaw_radians.cos();
        let pitch_sin = self.pitch_radians.sin();
        let pitch_cos = self.pitch_radians.cos();
        Vec3::new(yaw_sin * pitch_cos, pitch_sin, yaw_cos * pitch_cos).normalized()
    }

    pub fn right(self) -> Vec3 {
        let forward = self.forward();
        let base_right =
            Vec3::new(self.yaw_radians.cos(), 0.0, -self.yaw_radians.sin()).normalized();
        let base_up = forward.cross(base_right).normalized();
        let roll_sin = self.roll_radians.sin();
        let roll_cos = self.roll_radians.cos();
        (base_right * roll_cos + base_up * roll_sin).normalized()
    }

    pub fn up(self) -> Vec3 {
        self.forward().cross(self.right()).normalized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_ray_faces_camera_forward() {
        let camera = Camera::new(Vec3::ZERO);
        let ray = camera.ray_for_cell(1, 1, 3, 3);
        assert!(ray.direction.x.abs() < 0.001);
        assert!(ray.direction.y.abs() < 0.001);
        assert!(ray.direction.z > 0.99);
    }

    #[test]
    fn roll_rotates_camera_basis() {
        let camera = Camera::new(Vec3::ZERO).with_roll(std::f32::consts::FRAC_PI_2);

        assert!(camera.right().y > 0.99);
        assert!(camera.up().x < -0.99);
    }
}
