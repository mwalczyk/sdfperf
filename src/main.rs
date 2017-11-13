extern crate gl;
extern crate glutin;
extern crate cgmath;

mod program;
mod operator;
use operator::Graph;
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
    let mut mouse_position = (0.0, 0.0);
    let mut clicked_mouse_position = (0.0, 0.0);

    events_loop.run_forever(|event| {

        use glutin::{ControlFlow, Event, WindowEvent};

        match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => return glutin::ControlFlow::Break,
                glutin::WindowEvent::Resized(w, h) => gl_window.resize(w, h),
                glutin::WindowEvent::MouseMoved{device_id, position} => {
                    mouse_position = position;
                },
                glutin::WindowEvent::MouseInput{device_id, state, button} => {
                    if let glutin::ElementState::Pressed = state {
                        clicked_mouse_position = mouse_position;
                        println!("{:?}", clicked_mouse_position);
                        graph.add_operator(Vector2::new(clicked_mouse_position.0 as f32,
                                                                      clicked_mouse_position.1 as f32),
                                           Vector2::new(100.0, 50.0));
                    }
                },
                _ => (),
            },
            _ => ()
        }

        unsafe {
            gl::ClearColor(0.3, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            let elapsed = now.elapsed().unwrap();
            let ms = elapsed.as_secs() * 1000 + elapsed.subsec_nanos() as u64 / 1_000_000;
            graph.draw();
        }

        gl_window.swap_buffers().unwrap();

        ControlFlow::Continue
    });

}
