use std::{default, ops::ControlFlow};

use anyhow::Result;
use bitmaps::Bitmap;
use log::info;
use nalgebra::Vector2;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, Event, KeyboardInput, ScanCode, VirtualKeyCode, WindowEvent},
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
        input.handle_event(&event, target, control);
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

#[derive(Debug, Default)]
pub struct Input {
    raw_mouse_dpos: Option<[f32; 2]>,
    raw_mouse_dwheel: Option<[f32; 2]>,
    has_focus: bool,
    frame: u32,
}

impl Input {
    pub fn clear_every_frame(&mut self) {
        *self = Self {
            has_focus: self.has_focus,
            frame: self.frame,
            ..Default::default()
        }
    }
    pub fn handle_event(
        &mut self,
        event: &Event<()>,
        target: &EventLoopWindowTarget<()>,
        control: &mut event_loop::ControlFlow,
    ) {
        match event {
            // Event::Suspended => todo!(),
            // Event::Resumed => todo!(),
            Event::WindowEvent { window_id, event } => match event {
                winit::event::WindowEvent::Resized(_) => {}
                winit::event::WindowEvent::CloseRequested => control.set_exit(),
                winit::event::WindowEvent::Focused(foc) => {
                    self.has_focus = *foc;
                }
                winit::event::WindowEvent::KeyboardInput {
                    device_id,
                    input,
                    is_synthetic,
                } => {
                    let KeyboardInput {
                        scancode,
                        state,
                        virtual_keycode,
                        ..
                    } = input;
                }
                winit::event::WindowEvent::Ime(_) => {}
                winit::event::WindowEvent::CursorMoved {
                    device_id,
                    position,
                    ..
                } => {}
                winit::event::WindowEvent::CursorEntered { device_id } => {}
                winit::event::WindowEvent::CursorLeft { device_id } => {}
                winit::event::WindowEvent::MouseWheel {
                    device_id,
                    delta,
                    phase,
                    ..
                } => {}
                winit::event::WindowEvent::MouseInput {
                    device_id,
                    state,
                    button,
                    ..
                } => {}
                _ => {}
            },
            Event::DeviceEvent { device_id, event } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    let mut mot: [f32; 2] = Default::default();
                    mot[0] = delta.0 as f32;
                    mot[1] = delta.1 as f32;
                    self.raw_mouse_dpos = Some(mot);
                }
                DeviceEvent::MouseWheel { delta } => match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        let mut whe: [f32; 2] = Default::default();
                        whe[0] = *x;
                        whe[1] = *y;
                        self.raw_mouse_dwheel = Some(whe);
                    }
                    winit::event::MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => {
                        dbg!(delta);
                    }
                },
                DeviceEvent::Motion { axis, value } => {}
                DeviceEvent::Button { button, state } => {}
                DeviceEvent::Text { codepoint } => {
                    dbg!(&device_id, &codepoint);
                }
                _ => {}
            },

            Event::MainEventsCleared => {}
            Event::RedrawRequested(wid) => {}
            Event::RedrawEventsCleared => {
                self.clear_every_frame();
            }
            Event::LoopDestroyed => {}
            _ => {}
        }
    }
}
