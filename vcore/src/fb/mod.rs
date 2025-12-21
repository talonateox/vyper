pub mod terminal;

use limine::request::FramebufferRequest;

#[repr(C)]
pub struct Framebuffer {
    pub address: *mut u8,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
}

pub trait DrawTarget {
    fn draw_pixel(&mut self, x: usize, y: usize, color: u32);
}

impl DrawTarget for Framebuffer {
    fn draw_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }

        let offset = y * self.pitch + x * 4;
        unsafe {
            let ptr = self.address.add(offset) as *mut u32;
            ptr.write_volatile(color);
        }
    }
}

impl Framebuffer {
    pub fn from_limine(request: &FramebufferRequest) -> Self {
        if let Some(framebuffer_response) = request.get_response() {
            if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
                Self {
                    address: framebuffer.addr(),
                    width: framebuffer.width() as usize,
                    height: framebuffer.height() as usize,
                    pitch: framebuffer.pitch() as usize,
                }
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    }
}

unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}
