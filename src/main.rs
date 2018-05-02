extern crate sdl2;
extern crate gl;

use sdl2::video;
use gl::types::*;

fn main() {
    let sdl = sdl2::init().unwrap();
    let sdl_vid = sdl.video().unwrap();

    let window = sdl_vid.window("demy", 800,600)
        .opengl()
        .borderless()
        .resizable()
        .build().unwrap();

    let gl_context = window.gl_create_context().unwrap();

    gl::load_with(|s| sdl_vid.gl_get_proc_address(s) as *const std::os::raw::c_void);
    unsafe { gl::ClearColor(1_f32, 0_f32, 0_f32, 0_f32); }

    let mut event_pump = sdl.event_pump().unwrap();
    let mut is_running = true;

    while is_running {

        for event in event_pump.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit {..}=> is_running = false,
                _ => {}
            };
        }

        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT); }

        window.gl_swap_window();

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
