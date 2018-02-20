#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![feature(vec_remove_item)]
extern crate cgmath;
extern crate gl;
extern crate glutin;
extern crate uuid;

mod bounding_rect;
mod color;
mod graph;
mod interaction;
mod network;
mod operator;
mod preview;
mod program;
mod renderer;
mod shader_builder;
mod shader_string;

use color::Color;
use interaction::MouseInfo;
use operator::{Op, OpType};
use network::Network;
use preview::Shading;
use program::Program;
use renderer::Renderer;
use shader_builder::ShaderBuilder;

use glutin::GlContext;
use cgmath::{Vector2, Zero};

fn clear() {
    unsafe {
        let clear = Color::from_hex(0x2B2B2B);
        gl::ClearColor(clear.r, clear.g, clear.b, clear.a);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }
}

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_dimensions(800, 600)
        .with_title("sdfperf");
    let context = glutin::ContextBuilder::new().with_multisampling(4);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();
    unsafe { gl_window.make_current() }.unwrap();
    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    // Main objects
    let mut network = Network::new();
    let mut renderer = Renderer::new();
    let mut builder = ShaderBuilder::new();

    // Constants
    const ZOOM_INCREMENT: f32 = 0.05;
    const OPERATOR_SIZE: Vector2<f32> = Vector2 { x: 100.0, y: 50.0 };
    let mut current_size = Vector2::new(800.0, 600.0);

    // Store interaction state
    let mut mouse = MouseInfo::new();

    loop {
        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::Closed => (),

                    glutin::WindowEvent::Resized(w, h) => {
                        current_size.x = w as f32;
                        current_size.y = h as f32;
                        gl_window.resize(w, h);

                        renderer.resize(&current_size);
                    }

                    glutin::WindowEvent::MouseMoved { position, .. } => {
                        // Store the current mouse position.
                        mouse.last = mouse.curr;
                        mouse.curr = Vector2::new(position.0 as f32, position.1 as f32);

                        // Zero center and zoom.
                        mouse.curr -= current_size * 0.5;
                        // TODO: mouse.curr *= mouse.scroll;

                        network.handle_interaction(&mouse);
                    }

                    glutin::WindowEvent::MouseWheel { delta, .. } => {
                        if let glutin::MouseScrollDelta::LineDelta(_, line_y) = delta {
                            if line_y == 1.0 {
                                mouse.scroll -= ZOOM_INCREMENT;
                            } else {
                                mouse.scroll += ZOOM_INCREMENT;
                            }

                            // TODO: renderer.zoom(mouse.scroll);
                            network.handle_interaction(&mouse);
                        }
                    }

                    glutin::WindowEvent::MouseInput { state, button, .. } => {
                        if let glutin::ElementState::Pressed = state {
                            // Store the current mouse position.
                            mouse.clicked = mouse.curr;

                            // Store mouse button presses.
                            match button {
                                glutin::MouseButton::Left => mouse.ldown = true,
                                glutin::MouseButton::Right => mouse.rdown = true,
                                glutin::MouseButton::Middle => mouse.mdown = true,
                                _ => (),
                            }
                            network.handle_interaction(&mouse);
                        } else {
                            mouse.ldown = false;
                            mouse.rdown = false;
                            mouse.mdown = false;
                        }
                    }

                    glutin::WindowEvent::KeyboardInput { input, .. } => {
                        if let glutin::ElementState::Pressed = input.state {
                            if let Some(key) = input.virtual_keycode {
                                if input.modifiers.shift && key != glutin::VirtualKeyCode::LShift {
                                    // If the `shift` modifier is down, add a new op.
                                    let op_type = match key {
                                        glutin::VirtualKeyCode::S => OpType::Sphere,
                                        glutin::VirtualKeyCode::B => OpType::Box,
                                        glutin::VirtualKeyCode::P => OpType::Plane,
                                        glutin::VirtualKeyCode::U => OpType::Union,
                                        glutin::VirtualKeyCode::I => OpType::Intersection,
                                        glutin::VirtualKeyCode::M => OpType::SmoothMinimum,
                                        glutin::VirtualKeyCode::R => OpType::Render,
                                        _ => OpType::Sphere,
                                    };
                                    network.add_op(
                                        op_type,
                                        mouse.curr - OPERATOR_SIZE * 0.5,
                                        OPERATOR_SIZE,
                                    );
                                } else {
                                    // Handle other key commands.
                                    match key {
                                        glutin::VirtualKeyCode::Delete => network.delete_selected(),
                                        glutin::VirtualKeyCode::H => {
                                            mouse.scroll = 1.0;
                                            network.preview.home();
                                        }
                                        glutin::VirtualKeyCode::P => network.toggle_preview(),
                                        glutin::VirtualKeyCode::Key1 => {
                                            network.preview.set_shading(Shading::Diffuse)
                                        }
                                        glutin::VirtualKeyCode::Key2 => {
                                            network.preview.set_shading(Shading::Constant)
                                        }
                                        glutin::VirtualKeyCode::Key3 => {
                                            network.preview.set_shading(Shading::Normals)
                                        }
                                        _ => (),
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        });

        clear();

        // Check to see if the graph needs to be rebuilt.
        if network.dirty() {
            if let Some(root) = network.root {
                let indices = network.graph.traverse(root);
                let program = builder.build_sources(&network, indices);
                network.preview.set_valid_program(program);
                network.clean();
            } else {
                network.preview.set_valid_program(None);
            }
        }

        // Draw the graph (ops, connections, preview window, etc.).
        network.draw(&renderer);

        gl_window.swap_buffers().unwrap();
    }
}
