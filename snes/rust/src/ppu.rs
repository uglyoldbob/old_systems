//! The ppu module for the emulator. Responsible for emulating the chip that generates all of the graphics for the snes.

use crate::motherboard::SnesMotherboard;
use egui_multiwin::egui::Vec2;
use serde_with::Bytes;

#[cfg(feature = "eframe")]
use eframe::egui;

#[cfg(feature = "egui-multiwin")]
use egui_multiwin::egui;

/// The types of algorithms for scaling up the image
#[derive(
    PartialEq, strum::Display, strum::EnumIter, serde::Serialize, serde::Deserialize, Clone, Copy,
)]
pub enum ScalingAlgorithm {
    ///The Scale2x algorithm, based on EPX (Eric's pixel expansion)
    Scale2x,
    ///The Scale3x algorithm, extension of scale2x to x3.
    Scale3x,
    ///The eagle scaling algorithm
    Eagle,
}

/// A 24bpp pixel.
#[derive(Copy, Clone, std::cmp::PartialEq, Default)]
pub struct Pixel {
    /// The red channel
    r: u8,
    /// The green channel
    g: u8,
    /// The blue channel
    b: u8,
}

/// A generic pixel based image
#[derive(Clone)]
pub struct PixelImage<T> {
    /// The actual pixels of the image
    pixels: Vec<T>,
    /// The width of the image in pixels.
    pub width: u16,
    /// The height of the image in pixels.
    pub height: u16,
}

impl PixelImage<egui::Color32> {
    /// Apply data received from gstreamer to this image
    pub fn receive_from_gstreamer(&mut self, d: Vec<u8>) {
        for (i, p) in d.chunks_exact(3).enumerate() {
            let pixel = egui::Color32::from_rgb(p[0], p[1], p[2]);
            self.pixels[i] = pixel;
        }
    }

    /// Converts the image to an egui usable format
    pub fn to_egui(self) -> egui::ColorImage {
        egui::ColorImage {
            size: [self.width as usize, self.height as usize],
            pixels: self.pixels,
        }
    }

    /// Converts to a vector that can be eventually passed to a gstreamer pipeline
    pub fn to_gstreamer_vec(&self) -> Vec<u8> {
        let oc = self.width as usize * self.height as usize;
        let mut v = Vec::with_capacity(oc * 3);
        for p in self.pixels.iter() {
            v.push(p.r());
            v.push(p.g());
            v.push(p.b());
        }
        v
    }

    /// Converts to a slice that gstreamer can use
    pub fn to_gstreamer(&self, buf: &mut gstreamer::Buffer) {
        let v = self.to_gstreamer_vec();
        let mut p = buf.make_mut().map_writable().unwrap();
        for (a, b) in v.iter().zip(p.iter_mut()) {
            *b = *a;
        }
    }
}

impl<T> Default for PixelImage<T>
where
    T: Default + Clone,
{
    fn default() -> Self {
        Self {
            pixels: vec![T::default(); 1],
            width: 1,
            height: 1,
        }
    }
}

impl<T> PixelImage<T>
where
    T: Default + Clone + Copy + std::cmp::PartialEq,
{
    /// Retrieves the pixel for the image
    pub fn get_pixel(&self, pos: Vec2) -> T {
        let x = (pos.x as usize).min(self.width as usize - 1);
        let y = (pos.y as usize).min(self.height as usize - 1);
        let index = x + y * self.width as usize;
        self.pixels[index]
    }

    /// Resize the image using an optional resizing algorithm
    pub fn resize(self, scale: Option<ScalingAlgorithm>) -> PixelImage<T> {
        let pixels = self.pixels;
        let (pixels, width, height) = match scale {
            None => (pixels, self.width as usize, self.height as usize),
            Some(alg) => match alg {
                ScalingAlgorithm::Scale2x => {
                    let mut newpixels = vec![T::default(); pixels.len() * 4];
                    for y in 0..self.height {
                        for x in 0..self.width {
                            let p = pixels[y as usize * self.width as usize + x as usize];
                            let mut pg: [T; 4] = [p; 4];
                            let a = if y > 0 {
                                pixels[(y - 1) as usize * self.width as usize + x as usize]
                            } else {
                                T::default()
                            };
                            let d = if (y + 1) < self.height {
                                pixels[(y + 1) as usize * self.width as usize + x as usize]
                            } else {
                                T::default()
                            };
                            let c = if x > 0 {
                                pixels[y as usize * self.width as usize + x as usize - 1]
                            } else {
                                T::default()
                            };
                            let b = if (x + 1) < self.width {
                                pixels[y as usize * self.width as usize + x as usize + 1]
                            } else {
                                T::default()
                            };
                            if c == a && c != d && a != b {
                                pg[0] = a;
                            }
                            if a == b && a != c && b != d {
                                pg[1] = b;
                            }
                            if d == c && d != b && c != a {
                                pg[2] = c;
                            }
                            if b == d && b != a && d != c {
                                pg[3] = d;
                            }
                            newpixels[2 * y as usize * 2 * self.width as usize + 2 * x as usize] =
                                pg[0];
                            newpixels
                                [2 * y as usize * 2 * self.width as usize + 2 * x as usize + 1] =
                                pg[1];
                            newpixels
                                [(2 * y + 1) as usize * 2 * self.width as usize + 2 * x as usize] =
                                pg[2];
                            newpixels[(2 * y + 1) as usize * 2 * self.width as usize
                                + 2 * x as usize
                                + 1] = pg[3];
                        }
                    }
                    (newpixels, 2 * self.width as usize, 2 * self.height as usize)
                }
                ScalingAlgorithm::Scale3x => {
                    let mut newpixels = vec![T::default(); pixels.len() * 9];
                    for y in 0..self.height {
                        for x in 0..self.width {
                            let letters: [T; 9] = [
                                if x > 0 && y > 0 {
                                    pixels[(y - 1) as usize * self.width as usize + x as usize - 1]
                                } else {
                                    T::default()
                                },
                                if y > 0 {
                                    pixels[(y - 1) as usize * self.width as usize + x as usize]
                                } else {
                                    T::default()
                                },
                                if y > 0 && (x + 1) < self.width {
                                    pixels[(y - 1) as usize * self.width as usize + x as usize + 1]
                                } else {
                                    T::default()
                                },
                                if x > 0 {
                                    pixels[y as usize * self.width as usize + x as usize - 1]
                                } else {
                                    T::default()
                                },
                                pixels[y as usize * self.width as usize + x as usize],
                                if (x + 1) < self.width {
                                    pixels[y as usize * self.width as usize + x as usize + 1]
                                } else {
                                    T::default()
                                },
                                if x > 0 && (y + 1) < self.height {
                                    pixels[(y + 1) as usize * self.width as usize + x as usize - 1]
                                } else {
                                    T::default()
                                },
                                if (y + 1) < self.height {
                                    pixels[(y + 1) as usize * self.width as usize + x as usize]
                                } else {
                                    T::default()
                                },
                                if (y + 1) < self.height && (x + 1) < self.width {
                                    pixels[(y + 1) as usize * self.width as usize + x as usize + 1]
                                } else {
                                    T::default()
                                },
                            ];
                            let mut pg: [T; 9] = [letters[4]; 9];

                            let a = letters[0];
                            let b = letters[1];
                            let c = letters[2];
                            let d = letters[3];
                            let e = letters[4];
                            let f = letters[5];
                            let g = letters[6];
                            let h = letters[7];
                            let i = letters[8];

                            if d == b && d != h && b != f {
                                pg[0] = d;
                            }
                            if (d == b && d != h && b != f && e != c)
                                || (b == f && b != d && f != h && e != a)
                            {
                                pg[1] = b;
                            }
                            if b == f && b != d && f != h {
                                pg[2] = f;
                            }
                            if (h == d && h != f && d != b && e != a)
                                || (d == b && d != h && b != f && e != g)
                            {
                                pg[3] = d;
                            }
                            if (b == f && b != d && f != h && e != i)
                                || (f == h && f != b && h != d && e != d)
                            {
                                pg[5] = f;
                            }
                            if h == d && h != f && d != b {
                                pg[6] = d;
                            }
                            if (f == h && f != b && h != d && e != g)
                                || (h == d && h != f && d != b && e != i)
                            {
                                pg[7] = h;
                            }
                            if f == h && f != b && h != d {
                                pg[8] = f;
                            }

                            newpixels[3 * y as usize * 3 * self.width as usize + 3 * x as usize] =
                                pg[0];
                            newpixels
                                [3 * y as usize * 3 * self.width as usize + 3 * x as usize + 1] =
                                pg[1];
                            newpixels
                                [3 * y as usize * 3 * self.width as usize + 3 * x as usize + 2] =
                                pg[2];

                            newpixels
                                [(3 * y + 1) as usize * 3 * self.width as usize + 3 * x as usize] =
                                pg[3];
                            newpixels[(3 * y + 1) as usize * 3 * self.width as usize
                                + 3 * x as usize
                                + 1] = pg[4];
                            newpixels[(3 * y + 1) as usize * 3 * self.width as usize
                                + 3 * x as usize
                                + 2] = pg[5];

                            newpixels
                                [(3 * y + 2) as usize * 3 * self.width as usize + 3 * x as usize] =
                                pg[6];
                            newpixels[(3 * y + 2) as usize * 3 * self.width as usize
                                + 3 * x as usize
                                + 1] = pg[7];
                            newpixels[(3 * y + 2) as usize * 3 * self.width as usize
                                + 3 * x as usize
                                + 2] = pg[8];
                        }
                    }
                    (newpixels, 3 * self.width as usize, 3 * self.height as usize)
                }
                ScalingAlgorithm::Eagle => {
                    let mut newpixels = vec![T::default(); pixels.len() * 4];
                    for y in 0..self.height {
                        for x in 0..self.width {
                            let letters: [T; 9] = [
                                if x > 0 && y > 0 {
                                    pixels[(y - 1) as usize * self.width as usize + x as usize - 1]
                                } else {
                                    T::default()
                                },
                                if y > 0 {
                                    pixels[(y - 1) as usize * self.width as usize + x as usize]
                                } else {
                                    T::default()
                                },
                                if y > 0 && (x + 1) < self.width {
                                    pixels[(y - 1) as usize * self.width as usize + x as usize + 1]
                                } else {
                                    T::default()
                                },
                                if x > 0 {
                                    pixels[y as usize * self.width as usize + x as usize - 1]
                                } else {
                                    T::default()
                                },
                                pixels[y as usize * self.width as usize + x as usize],
                                if (x + 1) < self.width {
                                    pixels[y as usize * self.width as usize + x as usize + 1]
                                } else {
                                    T::default()
                                },
                                if x > 0 && (y + 1) < self.height {
                                    pixels[(y + 1) as usize * self.width as usize + x as usize - 1]
                                } else {
                                    T::default()
                                },
                                if (y + 1) < self.height {
                                    pixels[(y + 1) as usize * self.width as usize + x as usize]
                                } else {
                                    T::default()
                                },
                                if (y + 1) < self.height && (x + 1) < self.width {
                                    pixels[(y + 1) as usize * self.width as usize + x as usize + 1]
                                } else {
                                    T::default()
                                },
                            ];
                            let mut pg: [T; 4] = [letters[4]; 4];

                            let s = letters[0];
                            let t = letters[1];
                            let u = letters[2];
                            let v = letters[3];
                            let w = letters[5];
                            let xx = letters[6];
                            let yy = letters[7];
                            let z = letters[8];

                            if v == s && s == t {
                                pg[0] = s;
                            }
                            if t == u && u == w {
                                pg[1] = u;
                            }
                            if v == xx && xx == yy {
                                pg[2] = xx;
                            }

                            if w == z && z == yy {
                                pg[3] = z;
                            }

                            newpixels[2 * y as usize * 2 * self.width as usize + 2 * x as usize] =
                                pg[0];
                            newpixels
                                [2 * y as usize * 2 * self.width as usize + 2 * x as usize + 1] =
                                pg[1];
                            newpixels
                                [(2 * y + 1) as usize * 2 * self.width as usize + 2 * x as usize] =
                                pg[2];
                            newpixels[(2 * y + 1) as usize * 2 * self.width as usize
                                + 2 * x as usize
                                + 1] = pg[3];
                        }
                    }
                    (newpixels, 2 * self.width as usize, 2 * self.height as usize)
                }
            },
        };
        PixelImage::<T> {
            pixels,
            width: width as u16,
            height: height as u16,
        }
    }
}

/// A rgb image of variable size. Each pixel is 8 bits per channel, red, green, blue.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct RgbImage {
    /// The raw data of the image
    data: Vec<u8>,
    /// The width of the image in pixels.
    pub width: u16,
    /// The height of the image in pixels.
    pub height: u16,
}

impl Default for RgbImage {
    fn default() -> Self {
        Self {
            data: vec![0; 1],
            width: 1,
            height: 1,
        }
    }
}

impl RgbImage {
    /// Create a blank rgb image of the specified dimensions.
    pub fn new(w: u16, h: u16) -> Self {
        let cap = w as usize * h as usize * 3;
        let m = vec![0; cap];
        Self {
            data: m,
            width: w,
            height: h,
        }
    }

    /// Retrieves the pixel for the image
    pub fn get_pixel(&self, pos: Vec2) -> [u8; 3] {
        let mut p = [0; 3];
        let index = pos.x as usize + pos.y as usize * self.width as usize;
        p[0] = self.data[index * 3];
        p[1] = self.data[index * 3 + 1];
        p[2] = self.data[index * 3 + 2];
        p
    }

    /// Convert to a PixelImage<Pixel>
    pub fn to_pixels(&self) -> PixelImage<Pixel> {
        let pixels: Vec<Pixel> = self
            .data
            .chunks_exact(3)
            .map(|p| Pixel {
                r: p[0],
                g: p[1],
                b: p[2],
            })
            .collect();
        PixelImage::<Pixel> {
            pixels,
            width: self.width,
            height: self.height,
        }
    }

    /// Converts to to pixels using egui pixel format
    pub fn to_pixels_egui(&self) -> PixelImage<egui::Color32> {
        let pixels: Vec<egui::Color32> = self
            .data
            .chunks_exact(3)
            .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        PixelImage::<egui::Color32> {
            pixels,
            width: self.width,
            height: self.height,
        }
    }
}

/// The structure for the snes PPU (picture processing unit)
#[non_exhaustive]
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnesPpu {
    /// The frame data stored in the ppu for being displayed onto the screen later.
    frame_data: Box<RgbImage>,
    /// The frame number of the ppu, used for testing and debugging purposes.
    #[cfg(any(test, feature = "debugger"))]
    frame_number: u64,
    #[cfg(feature = "debugger")]
    /// For debugging pixel generation of the background
    pub bg_debug: Option<(u8, u8)>,
    /// The flag that indicates the end of a frame has occurred. Used for synchronizing frame rate of the emulator.
    frame_end: bool,
}

impl SnesPpu {
    /// Construct a new ppu
    pub fn new() -> Self {
        Self {
            frame_data: Box::new(RgbImage::new(256, 224)),
            #[cfg(any(test, feature = "debugger"))]
            frame_number: 0,
            #[cfg(any(test, feature = "debugger"))]
            bg_debug: None,
            frame_end: false,
        }
    }

    /// Returns true if the frame has ended. Used for frame rate synchronizing.
    pub fn get_frame_end(&mut self) -> bool {
        let flag = self.frame_end;
        self.frame_end = false;
        flag
    }

    /// Return the frame number of the ppu, mostly used for testing and debugging the ppu
    #[cfg(any(test, feature = "debugger"))]
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Returns a reference to the frame data stored in the ppu.
    pub fn get_frame(&mut self) -> &RgbImage {
        &self.frame_data
    }

    /// Get a backup of the ppu frame
    pub fn backup_frame(&self) -> Box<RgbImage> {
        self.frame_data.clone()
    }

    /// Restore the frame from a backup
    pub fn set_frame(&mut self, f: &RgbImage) {
        self.frame_data = Box::new(f.clone());
    }

    /// Run a single clock cycle of the ppu
    pub fn cycle(&mut self, bus: &mut SnesMotherboard) {
        self.frame_end = true;
    }
}
