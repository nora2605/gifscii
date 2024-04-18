use std::cmp::Ordering;
use std::io::{BufReader, Write};
use std::time::Duration;

use clap::Parser;
use anyhow::Result;

use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, Delay, RgbaImage};

const HALF_BLOCK: char = '▄';

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_parser)]
    input: String,
}

fn main() -> ! {
    ctrlc::set_handler(move || {
        crossterm::execute!(
            std::io::stdout(),
            crossterm::cursor::Show,
            ResetColor
        ).unwrap();
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");
    let args = Args::parse();

    let file = match std::fs::File::open(args.input) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("File doesn't exist. Get mogged.");
            std::process::exit(1);
        }
    };

    let g_decoder
        = GifDecoder::new(BufReader::new(file)).unwrap();
    let mut raw_frames = vec![];
    g_decoder.into_frames().map(|f| -> Result<(RgbaImage, Delay)> {
        let f = f?;
        let del = f.delay();
        let buf: &RgbaImage = f.buffer();
        Ok((buf.clone(), del))
    })
    .for_each(|f| {
        raw_frames.push(f.unwrap());
    });

    let (t_columns, t_rows) = crossterm::terminal::size().unwrap();
    let (g_sx, g_sy) = raw_frames[0].0.dimensions();

    // calculate the dimensions of the largest image that fits (2 pixels per character in height)
    let (sx, sy) = if g_sx > t_columns.into() || g_sy > t_rows.into() {
        let scale = f64::min(t_columns as f64 / g_sx as f64, t_rows as f64 / g_sy as f64);
        ((g_sx as f64 * scale) as u32, 2 * (g_sy as f64 * scale) as u32)
    } else {
        (g_sx, g_sy * 2)
    };

    let res_frames = raw_frames.iter().map(|(f, del)| {
        let mut res_frame = RgbaImage::new(sx, sy);
        for y in 0..sy {
            for x in 0..sx {
                let x0 = ((x as f64 / sx as f64) * g_sx as f64) as u32;
                let y0 = ((y as f64 / sy as f64) * g_sy as f64) as u32;
                let x1 = (((x + 1) as f64 / sx as f64) * g_sx as f64) as u32;
                let y1 = (((y + 1) as f64 / sy as f64) * g_sy as f64) as u32;
                let mut r = 0; let mut g = 0; let mut b = 0; let mut a = 0;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let pixel = f.get_pixel(x, y);
                        let [r0, g0, b0, a0] = pixel.0;
                        r += r0 as u32; g += g0 as u32; b += b0 as u32; a += a0 as u32;
                    }
                }
                let n = (x1 - x0) * (y1 - y0);
                let r = (r / n) as u8;
                let g = (g / n) as u8;
                let b = (b / n) as u8;
                let a = (a / n) as u8;
                res_frame.put_pixel(x, y, [r, g, b, a].into());
            }
        }
        (res_frame, *del)
    }).collect::<Vec<_>>();

    drop(raw_frames);

    crossterm::execute!(
        std::io::stdout(),
        crossterm::cursor::Hide,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
    ).unwrap();

    loop {
        res_frames.iter().for_each(|(f, d)| {
            let start_time = std::time::Instant::now();
            for y in 0..sy/2 {
                for x in 0..sx {
                    let pixel1 = f.get_pixel(x, 2*y);
                    let pixel2 = f.get_pixel(x, 2*y + 1);
                    let [r1, g1, b1, _] = pixel1.0;
                    let [r2, g2, b2, _] = pixel2.0;
                    crossterm::queue!(
                        std::io::stdout(),
                        SetColors(
                            Colors::new(
                                Color::Rgb { r: r2, g: g2, b: b2 },
                                Color::Rgb { r: r1, g: g1, b: b1 })
                            ),
                        Print(HALF_BLOCK.to_string())
                    ).unwrap();
                }
                if y != sy/2 - 1 {
                    crossterm::queue!(
                        std::io::stdout(),
                        ResetColor,
                        Print("\n")
                    ).unwrap();
                }   
            }
            std::io::stdout().flush().unwrap();
            std::thread::sleep(if Duration::from(*d).cmp(&start_time.elapsed()) == Ordering::Greater {Duration::from(*d) - start_time.elapsed()} else { Duration::from_secs(0) });
            crossterm::execute!(
                std::io::stdout(),
                crossterm::cursor::MoveTo(0, 0),
            ).unwrap();
        });
    }
}