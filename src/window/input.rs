use std::{
    collections::{HashMap, HashSet},
    default,
    ops::ControlFlow,
    sync::Arc,
};

use anyhow::*;
use bitmaps::Bitmap;
use log::info;
use nalgebra::*;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::*,
    event_loop::{self, EventLoopWindowTarget},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyInput {
    Keyboard(VirtualKeyCode),
    Mouse(MouseButton),
}

macro_rules! impl_from_for_keyinput {
    ($ty:ty, $key:ident) => {
        impl From<$ty> for KeyInput {
            fn from(value: $ty) -> Self {
                KeyInput::$key(value)
            }
        }
    };
}
impl_from_for_keyinput!(MouseButton, Mouse);
impl_from_for_keyinput!(VirtualKeyCode, Keyboard);

// 其实并不是原始输入, 把一帧的多个事件合并了.
// pressed 事件无视重复输入
// delta 事件累加
// 懒了懒了, 就先这么做吧.
#[derive(Debug, Default)]
pub struct Input {
    mouse_dpos: Vector2<f32>,
    mouse_dwheel: Vector2<f32>,
    mouse_pos: Option<Point2<f32>>,

    pressed: HashSet<KeyInput>,
    just_pressed: HashSet<KeyInput>,
    just_released: HashSet<KeyInput>,
}

// 构建
impl Input {
    fn clear_every_frame(&mut self) {
        self.mouse_dpos = Vector2::<f32>::default();
        self.mouse_dwheel = Vector2::<f32>::default();
        self.just_pressed.clear();
        self.just_released.clear();
    }

    pub fn handle_event(&mut self, event: &Event<()>) {
        self.record_raw_event(event);
        match event {
            // Event::Suspended => todo!(),
            // Event::Resumed => todo!(),
            Event::MainEventsCleared => {}
            Event::RedrawRequested(wid) => {}
            Event::RedrawEventsCleared => {
                self.clear_every_frame();
            }
            Event::LoopDestroyed => {}
            _ => {}
        }
    }

    fn record_raw_event(&mut self, event: &Event<()>) {
        match event {
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::KeyboardInput {
                    device_id,
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(virtual_keycode),
                            ..
                        },
                    is_synthetic,
                } => {
                    if *is_synthetic {
                        println!("SYN {:?}", virtual_keycode);
                    }
                    match state {
                        ElementState::Pressed => self.record_pressed(*virtual_keycode),
                        ElementState::Released => self.record_released(*virtual_keycode),
                    }
                }
                WindowEvent::MouseInput {
                    device_id,
                    state,
                    button,
                    ..
                } => match state {
                    ElementState::Pressed => self.record_pressed(*button),
                    ElementState::Released => self.record_released(*button),
                },
                WindowEvent::CursorMoved {
                    device_id,
                    position,
                    ..
                } => {
                    self.mouse_pos = Some(Point2::new(position.x as f32, position.y as f32));
                }
                WindowEvent::MouseWheel {
                    device_id,
                    delta,
                    phase,
                    ..
                } => match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        self.mouse_dwheel =
                            Vector2::new(self.mouse_dwheel[0], self.mouse_dwheel[1])
                                + Vector2::new(*x, *y);
                    }
                    // 另一个不知道是干啥用的, windows上好像从来没收到过另一种类型的事件
                    _ => {}
                },
                WindowEvent::CursorEntered { device_id } => {}
                WindowEvent::CursorLeft { device_id } => {}
                WindowEvent::Ime(_) => {}
                _ => {}
            },
            Event::DeviceEvent { device_id, event } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    self.mouse_dpos = Vector2::new(self.mouse_dpos[0], self.mouse_dpos[1])
                        + Vector2::new(delta.0 as f32, delta.1 as f32);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn record_pressed(&mut self, key: impl Into<KeyInput>) {
        let key: KeyInput = key.into();
        if self.pressed.insert(key) {
            self.just_pressed.insert(key);
        }
    }

    fn record_released(&mut self, key: impl Into<KeyInput>) {
        let key: KeyInput = key.into();
        if self.pressed.remove(&key) {
            self.just_released.insert(key);
        }
    }
}

// 基本使用
impl Input {
    pub fn is_pressed(&self, key: impl Into<KeyInput>) -> bool {
        let key = key.into();
        self.pressed.contains(&key)
    }
    pub fn is_just_pressed(&self, key: impl Into<KeyInput>) -> bool {
        let key = key.into();
        self.just_pressed.contains(&key)
    }
    pub fn is_just_released(&self, key: impl Into<KeyInput>) -> bool {
        let key = key.into();
        self.just_released.contains(&key)
    }
    pub fn get_mouse_pos(&self) -> Option<Point2<f32>> {
        self.mouse_pos
    }
    pub fn get_mouse_delta_pos(&self) -> Vector2<f32> {
        self.mouse_dpos
    }
    pub fn get_mouse_delta_wheel(&self) -> Vector2<f32> {
        self.mouse_dwheel
    }
}

// 封装
#[derive(Debug, Default)]
pub struct InputAction {
    wasd_hold: Vec<KeyInput>,
    pos_move: Vector3<f32>,
}

impl InputAction {
    pub fn update(&mut self, input: &Input) {
        // wasd 排序
        let wasd: [KeyInput; 6] = [
            VirtualKeyCode::W.into(),
            VirtualKeyCode::A.into(),
            VirtualKeyCode::S.into(),
            VirtualKeyCode::D.into(),
            VirtualKeyCode::Space.into(),
            VirtualKeyCode::LShift.into(),
        ];
        for k in wasd.iter() {
            if input.is_just_pressed(*k) {
                self.wasd_hold.push(*k);
            }
            if input.is_just_released(*k) {
                let mut idx = None;
                for i in 0..wasd.len() {
                    if self.wasd_hold[i] == *k {
                        idx = Some(i);
                        break;
                    }
                }
                if let Some(idx) = idx {
                    self.wasd_hold.remove(idx);
                }
            }
        }

        // 更新move
        self.pos_move = Vector3::zeros();
        for k in self.wasd_hold.iter().rev() {
            match k {
                KeyInput::Keyboard(k) => match *k {
                    VirtualKeyCode::D => {
                        if self.pos_move.x == 0f32 {
                            self.pos_move.x = 1f32
                        }
                    }
                    VirtualKeyCode::A => {
                        if self.pos_move.x == 0f32 {
                            self.pos_move.x = -1f32
                        }
                    }
                    VirtualKeyCode::Space => {
                        if self.pos_move.y == 0f32 {
                            self.pos_move.y = 1f32
                        }
                    }
                    VirtualKeyCode::LShift => {
                        if self.pos_move.y == 0f32 {
                            self.pos_move.y = -1f32
                        }
                    }
                    VirtualKeyCode::S => {
                        if self.pos_move.z == 0f32 {
                            self.pos_move.z = 1f32
                        }
                    }
                    VirtualKeyCode::W => {
                        if self.pos_move.z == 0f32 {
                            self.pos_move.z = -1f32
                        }
                    }
                    _ => unreachable!(),
                },
                _ => {}
            }
        }
    }

    pub fn get_move(&self) -> Vector3<f32> {
        self.pos_move
    }
}
