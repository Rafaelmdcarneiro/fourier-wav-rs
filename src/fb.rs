use framebuffer::Framebuffer;

pub struct FbPainter {
    fb: Framebuffer,
    w: usize,
    h: usize,
    line_length: usize,
    bytespp: usize,
    frame: Vec<u8>,
}

impl FbPainter {

    pub fn new() -> FbPainter {
        let mut fb = Framebuffer::new("/dev/fb0").unwrap();
        let w = fb.var_screen_info.xres as usize;
        let h = fb.var_screen_info.yres as usize;
        let line_length = fb.fix_screen_info.line_length as usize;
        let bytespp = (fb.var_screen_info.bits_per_pixel / 8) as usize;
        let mut frame = vec![0u8; line_length * h];

        FbPainter {
            fb: fb,
            w: w,
            h: h,
            line_length: line_length,
            bytespp: bytespp,
            frame: frame,
        }
    }

    pub fn update(&mut self, buffer: &[f32]) {
        let bins = buffer.len();
        let bin_width = self.w as f32 / bins as f32;

        for (i, magnitude) in buffer.iter().enumerate() {
            let x_start = (i as f32 * bin_width) as usize;
            let x_stop = ((i + 1) as f32 * bin_width) as usize;
            let y_stop = (self.h as f32 * magnitude) as usize;

            for y in 0 .. y_stop {
                for x in x_start .. x_stop {
                    let curr_index = y * self.line_length + x * self.bytespp;
                    self.frame[curr_index] = 255;
                    self.frame[curr_index + 1] = 255;
                    self.frame[curr_index + 2] = 255;
                }
            }
        }

        self.frame.iter_mut().map(|byte| 255);
        println!("{}", self.frame[0]);
        let _ = self.fb.write_frame(&self.frame);
    }
}
