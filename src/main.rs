extern crate sdl2;
extern crate gl as glu; // gl-unsafe

use glu::types::*;

#[allow(non_snake_case)]
pub mod gl {
    use super::glu;
    use super::glu::types::*;
    use std::os::raw;
    use std::ffi;
    use std::mem;

    pub fn ClearColor(r: f32, g: f32, b: f32, a: f32) {
        unsafe { glu::ClearColor(r,g,b,a); }
    }
    pub fn Clear(what: GLenum) {
        unsafe { glu::Clear(what); }
    }

    pub type DebugCallback = fn(GLenum, GLenum, GLuint, GLenum, GLsizei, &ffi::CStr);

    pub fn DebugMessageCallback(callback: DebugCallback) {
        unsafe { 
            // mem transmute here because casting to *const c_void somehow gives us an invalid
            // pointer
            glu::DebugMessageCallback(opengl_debug_callback, 
                                      mem::transmute::<_, *const raw::c_void>(callback));
        }
    }

    extern "system" fn opengl_debug_callback(
        source: GLenum, ttype: GLenum, id: GLuint, severity: GLenum, 
        length: GLsizei, message: *const GLchar, raw_data: *mut raw::c_void) {

        unsafe {
            let cb = mem::transmute::<_, DebugCallback>(raw_data);
            let s = ffi::CStr::from_ptr(message);
            (cb)(source, ttype, id, severity, length, s);
        }
    }
}


pub struct DataBuffer {
    id: GLuint,
}

impl DataBuffer {
    pub fn new() -> DataBuffer {
        unsafe {
            let mut id: GLuint = 0;
            glu::GenBuffers(1, &mut id as *mut GLuint);

            DataBuffer {
                id
            }
        }
    }

    pub fn bind(&self, bind_point: GLenum) {
        unsafe {
            glu::BindBuffer(bind_point, self.id);
        }
    }

    pub fn unbind(binding: GLenum) {
        unsafe { glu::BindBuffer(binding, 0); }
    }

    pub fn set_data<T>(&self, bind_point: GLenum, size: usize, data: &T, usage: GLenum) {
        unsafe {
            glu::BufferData(bind_point, size as isize, 
                            std::mem::transmute::<_, *const std::os::raw::c_void>(data), 
                            usage);
        }
    }
}

impl Drop for DataBuffer {
    fn drop(&mut self) {
        unsafe { glu::DeleteBuffers(1, &mut self.id as *mut GLuint) }
    }
}

pub struct VertexArray {
    id: GLuint,
}

impl VertexArray {
    pub fn new() -> VertexArray {
        unsafe {
            let mut id: GLuint = 0;
            glu::GenVertexArrays(1, &mut id as *mut GLuint);

            VertexArray {
                id
            }
        }
    }
    pub fn bind(&self) {
        unsafe { glu::BindVertexArray(self.id); }
    }

    pub fn unbind() {
        unsafe { glu::BindVertexArray(0); }
    }

    pub fn enable_attrib(&self, index: GLuint) {
        unsafe { glu::EnableVertexAttribArray(index); }
    }

    pub fn setup_attrib(&self, index: GLuint, size: GLint,
                        ttype: GLenum, normalized : bool , stride: GLsizei, ptr: isize) {
        unsafe { glu::VertexAttribPointer(index, size, ttype, normalized as GLboolean, stride, 
                                          std::mem::transmute::<_, *const std::os::raw::c_void>(ptr)); } 
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            glu::DeleteVertexArrays(1, &mut self.id as *mut GLuint);
        }
    }
}

fn ogl_debug(source: GLenum, ttype: GLenum, _id: GLuint, _severity: GLenum, 
             _length: GLsizei, message: &std::ffi::CStr) {
    println!("[GL] {} {} {:?}", source, ttype, message);
}

fn main() {
    let sdl = sdl2::init().unwrap();
    let sdl_vid = sdl.video().unwrap();

    {
        use sdl2::video::GLProfile;
        let gl_attr = sdl_vid.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_flags().debug().set();
        gl_attr.set_context_version(3, 3);
    }

    let window = sdl_vid.window("demy", 800,600)
        .opengl()
        .borderless()
        .resizable()
        .build().unwrap();

    let _gl_context = window.gl_create_context().unwrap();

    glu::load_with(|s| sdl_vid.gl_get_proc_address(s) as *const std::os::raw::c_void);

    gl::ClearColor(1_f32, 0_f32, 0_f32, 0_f32); 
    gl::DebugMessageCallback(ogl_debug);

    let vao = VertexArray::new();
    let vbo = DataBuffer::new();

    let quad: [f32; 8] = [
        -1_f32, -1_f32,
        -1_f32, 1_f32,
        1_f32, -1_f32,
        1_f32, 1_f32,
    ];
    
    vao.bind();
    vbo.bind(glu::ARRAY_BUFFER);
    vbo.set_data(glu::ARRAY_BUFFER, std::mem::size_of::<f32>() * 8, &quad, glu::STATIC_DRAW);
    vao.enable_attrib(0);
    vao.setup_attrib(0, 2, glu::FLOAT, false, 0, 0);

    VertexArray::unbind();
    DataBuffer::unbind(glu::ARRAY_BUFFER);

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

        gl::Clear(glu::COLOR_BUFFER_BIT);

        window.gl_swap_window();

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
