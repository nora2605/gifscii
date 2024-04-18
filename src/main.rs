use std::cmp::Ordering;
use std::io::{BufReader, Write};
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;
use clap::builder::{PossibleValuesParser, TypedValueParser};

use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, Delay, Frames, ImageDecoder, RgbaImage};

const HALF_BLOCK: char = '▄';

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_parser)]
    input: String,
    
    #[arg(long, value_parser(PossibleValuesParser::new(["nearest", "triangle", "catmullrom", "gaussian", "lanczos3"]).map(|s| FilterType::from_str(&s).unwrap())), default_value = "triangle", required = false)]
    resize_mode: FilterType,

    #[arg(long, default_value = "false", required = false)]
    no_resize: bool,
}

#[derive(Clone, Copy)]
struct FilterType(image::imageops::FilterType);

impl FromStr for FilterType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(match s {
            "nearest" => image::imageops::FilterType::Nearest,
            "triangle" => image::imageops::FilterType::Triangle,
            "catmullrom" => image::imageops::FilterType::CatmullRom,
            "gaussian" => image::imageops::FilterType::Gaussian,
            "lanczos3" => image::imageops::FilterType::Lanczos3,
            _ => return Err("Invalid filter type")
        }))
    }
}

fn main() -> ! {
    let mut stdout = std::io::stdout();

    ctrlc::set_handler(move || {
        crossterm::execute!(
            std::io::stdout(),
            crossterm::cursor::Show,
            ResetColor
        ).unwrap();
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");
    let args = Args::parse();

    let Ok(file) = std::fs::File::open(&args.input) else {
        eprintln!("File doesn't exist.");
        std::process::exit(1);
    };

    if !args.input.ends_with(".gif") {
        eprintln!("File is not a gif.");
        std::process::exit(1);
    }

    let g_decoder = GifDecoder::new(BufReader::new(file)).unwrap();
    let (t_columns, t_rows) = crossterm::terminal::size().unwrap();
    // basically 2x the rows because of the half block
    let (t_sx, t_sy): (u32, u32) = (t_columns.into(), (t_rows * 2).into());
    let (g_sx, g_sy) = g_decoder.dimensions();

    let (sx, sy) = if (g_sx > t_sx || g_sy > t_sy) && !args.no_resize {
        let scale = f64::min(t_sx as f64 / g_sx as f64, t_sy as f64 / g_sy as f64);
        ((g_sx as f64 * scale) as u32, (g_sy as f64 * scale) as u32)
    } else { (g_sx, g_sy) };
    
    
    
    let res_frames = resize_encode(g_decoder.into_frames(), (sx, sy), args.resize_mode);

    crossterm::execute!(
        stdout,
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
                        stdout,
                        ResetColor,
                        Print("\n")
                    ).unwrap();
                }   
            }
            stdout.flush().unwrap();
            std::thread::sleep(if Duration::from(*d).cmp(&start_time.elapsed()) == Ordering::Greater {Duration::from(*d) - start_time.elapsed()} else { Duration::from_secs(0) });
            crossterm::execute!(
                stdout,
                crossterm::cursor::MoveTo(0, 0),
            ).unwrap();
        });
    }
}

fn resize_encode(frames: Frames, to: (u32, u32), resize_mode: FilterType) -> Vec<(RgbaImage, Delay)> {
    let (sx, sy) = to;
    frames.map(|f| -> (RgbaImage, Delay) {
        let f = f.unwrap();
        let del = f.delay();
        let f: &RgbaImage = f.buffer();
        (image::imageops::resize(f, sx, sy, resize_mode.0), del)
    }).collect::<Vec<_>>()
}