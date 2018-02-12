#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
extern crate gl;
extern crate glutin;
extern crate cgmath;
extern crate uuid;

mod program;
mod operator;
mod graph;
mod bounding_rect;
mod color;
mod renderer;
mod shader_builder;
mod shader_string;

use color::Color;
use graph::Graph;
use operator::{Op, OpType, MouseInfo};
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
    let window = glutin::WindowBuilder::new().with_dimensions(800, 600).with_title("sdfperf");
    let context = glutin::ContextBuilder::new().with_multisampling(4);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe { gl_window.make_current() }.unwrap();
    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    // Main objects
    let mut graph = Graph::new();
    let mut renderer = Renderer::new();
    let mut shader_builder = ShaderBuilder::new();

    // Constants
    const ZOOM_INCREMENT: f32 = 0.05;
    const OPERATOR_SIZE: Vector2<f32> = Vector2 { x: 100.0, y: 50.0 };
    let mut current_zoom = 1.0;
    let mut current_size = Vector2::new(800.0, 600.0);

    // Store interaction state
    let mut mouse_info = MouseInfo {
        curr: Vector2::zero(),
        last: Vector2::zero(),
        clicked: Vector2::zero(),
        down: false
    };

    loop {
        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::Closed => (),

                    glutin::WindowEvent::Resized(w, h) => {
                        current_size.x = w as f32;
                        current_size.y = h as f32;
                        gl_window.resize(w, h);
                    },

                    glutin::WindowEvent::MouseMoved { position, .. } => {
                        // Store the current mouse position.
                        mouse_info.last = mouse_info.curr;
                        mouse_info.curr = Vector2::new(position.0 as f32, position.1 as f32);

                        // Zero center and zoom.
                        mouse_info.curr -= current_size * 0.5;
                        mouse_info.curr *= current_zoom;

                        graph.handle_interaction(&mouse_info);
                    },

                    glutin::WindowEvent::MouseWheel {delta, .. } => {
                        if let glutin::MouseScrollDelta::LineDelta(_, line_y) = delta {
                            if line_y == 1.0 {
                                current_zoom -= ZOOM_INCREMENT;
                            }
                            else {
                                current_zoom += ZOOM_INCREMENT;
                            }
                            renderer.zoom(current_zoom);
                        }
                    },

                    glutin::WindowEvent::MouseInput { state, .. } => {

                        if let glutin::ElementState::Pressed = state {
                            // Store the current mouse position.
                            mouse_info.clicked = mouse_info.curr;
                            mouse_info.down = true;

                            graph.handle_interaction(&mouse_info);
                        }
                        else {
                            mouse_info.down = false;
                        }
                    },

                    glutin::WindowEvent::KeyboardInput { input, .. } => {
                        if let glutin::ElementState::Pressed = input.state {

                            if let Some(key) = input.virtual_keycode {
                                if input.modifiers.shift {
                                    let op_type = match key {
                                        glutin::VirtualKeyCode::S => OpType::Sphere,
                                        glutin::VirtualKeyCode::B => OpType::Box,
                                        glutin::VirtualKeyCode::P => OpType::Plane,
                                        glutin::VirtualKeyCode::U => OpType::Union,
                                        glutin::VirtualKeyCode::I => OpType::Intersection,
                                        glutin::VirtualKeyCode::M => OpType::SmoothMinimum,
                                        glutin::VirtualKeyCode::R => OpType::Render,
                                        _ => OpType::Sphere
                                    };
                                    graph.add_op(op_type,mouse_info.curr - OPERATOR_SIZE * 0.5, OPERATOR_SIZE);
                                }
                                if let Some(glutin::VirtualKeyCode::Delete) = input.virtual_keycode {
                                    graph.delete_selected();
                                }
                            }
                        }
                    }
                    _ => (),
                },
                _ => ()
            }
        });

        clear();

        // Check to see if the graph needs to be rebuilt.
        if graph.dirty() {
            let program = shader_builder.traverse_postorder(&graph);
            renderer.set_preview_program(program);

            graph.clean();
        }

        // Draw the graph (ops, connections, etc.).
        graph.draw(&renderer);

        renderer.draw_preview();

        gl_window.swap_buffers().unwrap();
    }
}

