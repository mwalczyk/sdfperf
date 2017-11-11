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

    let now = SystemTime::now();

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new();
    let context = glutin::ContextBuilder::new();
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    // It is essential to make the context current before calling `gl::load_with`.
    unsafe { gl_window.make_current() }.unwrap();

    // Load the OpenGL function pointers
    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    let mut graph = Graph::new();
    graph.add_operator(Vector2::new(10.0, 10.0), Vector2::new(10.0, 10.0));

    events_loop.run_forever(|event| {
        use glutin::{ControlFlow, Event, WindowEvent};

        match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => return glutin::ControlFlow::Break,
                glutin::WindowEvent::Resized(w, h) => gl_window.resize(w, h),
                glutin::WindowEvent::MouseInput{device_id, state, button} => {
                    match state {
                        
                    }
                }
                _ => (),
            },
            _ => ()
        }
        unsafe {
            gl::ClearColor(0.3, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            //let elapsed = now.elapsed().unwrap();
            //let ms = elapsed.as_secs() * 1000 +
            //    elapsed.subsec_nanos() as u64 / 1_000_000;

            //prog.uniform1f("u_time", ms as f64);
            graph.draw();
        }

        gl_window.swap_buffers().unwrap();

        ControlFlow::Continue
    });

}
