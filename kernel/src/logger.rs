use alloc::format;
use bootloader_lib::GraphicInfo;
use core::fmt::Write;
use core::{fmt, ptr};
use log::{error, Level};
use noto_sans_mono_bitmap::{get_bitmap, BitmapChar, BitmapHeight, FontWeight};
use spin::Mutex;
use uart_16550::SerialPort;
use uefi::proto::console::gop::PixelFormat;

pub static mut LOGGER: Option<LockedLogger> = None;
// TODO move to `drivers`
#[cfg(feature = "qemu")]
static mut PORT: SerialPort = unsafe { SerialPort::new(0x3F8) };

pub fn init(info: GraphicInfo) {
    #[cfg(feature = "qemu")]
    unsafe {
        PORT.init();
    }
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
    #[allow(dead_code)]
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
    font_size: BitmapHeight,
    palette: RGB,
    framebuffer: &'static mut [u8],
    x_pos: usize,
    y_pos: usize,
}

struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

impl RGB {
    fn to_rgb_pixel(&self) -> [u8; 4] {
        [self.r, self.g, self.b, 0]
    }

    fn to_bgr_pixel(&self) -> [u8; 4] {
        [self.b, self.g, self.r, 0]
    }
}

impl From<(u8, u8, u8)> for RGB {
    fn from(rgb: (u8, u8, u8)) -> Self {
        RGB {
            r: rgb.0,
            g: rgb.1,
            b: rgb.2,
        }
    }
}

impl Logger {
    pub fn new(info: GraphicInfo) -> Self {
        let mut logger = Self {
            bytes_per_pixel: info.fb_size as usize
                / (info.mode.resolution().0 * info.mode.resolution().1),
            framebuffer: unsafe {
                &mut *ptr::slice_from_raw_parts_mut(info.fb_addr as *mut u8, info.fb_size as usize)
            },
            font_size: BitmapHeight::Size20,
            palette: RGB::from((0, 0, 0)),
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
        self.y_pos += self.font_size.val();
        self.carriage_return()
    }

    #[inline]
    fn carriage_return(&mut self) {
        self.x_pos = 0;
    }

    pub fn clear(&mut self) {
        self.x_pos = 0;
        self.y_pos = 0;
        let background = match self.format {
            PixelFormat::Rgb => Self::BACKGROUND_PIXEL.to_rgb_pixel(),
            PixelFormat::Bgr => Self::BACKGROUND_PIXEL.to_bgr_pixel(),
            _ => [0, 0, 0, 0],
        };
        for i in 0..self.framebuffer.len() {
            self.framebuffer[i] = background[i % 4];
        }
        #[cfg(feature = "qemu")]
        unsafe {
            PORT.send(b'\n');
        }
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
        log_level: Level,
        args: &fmt::Arguments,
        file: &'a str,
        line: u32,
    ) -> fmt::Result {
        self.set_palette(log_level);
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
        #[cfg(feature = "qemu")]
        unsafe {
            PORT.write_char(c).expect("failed to write char to port");
        }
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let bitmap_char = get_bitmap(c, FontWeight::Regular, self.font_size).unwrap();
                if self.x_pos + bitmap_char.width() >= self.width() {
                    self.newline();
                }
                if self.y_pos > (self.height() - self.font_size.val()) {
                    let letter_height = self.font_size.val();
                    let dst = self.framebuffer.as_mut_ptr();
                    let src = &self.framebuffer[(letter_height - (self.ver_res - self.y_pos))
                        * self.stride
                        * self.bytes_per_pixel] as *const u8;
                    let count = (self.ver_res - letter_height) * self.stride * self.bytes_per_pixel;
                    unsafe {
                        ptr::copy(src, dst, count);
                    }
                    self.y_pos = self.height() - letter_height;
                    let background = match self.format {
                        PixelFormat::Rgb => Self::BACKGROUND_PIXEL.to_rgb_pixel(),
                        PixelFormat::Bgr => Self::BACKGROUND_PIXEL.to_bgr_pixel(),
                        _ => [0, 0, 0, 0],
                    };
                    for i in
                        (self.y_pos * self.stride * self.bytes_per_pixel)..self.framebuffer.len()
                    {
                        self.framebuffer[i] = background[i % 4];
                    }
                }
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
        let color = self.get_color(intensity as i32);
        let bytes_per_pixel = self.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        // FIXME volatile
        // let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }

    const BACKGROUND_PIXEL: RGB = RGB {
        r: 40,
        g: 44,
        b: 52,
    };

    /// Colorized log in bgr
    ///
    /// background 40 44 52
    ///
    /// TRACE gray 220, 223, 228
    ///
    /// DEBUG green 161, 193, 129
    ///
    /// INFO blue 115, 173, 233
    ///
    /// WARN yellow 223, 193, 132
    ///
    /// ERROR red 210, 114, 119
    fn set_palette(&mut self, level: Level) {
        let rgb = match level {
            Level::Error => (210, 114, 119),
            Level::Warn => (223, 193, 132),
            Level::Info => (115, 173, 233),
            Level::Debug => (161, 193, 129),
            Level::Trace => (220, 223, 228),
        };
        self.palette = RGB::from(rgb);
    }

    fn get_color(&self, intensity: i32) -> [u8; 4] {
        let red = ((self.palette.r - Self::BACKGROUND_PIXEL.r) as i32 * intensity) / 255
            + Self::BACKGROUND_PIXEL.r as i32;
        let green = ((self.palette.g - Self::BACKGROUND_PIXEL.g) as i32 * intensity) / 255
            + Self::BACKGROUND_PIXEL.g as i32;
        let blue = ((self.palette.b - Self::BACKGROUND_PIXEL.b) as i32 * intensity) / 255
            + Self::BACKGROUND_PIXEL.b as i32;

        match self.format {
            PixelFormat::Rgb => [red as u8, green as u8, blue as u8, 0],
            PixelFormat::Bgr => [blue as u8, green as u8, red as u8, 0],
            _ => [0, 0, 0, 0],
        }
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
