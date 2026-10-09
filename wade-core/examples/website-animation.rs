//! Export the installer's animation through the core's event loop and face renderer.

use std::error::Error;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use embedded_graphics::pixelcolor::{Rgb565, Rgb888, raw::RawU16};
use embedded_graphics::prelude::RgbColor;
use wade_core::Instant;
use wade_core::character::Expression;
use wade_core::harness::Harness;
use wade_core::layout::{SCREEN_SIZE, WADE_CENTER};
use wade_core::render::{self, Band, ROW_BYTES};
use wade_core::view::BuddyView;

const FRAME_MS: u64 = 40;
const START_MS: u64 = 1_000;

fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args_os()
        .nth(1)
        .ok_or("usage: website-animation OUTPUT_DIRECTORY")?;
    let directory = Path::new(&directory);
    fs::create_dir_all(directory)?;
    let mut harness = Harness::new(42);
    harness.run_until(Instant::from_millis(START_MS));
    let start = harness.buddy().pose;
    let mut blinks = 0;
    let mut happy = 0;
    for frame in 0..750 {
        let elapsed = frame * FRAME_MS;
        let now = Instant::from_millis(START_MS + elapsed);
        harness.run_until(now);
        if elapsed == 6_000 {
            harness.tap(now, WADE_CENTER);
        }
        let buddy = harness.buddy();
        blinks += usize::from(buddy.blinking);
        happy += usize::from(buddy.expression == Expression::Happy);
        write_frame(&directory.join(format!("frame-{frame:03}.png")), &buddy)?;
        // End at the opening pose so the GIF loops without a jump.
        if elapsed >= 12_000 && buddy.pose == start {
            for old in frame + 1..750 {
                let path = directory.join(format!("frame-{old:03}.png"));
                if path.exists() {
                    fs::remove_file(path)?;
                }
            }
            println!(
                "{} frames at 25 fps; {blinks} blinking frames, {happy} Happy frames",
                frame + 1
            );
            return Ok(());
        }
    }
    Err("Wade did not return to the opening pose within 30 seconds".into())
}

fn write_frame(path: &Path, buddy: &BuddyView) -> Result<(), Box<dyn Error>> {
    let mut pixels = Vec::new();
    let mut buffer = vec![0; ROW_BYTES * 40];
    for top in Band::tops(40) {
        buffer.fill(0);
        let mut band = Band::new(&mut buffer, top);
        let Ok(()) = render::face::draw(&buddy.pose, buddy.eye_style, buddy.color, &mut band);
        for pixel in band.bytes().as_chunks::<2>().0 {
            let packed = u16::from_be_bytes(*pixel);
            let color = Rgb888::from(Rgb565::from(RawU16::new(packed)));
            pixels.extend_from_slice(&[color.r(), color.g(), color.b()]);
        }
    }
    let file = BufWriter::new(File::create(path)?);
    let mut encoder = png::Encoder::new(file, SCREEN_SIZE.width, SCREEN_SIZE.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&pixels)?;
    Ok(())
}
