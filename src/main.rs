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
mod shader_builder;
mod shader_string;

use graph::Graph;
use program::Program;
use shader_builder::ShaderBuilder;

use std::time::{Duration, SystemTime};
use glutin::GlContext;
use cgmath::{Vector2, Zero};

fn clear() {
    unsafe {
        gl::ClearColor(0.15, 0.15, 0.15, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }
}

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_dimensions(800, 600);
    let context = glutin::ContextBuilder::new();
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe { gl_window.make_current() }.unwrap();
    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    // Main objects
    let mut graph = Graph::new();
    let mut shader_builder = ShaderBuilder::new();

    // Constants
    const ZOOM_INCREMENT: f32 = 0.05;
    const OPERATOR_SIZE: Vector2<f32> = Vector2 { x: 100.0, y: 50.0 };

    // Store system time
    let now = SystemTime::now();

    // Store interaction state
    let mut mouse_down = false;
    let mut mouse_position = Vector2::zero();
    let mut last_clicked = Vector2::zero();
    let mut current_zoom = 1.0;
    let mut current_size = Vector2::new(800.0, 600.0);

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
                        mouse_position = Vector2::new(position.0 as f32, position.1 as f32);

                        // Zero center and zoom.
                        mouse_position -= current_size * 0.5;
                        mouse_position *= current_zoom;

                        graph.handle_interaction(mouse_position, mouse_down);
                    },

                    glutin::WindowEvent::MouseWheel {delta, .. } => {
                        if let glutin::MouseScrollDelta::LineDelta(_, line_y) = delta {
                            if line_y == 1.0 {
                                current_zoom -= ZOOM_INCREMENT;
                            }
                            else {
                                current_zoom += ZOOM_INCREMENT;
                            }
                            graph.set_network_zoom(current_zoom);
                        }
                    },

                    glutin::WindowEvent::MouseInput { state, .. } => {

                        if let glutin::ElementState::Pressed = state {
                            // Store the current mouse position.
                            last_clicked = mouse_position;
                            mouse_down = true;

                            graph.handle_interaction(last_clicked, mouse_down);
                        }
                        else {
                            mouse_down = false;
                        }
                    },

                    glutin::WindowEvent::KeyboardInput { input, .. } => {
                        if let glutin::ElementState::Pressed = input.state {
                            if let Some(key) = input.virtual_keycode {
                                match key {
                                    glutin::VirtualKeyCode::A => graph.add_op(mouse_position - OPERATOR_SIZE * 0.5, OPERATOR_SIZE),
                                    _ => ()
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

        // Calculate the amount of time that has elapsed (in ms) since
        // application launch.
        let elapsed = now.elapsed().unwrap();
        let elapsed_ms = elapsed.as_secs() * 1000 + elapsed.subsec_nanos() as u64 / 1_000_000;

        // Check to see if the graph needs to be rebuilt.
        if graph.dirty() {
            let program = shader_builder.traverse(&graph);
            graph.set_program(program);
        }

        // Draw the graph (ops, connections, etc.).
        graph.draw();

        gl_window.swap_buffers().unwrap();
    }
}

