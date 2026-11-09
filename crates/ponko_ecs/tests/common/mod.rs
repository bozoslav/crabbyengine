//! Components shared by the integration tests.
#![allow(dead_code)]

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Pos(pub f32, pub f32);
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Vel(pub f32, pub f32);
#[derive(Debug, PartialEq)]
pub struct Tag;
pub struct Frozen;
