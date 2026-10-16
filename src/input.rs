#![allow(unused)]
use glam::Vec2;
use std::collections::HashSet;
use std::hash::Hash;
use winit::{
    event::{MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

const SCROLL_SENS: f64 = 0.02;

pub struct ButtonState<T: Copy + Eq + Hash> {
    held: HashSet<T>,
    pressed: HashSet<T>,
    released: HashSet<T>,
}

impl<T: Copy + Eq + Hash> Default for ButtonState<T> {
    fn default() -> Self {
        Self {
            held: HashSet::new(),
            pressed: HashSet::new(),
            released: HashSet::new(),
        }
    }
}

impl<T: Copy + Eq + Hash> ButtonState<T> {
    pub fn press(&mut self, b: T) {
        if self.held.insert(b) {
            self.pressed.insert(b);
        }
    }
    pub fn release(&mut self, b: T) {
        if self.held.remove(&b) {
            self.released.insert(b);
        }
    }
    pub fn held(&self, b: T) -> bool {
        self.held.contains(&b)
    }
    pub fn just_pressed(&self, b: T) -> bool {
        self.pressed.contains(&b)
    }
    pub fn just_released(&self, b: T) -> bool {
        self.released.contains(&b)
    }
    pub fn clear_held(&mut self) {
        self.held.clear();
    }
    pub fn end_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }
}

#[derive(Default)]
pub struct Input {
    keys: ButtonState<KeyCode>,
    mouse: ButtonState<MouseButton>,
    mouse_delta: Vec2,
    scroll_delta: f32,
}

impl Input {
    pub fn on_key(&mut self, key: KeyCode, pressed: bool, repeat: bool) {
        if repeat {
            return;
        }
        if pressed {
            self.keys.press(key);
        } else {
            self.keys.release(key);
        }
    }
    pub fn on_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        if pressed {
            self.mouse.press(button);
        } else {
            self.mouse.release(button);
        }
    }
    pub fn on_mouse_motion(&mut self, dx: f64, dy: f64) {
        self.mouse_delta += Vec2::new(dx as f32, dy as f32);
    }
    pub fn on_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll_delta += match delta {
            MouseScrollDelta::LineDelta(_x, y) => *y,
            MouseScrollDelta::PixelDelta(pos) => (pos.y * SCROLL_SENS) as f32,
        };
    }
    pub fn clear_held(&mut self) {
        self.keys.clear_held();
        self.mouse.clear_held();
    }
    pub fn end_frame(&mut self) {
        self.keys.end_frame();
        self.mouse.end_frame();
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = 0.0;
    }

    pub fn key_held(&self, key: KeyCode) -> bool {
        self.keys.held(key)
    }
    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys.just_pressed(key)
    }
    pub fn key_just_released(&self, key: KeyCode) -> bool {
        self.keys.just_released(key)
    }
    pub fn mouse_held(&self, b: MouseButton) -> bool {
        self.mouse.held(b)
    }
    pub fn mouse_just_pressed(&self, b: MouseButton) -> bool {
        self.mouse.just_pressed(b)
    }
    pub fn mouse_just_released(&self, b: MouseButton) -> bool {
        self.mouse.just_released(b)
    }
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }
    pub fn mouse_scroll(&self) -> f32 {
        self.scroll_delta
    }
}

#[derive(Default)]
pub struct CursorState {
    pub want_grab: bool,
    pub grabbed: bool,
    pub visible: bool,
}
