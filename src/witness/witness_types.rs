#![allow(unused)]

pub struct Puzzle;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
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

#[derive(Clone, Copy)]
pub struct Entity {
    unk1: [u8; 0x8],
    name: usize,
    unk2: [u8; 0x14],
    pub position: Vec3,
}
