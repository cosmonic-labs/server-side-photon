/// Generate a gradient PNG test image of specified dimensions.
/// Usage: mkimage <width> <height> [output.png]
/// If no output path, writes to stdout.
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: mkimage <width> <height> [output.png]");
        std::process::exit(1);
    }

    let width: u32 = args[1].parse().expect("invalid width");
    let height: u32 = args[2].parse().expect("invalid height");

    let mut buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut buf, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("failed to write PNG header");

        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let r = (x * 255 / width.max(1)) as u8;
                let g = (y * 255 / height.max(1)) as u8;
                let b = ((x.wrapping_mul(3).wrapping_add(y.wrapping_mul(7))) % 256) as u8;
                data.extend_from_slice(&[r, g, b, 255]);
            }
        }
        writer.write_image_data(&data).expect("failed to write PNG data");
    }

    if args.len() > 3 {
        std::fs::write(&args[3], &buf).expect("failed to write file");
        eprintln!("Wrote {}x{} PNG ({} bytes) to {}", width, height, buf.len(), args[3]);
    } else {
        std::io::stdout().write_all(&buf).expect("failed to write stdout");
    }
}
