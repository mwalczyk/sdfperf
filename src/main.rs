#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unreachable_code)]

extern crate gl;
extern crate glutin;
extern crate cgmath;

mod program;
mod operator;
mod graph;
mod bounding_rect;

use operator::Operator;
use graph::Graph;
use program::Program;

use gl::types::*;
use std::mem;
use std::ptr;
use std::str;
use std::ffi::CString;
use std::time::{Duration, SystemTime};
use cgmath::Vector2;

fn main() {
    use glutin::GlContext;

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_dimensions(800, 600);
    let context = glutin::ContextBuilder::new();
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    // It is essential to make the context current before calling `gl::load_with`
    unsafe { gl_window.make_current() }.unwrap();

    // Load the OpenGL function pointers
    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    let mut graph = Graph::new();
    let now = SystemTime::now();
    let mut mouse_down = false;
    let mut mouse_position = Vector2::new(0.0, 0.0);
    let mut clicked_mouse_position = Vector2::new(0.0, 0.0);
    let mut current_zoom = 1.0;

    loop {
        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::Closed => (),
                    glutin::WindowEvent::Resized(w, h) => gl_window.resize(w, h),
                    glutin::WindowEvent::MouseMoved { position, .. } => {
                        // Store mouse position
                        mouse_position = Vector2::new(position.0 as f32, position.1 as f32);
                        if let Some(window_size) = gl_window.get_inner_size_pixels() {
                            // Zero center
                            mouse_position.x = mouse_position.x - (window_size.0 / 2) as f32;
                            mouse_position.y = mouse_position.y - (window_size.1 / 2) as f32;

                            // Zoom
                            mouse_position.x *= current_zoom;
                            mouse_position.y *= current_zoom;
                        }

                        graph.handle_interaction(mouse_position, mouse_down);
                    },
                    glutin::WindowEvent::MouseWheel {delta, .. } => {
                        if let glutin::MouseScrollDelta::LineDelta(_, line_y) = delta {
                            if line_y == 1.0 {
                                current_zoom -= 0.05;
                            }
                            else {
                                current_zoom += 0.05;
                            }
                            graph.set_network_zoom(current_zoom);
                        }
                    },
                    glutin::WindowEvent::MouseInput { state, .. } => {
                        // Check if any operator was selected and store the click position
                        if let glutin::ElementState::Pressed = state {
                            clicked_mouse_position = mouse_position;
                            mouse_down = true;

                            graph.handle_interaction(clicked_mouse_position, mouse_down);
                        }
                        else {
                            mouse_down = false;
                        }
                    },
                    glutin::WindowEvent::KeyboardInput { input, .. } => {
                        // `a` adds a new operator to the graph
                        if let glutin::ElementState::Pressed = input.state {
                            if input.scancode == 30 {
                                const OPERATOR_SIZE: (f32, f32) = (100.0, 50.0);

                                graph.add_operator(Vector2::new(mouse_position.x - OPERATOR_SIZE.0 / 2.0,
                                                                            mouse_position.y - OPERATOR_SIZE.1 / 2.0),
                                                   Vector2::new(OPERATOR_SIZE.0, OPERATOR_SIZE.1));

                                graph.handle_interaction(mouse_position, mouse_down);
                            }
                        }
                    }
                    _ => (),
                },
                _ => ()
            }
        });

        unsafe {
            gl::ClearColor(0.15, 0.15, 0.15, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            let elapsed = now.elapsed().unwrap();
            let ms = elapsed.as_secs() * 1000 + elapsed.subsec_nanos() as u64 / 1_000_000;

            graph.draw();
        }

        gl_window.swap_buffers().unwrap();
    }
}
