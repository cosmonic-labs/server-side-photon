/// Build a binary-framed transform request payload.
/// Usage: mkpayload <transform_name> <image.png> [param=value ...]
/// Writes the binary payload to stdout.
///
/// Examples:
///   mkpayload effects.oil test.png int_val=4 float_val=55
///   mkpayload monochrome.grayscale test.png
///   mkpayload filters.filter test.png filter_name=oceanic
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: mkpayload <transform> <image.png> [param=value ...]");
        std::process::exit(1);
    }

    let transform = &args[1];
    let image_path = &args[2];

    // Build params object
    let mut params = serde_json::Map::new();
    for arg in &args[3..] {
        let (key, val) = arg.split_once('=').unwrap_or_else(|| {
            eprintln!("Invalid param (expected key=value): {arg}");
            std::process::exit(1);
        });

        // Try parsing as number first, fall back to string
        if let Ok(i) = val.parse::<i64>() {
            params.insert(key.to_string(), serde_json::Value::Number(i.into()));
        } else if let Ok(f) = val.parse::<f64>() {
            params.insert(
                key.to_string(),
                serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap()),
            );
        } else {
            params.insert(key.to_string(), serde_json::Value::String(val.to_string()));
        }
    }

    let header = serde_json::json!({
        "transform": transform,
        "params": params,
    });
    let header_bytes = serde_json::to_vec(&header).expect("failed to serialize JSON");
    let image_bytes = std::fs::read(image_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {image_path}: {e}");
        std::process::exit(1);
    });

    let mut out = std::io::stdout().lock();
    out.write_all(&(header_bytes.len() as u32).to_be_bytes())
        .expect("write failed");
    out.write_all(&header_bytes).expect("write failed");
    out.write_all(&image_bytes).expect("write failed");
}
