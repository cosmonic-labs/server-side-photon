/// Validate a PNG file and print its dimensions.
/// Usage: check-png <file.png>
/// Exit code 0 if valid PNG, 1 otherwise.
/// Outputs: width height bytes  (tab-separated)

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: check-png <file.png>");
        std::process::exit(1);
    }

    let path = &args[1];
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("ERROR: cannot read {path}: {e}");
            std::process::exit(1);
        }
    };

    if data.len() < 8 || &data[..8] != b"\x89PNG\r\n\x1a\n" {
        eprintln!("ERROR: not a valid PNG file");
        std::process::exit(1);
    }

    // Decode with png crate to get dimensions
    let decoder = png::Decoder::new(data.as_slice());
    match decoder.read_info() {
        Ok(reader) => {
            let info = reader.info();
            println!("{}\t{}\t{}", info.width, info.height, data.len());
        }
        Err(e) => {
            eprintln!("ERROR: corrupt PNG: {e}");
            std::process::exit(1);
        }
    }
}
