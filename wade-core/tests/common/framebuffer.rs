//! An in-memory 320×240 Rgb565 framebuffer and snapshot comparison
//! (docs/testing.md#snapshots).

use std::convert::Infallible;
use std::path::PathBuf;

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use wade_core::layout::SCREEN_SIZE;

const W: usize = SCREEN_SIZE.width as usize;
const H: usize = SCREEN_SIZE.height as usize;

pub struct Framebuffer {
    pixels: Vec<Rgb565>,
}

impl Framebuffer {
    pub fn new() -> Self {
        Framebuffer {
            pixels: vec![Rgb565::BLACK; W * H],
        }
    }

    fn to_rgb8(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|c| {
                let c: embedded_graphics::pixelcolor::Rgb888 = (*c).into();
                [c.r(), c.g(), c.b()]
            })
            .collect()
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        SCREEN_SIZE
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Infallible>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        for Pixel(p, c) in pixels {
            if let (Ok(x), Ok(y)) = (usize::try_from(p.x), usize::try_from(p.y))
                && x < W
                && y < H
            {
                self.pixels[y * W + x] = c;
            }
        }
        Ok(())
    }
}

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
}

fn write_png(path: &std::path::Path, rgb: &[u8]) {
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), W as u32, H as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut w| w.write_image_data(rgb))
        .expect("write png");
}

fn read_png(path: &std::path::Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    Some(buf)
}

/// Compare `fb` with `tests/snapshots/<name>.png`. On a mismatch, write
/// `<name>.actual.png` and panic. `UPDATE_SNAPSHOTS=1` rewrites the golden image.
pub fn assert_snapshot(name: &str, fb: &Framebuffer) {
    let dir = snapshot_dir();
    let golden = dir.join(format!("{name}.png"));
    let actual_path = dir.join(format!("{name}.actual.png"));
    let actual = fb.to_rgb8();

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        write_png(&golden, &actual);
        let _ = std::fs::remove_file(&actual_path);
        return;
    }

    match read_png(&golden) {
        Some(expected) if expected == actual => {
            let _ = std::fs::remove_file(&actual_path);
        }
        Some(_) => {
            write_png(&actual_path, &actual);
            panic!("snapshot {name} differs; see {}", actual_path.display());
        }
        None => {
            write_png(&actual_path, &actual);
            panic!(
                "snapshot {name} missing; run UPDATE_SNAPSHOTS=1 cargo test to create {}",
                golden.display()
            );
        }
    }
}
