use std::{collections::HashSet, default, ops::ControlFlow};

use anyhow::Result;
use bitmaps::Bitmap;
use log::info;
use nalgebra::{Point2, Vector2};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::*,
    event_loop::{self, EventLoopWindowTarget},
    window::WindowBuilder,
};

use crate::render::RenderState;

pub async fn run() -> Result<()> {
    let event_loop = event_loop::EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop)?;
    let mut first_frame = true;
    let mut input = Input::default();
    let mut render = RenderState::init(&window).await?;
    event_loop.run(move |event, target, mut control| {
        match &event {
            Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => control.set_exit(),
                _ => {}
            },
            _ => {}
        };

        input.handle_event(&event);

        if first_frame {
            match event {
                Event::RedrawEventsCleared => {
                    first_frame = false;
                }
                _ => {}
            }
        } else {
            match event {
                Event::WindowEvent { event, window_id } => match event {
                    WindowEvent::Resized(size) => {
                        if size.width > 0 && size.height > 0 {
                            render.resize(size.width, size.height);
                        }
                    }
                    _ => {}
                },
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                Event::RedrawRequested(wid) => {
                    render.redraw();
                }
                Event::RedrawEventsCleared => {}
                _ => {}
            }
        }
    })
}

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
    mouse_dpos: [f32; 2],
    mouse_dwheel: [f32; 2],
    mouse_pos: Option<[f32; 2]>,

    pressed: HashSet<KeyInput>,
    just_pressed: HashSet<KeyInput>,
    just_released: HashSet<KeyInput>,
}

impl Input {
    fn clear_every_frame(&mut self) {
        self.mouse_dpos = [0.0,0.0];
        self.mouse_dwheel = [0.0,0.0];
        self.just_pressed.clear();
        self.just_released.clear();
    }

    pub fn handle_event(&mut self, event: &Event<()>) {
        self.record_raw_event(event);
        match event {
            // Event::Suspended => todo!(),
            // Event::Resumed => todo!(),
            Event::MainEventsCleared => {
            }
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
                } => match state {
                    ElementState::Pressed => self.record_pressed(*virtual_keycode),
                    ElementState::Released => self.record_released(*virtual_keycode),
                },
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
                    self.mouse_pos = Some([position.x as f32, position.y as f32]);
                }
                WindowEvent::MouseWheel {
                    device_id,
                    delta,
                    phase,
                    ..
                } => match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        self.mouse_dwheel = [self.mouse_dwheel[0] + *x, self.mouse_dwheel[1] + *y];
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
                    self.mouse_dpos = [
                        self.mouse_dpos[0] + delta.0 as f32,
                        self.mouse_dpos[1] + delta.1 as f32,
                    ];
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
