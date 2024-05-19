#![allow(unused)]

use super::back_to_enum;

pub struct Puzzle;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Vec3 {
    pub fn len(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn normalized(&self) -> Self {
        let len = self.len();
        Self {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Color<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

impl Color<f32> {
    pub const RED: Color<f32> = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Color<f32> = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Color<f32> = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const WHITE: Color<f32> = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Color<f32> = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

    pub const PINK: Color<f32> = Color { r: 1.0, g: 0.4, b: 0.4, a: 1.0 };
} 

#[derive(Clone, Copy)]
pub struct Entity {
    unk1: [u8; 0x8],
    name: usize,
    unk2: [u8; 0x14],
    pub position: Vec3,
}

back_to_enum!{
    #[repr(C)]
    pub enum InteractionStatus {
        FocusMode = 0x0,
        SolvingPanel = 0x1,
        Walking = 0x2,
        Cinematic = 0x3,
    }
}
