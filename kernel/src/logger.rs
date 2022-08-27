use alloc::format;
use bootloader_lib::GraphicInfo;
use core::fmt::Write;
use core::{fmt, ptr};
use log::{debug, error, info};
use noto_sans_mono_bitmap::{get_bitmap, BitmapChar, BitmapHeight, FontWeight};
use spin::Mutex;
use uefi::proto::console::gop::PixelFormat;

pub static mut LOGGER: Option<LockedLogger> = None;

pub fn init_logger(info: GraphicInfo) {
    let logger = unsafe {
        LOGGER = Some(LockedLogger::new(info));
        LOGGER.as_ref().unwrap()
    };
    log::set_logger(logger).unwrap();
    log::set_max_level(log::STATIC_MAX_LEVEL);
}

pub struct LockedLogger(Mutex<Logger>);

impl LockedLogger {
    pub fn new(info: GraphicInfo) -> Self {
        LockedLogger(Mutex::new(Logger::new(info)))
    }

    /// Force-unlocks the logger to prevent a deadlock.
    ///
    /// This method is not memory safe and should be only used when absolutely necessary.
    pub unsafe fn force_unlock(&self) {
        self.0.force_unlock();
    }
}

impl log::Log for LockedLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut logger = self.0.lock();
        if let Err(e) = logger.write(
            record.level(),
            record.args(),
            record.file().unwrap_or("<unknown file>"),
            record.line().unwrap_or(0),
        ) {
            error!("Logger Error: {:#?}", e);
        }
    }

    fn flush(&self) {}
}

pub struct Logger {
    hor_res: usize,
    ver_res: usize,
    format: PixelFormat,
    stride: usize,
    bytes_per_pixel: usize,
    framebuffer: &'static mut [u8],
    x_pos: usize,
    y_pos: usize,
}

impl Logger {
    pub fn new(info: GraphicInfo) -> Self {
        let mut logger = Self {
            bytes_per_pixel: info.fb_size as usize
                / (info.mode.resolution().0 * info.mode.resolution().1),
            framebuffer: unsafe {
                &mut *ptr::slice_from_raw_parts_mut(info.fb_addr as *mut u8, info.fb_size as usize)
            },
            x_pos: 0,
            y_pos: 0,
            hor_res: info.mode.resolution().0,
            ver_res: info.mode.resolution().1,
            format: info.mode.pixel_format(),
            stride: info.mode.stride(),
        };
        logger.clear();
        logger
    }

    #[inline]
    fn newline(&mut self) {
        self.y_pos += 14;
        self.carriage_return()
    }

    #[inline]
    fn carriage_return(&mut self) {
        self.x_pos = 0;
    }

    // #[inline]
    pub fn clear(&mut self) {
        self.x_pos = 0;
        self.y_pos = 0;
        self.framebuffer.fill(0);
    }

    #[inline]
    fn width(&self) -> usize {
        self.hor_res
    }

    #[inline]
    fn height(&self) -> usize {
        self.ver_res
    }

    fn write<'a>(
        &mut self,
        log_level: log::Level,
        args: &fmt::Arguments,
        file: &'a str,
        line: u32,
    ) -> fmt::Result {
        let s = format!("{}", *args);
        let prefix = format!("[{:>5}]: {:>12}@{:03}: ", log_level, file, line);
        let mut lines = s.lines();
        let first = lines.next().unwrap_or("");
        write!(self, "{}{}\n", prefix, first)?;

        for line in lines {
            write!(self, "{}{}\n", prefix, line)?;
        }

        if let Some('\n') = s.chars().next_back() {
            write!(self, "{}\n", prefix)?;
        }

        Ok(())
    }

    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                if self.x_pos >= self.width() {
                    self.newline();
                }
                if self.y_pos > (self.height() - BitmapHeight::Size14.val()) {
                    let letter_height = BitmapHeight::Size14.val();
                    let dst = self.framebuffer.as_mut_ptr();
                    let src = &self.framebuffer[(letter_height - (self.ver_res - self.y_pos))
                        * self.stride
                        * self.bytes_per_pixel] as *const u8;
                    let count = (self.ver_res - letter_height) * self.stride * self.bytes_per_pixel;
                    // debug!("dst {:#?}, src {:#?}, count {}", dst, src, count);
                    unsafe {
                        ptr::copy(src, dst, count);
                    }
                    self.y_pos = self.height() - letter_height;
                    self.framebuffer[(self.y_pos * self.stride * self.bytes_per_pixel)..].fill(0);
                }
                let bitmap_char = get_bitmap(c, FontWeight::Regular, BitmapHeight::Size14).unwrap();
                self.write_rendered_char(bitmap_char);
            }
        }
    }

    fn write_rendered_char(&mut self, rendered_char: BitmapChar) {
        for (y, row) in rendered_char.bitmap().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.x_pos += rendered_char.width();
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * self.stride + x;
        let color = match self.format {
            PixelFormat::Rgb => [intensity, intensity, intensity / 2, 0],
            PixelFormat::Bgr => [intensity / 2, intensity, intensity, 0],
            _ => [0, 0, 0, 0],
        };
        let bytes_per_pixel = self.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        // FIXME volatile
        // let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }
}

unsafe impl Send for Logger {}
unsafe impl Sync for Logger {}

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}
