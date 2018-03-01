#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![feature(vec_remove_item)]
extern crate cgmath;
extern crate gl;
extern crate glutin;
extern crate image;
extern crate uuid;

mod bounds;
mod color;
mod graph;
mod interaction;
mod network;
mod operator;
mod preview;
mod program;
mod renderer;
mod shader_builder;
mod texture;

// TODO:
// - Limit generators (i.e. sphere) to ONE output, since
//   the current graph traversal code doesn't work if the
//   same generator is connected to multiple other nodes.
//   the other option would be to properly handle this
//   during graph traversal so that the shader code for this
//   generator is duplicated. This would mean that transforms
//   should be their own family of operator as well.

use color::Color;
use interaction::{MouseInfo, Panel};
use operator::{Op, OpType};
use network::Network;
use preview::Shading;
use program::Program;
use renderer::Renderer;
use shader_builder::ShaderBuilder;

use glutin::GlContext;
use cgmath::{Vector2, Vector3, Zero};

fn clear() {
    unsafe {
        let clear = Color::from_hex(0x2B2B2B, 1.0);
        gl::ClearColor(clear.r, clear.g, clear.b, clear.a);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }
}

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_dimensions(800, 600)
        .with_title("signed-distance fields");
    let context = glutin::ContextBuilder::new().with_multisampling(4);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();
    unsafe { gl_window.make_current() }.unwrap();
    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    // Constants
    const ZOOM_INCREMENT: f32 = 0.05;
    const OPERATOR_SIZE: Vector2<f32> = Vector2 { x: 100.0, y: 50.0 };
    let mut current_size = Vector2::new(800.0, 600.0);

    // Main objects
    let mut network = Network::new(current_size);
    let mut builder = ShaderBuilder::new();

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
                                        glutin::VirtualKeyCode::T => OpType::Torus,
                                        glutin::VirtualKeyCode::U => OpType::Union,
                                        glutin::VirtualKeyCode::D => OpType::Subtraction,
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
                                            network.preview.set_shading(Shading::Depth)
                                        }
                                        glutin::VirtualKeyCode::Key2 => {
                                            network.preview.set_shading(Shading::Steps)
                                        }
                                        glutin::VirtualKeyCode::Key3 => {
                                            network.preview.set_shading(Shading::AmbientOcclusion)
                                        }
                                        glutin::VirtualKeyCode::Key4 => {
                                            network.preview.set_shading(Shading::Normals)
                                        }
                                        glutin::VirtualKeyCode::Equals => {
                                            network.scale_selected(0.05);
                                        }
                                        glutin::VirtualKeyCode::Minus => {
                                            network.scale_selected(-0.05);
                                        }
                                        glutin::VirtualKeyCode::Left => {
                                            network.translate_selected(&(Vector3::unit_x() * 0.05));
                                        }
                                        glutin::VirtualKeyCode::Right => {
                                            network
                                                .translate_selected(&(Vector3::unit_x() * -0.05));
                                        }
                                        glutin::VirtualKeyCode::Up => {
                                            network
                                                .translate_selected(&(Vector3::unit_y() * -0.05));
                                        }
                                        glutin::VirtualKeyCode::Down => {
                                            network.translate_selected(&(Vector3::unit_y() * 0.05));
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
        network.draw();

        gl_window.swap_buffers().unwrap();
    }
}
