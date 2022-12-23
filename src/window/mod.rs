use std::{collections::HashSet, default, ops::ControlFlow, time::*};

use anyhow::{Result, Ok};
use bitmaps::Bitmap;
use log::info;
use nalgebra::{Point2, Rotation3, Vector2};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::*,
    event_loop::{self, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};

use crate::{render::{camera::Camera, RenderState}, scene::Scene};

pub mod input;
use input::*;

pub async fn run() -> Result<()> {
    let event_loop = event_loop::EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop)?;
    let mut first_frame = true;
    let mut last_time: Option<Instant> = None;
    let mut time: Option<Instant> = None;
    let mut input = Input::default();
    let mut input_action = InputAction::default();
    let mut camera = create_camera(&window);
    let mut render = RenderState::init(&window, &camera).await?;
    let mut scene = Scene::init(&mut render)?;
    let size = window.inner_size();
    render.resize(&mut camera, size.width, size.height)?;

    event_loop.run(move |event, target, mut control| {
        match &event {
            Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => control.set_exit(),
                _ => {}
            },
            _ => {}
        };

        if first_frame {
            match event {
                Event::MainEventsCleared => {
                    // 更新时间
                    time = Some(Instant::now());
                }
                Event::RedrawEventsCleared => {
                    first_frame = false;
                }
                _ => {}
            }
        } else {
            input.handle_event(&event);
            match event {
                Event::WindowEvent { event, window_id } => match event {
                    WindowEvent::Resized(size) => {
                        if size.width > 0 && size.height > 0 {
                            render.resize(&mut camera, size.width, size.height);
                        }
                    }
                    _ => {}
                },
                Event::MainEventsCleared => {
                    // 更新时间
                    last_time = time;
                    time = Some(Instant::now());
                    let dt = time
                        .expect("time未设置")
                        .duration_since(last_time.expect("last_time未设置"))
                        .as_secs_f32();

                    // 更新逻辑输入动作
                    input_action.update(&input);

                    // 更新相机
                    {
                        let rot_speed = 0.2f32;
                        let move_speed = 10f32;
                        // 位置
                        let dpos = dt * move_speed * input_action.get_move();
                        let dpos = camera
                            .view_matrix
                            .try_inverse()
                            .expect("相机view矩阵不可逆")
                            .transform_vector(&dpos);
                        camera.position += dpos;
                        // 旋转
                        let dmouse = input.get_mouse_delta_pos();
                        let mut right = camera.direction.cross(&camera.up);
                        let rotx = {
                            let aa = dt * rot_speed * -dmouse.y * right;
                            Rotation3::new(aa)
                        };
                        let roty = {
                            let aa = dt * rot_speed * -dmouse.x * camera.up;
                            Rotation3::new(aa)
                        };
                        camera.direction = roty * (rotx * camera.direction);
                        // 重新计算矩阵
                        camera.calculate();
                    }

                    window.request_redraw();
                }
                Event::RedrawRequested(wid) => {
                    render.redraw(&camera, &mut scene);
                }
                Event::RedrawEventsCleared => {}
                _ => {}
            }
        }
    })
}

fn create_camera(window: &Window) -> Camera {
    let mut camera = Camera::default();
    let size = window.inner_size();
    camera.aspect = size.width as f32 / size.height as f32;
    camera.calculate();
    camera
}
