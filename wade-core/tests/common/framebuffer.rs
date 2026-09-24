//! An in-memory 320×240 Rgb565 framebuffer and snapshot comparison
//! (docs/testing.md#snapshots).

use std::convert::Infallible;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use wade_core::layout::SCREEN_SIZE;

const W: usize = SCREEN_SIZE.width as usize;
const H: usize = SCREEN_SIZE.height as usize;

#[derive(Debug)]
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

fn write_png(path: &Path, rgb: &[u8]) {
    let file = File::create(path).expect("create png");
    let mut enc = png::Encoder::new(BufWriter::new(file), SCREEN_SIZE.width, SCREEN_SIZE.height);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut w| w.write_image_data(rgb))
        .expect("write png");
}

/// Decode a golden image to 8-bit RGB. `Ok(None)` if it does not exist.
fn read_png(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut decoder = png::Decoder::new(BufReader::new(file));
    // Normalize palette and low-bit-depth images, e.g. after a PNG optimizer.
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("image too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if (info.width, info.height) != (SCREEN_SIZE.width, SCREEN_SIZE.height) {
        return Err(format!(
            "golden image is {}×{}, expected {SCREEN_SIZE}",
            info.width, info.height
        ));
    }
    if info.color_type != png::ColorType::Rgb {
        return Err(format!(
            "golden image is {:?}, expected Rgb",
            info.color_type
        ));
    }
    buf.truncate(info.buffer_size());
    Ok(Some(buf))
}

/// Compare `fb` with `tests/snapshots/<name>.png`. On a mismatch, write
/// `<name>.actual.png` and panic. `UPDATE_SNAPSHOTS=1` rewrites the golden image.
pub fn assert_snapshot(name: &str, fb: &Framebuffer) {
    let dir = snapshot_dir();
    let golden = dir.join(format!("{name}.png"));
    let actual_path = dir.join(format!("{name}.actual.png"));
    let actual = fb.to_rgb8();

    if std::env::var("UPDATE_SNAPSHOTS").is_ok_and(|v| v == "1") {
        write_png(&golden, &actual);
        let _ = fs::remove_file(&actual_path);
        return;
    }

    match read_png(&golden) {
        Ok(Some(expected)) if expected == actual => {
            let _ = fs::remove_file(&actual_path);
        }
        Ok(Some(_)) => {
            write_png(&actual_path, &actual);
            panic!("snapshot {name} differs; see {}", actual_path.display());
        }
        Ok(None) => {
            write_png(&actual_path, &actual);
            panic!(
                "snapshot {name} missing; run UPDATE_SNAPSHOTS=1 cargo test to create {}",
                golden.display()
            );
        }
        Err(e) => panic!("snapshot {name}: cannot read {}: {e}", golden.display()),
    }
}
