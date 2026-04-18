# Photon wasmCloud Demo

A reference application demonstrating server-side image processing with WebAssembly using [wasmCloud](https://wasmcloud.com) and [photon-rs](https://github.com/silvia-odwyer/photon).

All ~130 image transforms from photon-rs run server-side as Wasm components, proving that matrix math and image processing work well in the WebAssembly component model.

## Architecture

```
┌────────┐      ┌──────────────┐      ┌──────────┐      ┌──────────────┐
│Browser │─────▶│ http-api     │─────▶│ NATS     │─────▶│ task-photon  │
│        │◀─────│ (:8000)      │◀─────│          │◀─────│              │
└────────┘      └──────────────┘      └──────────┘      └──────────────┘
                 Serves UI                               Runs photon-rs
                 POST /api/transform                     transforms on
                 GET  /api/transforms                    image bytes
```

- **http-api** (`http-api/`): HTTP server component that serves the web UI at `/`, exposes REST endpoints, and forwards image processing requests to the worker via NATS messaging.
- **task-photon** (`task-photon/`): Worker component that receives images + transform names over NATS, applies photon-rs transforms, and returns processed images.

Both components compile to `wasm32-wasip2` and communicate via wasmCloud's `wasmcloud:messaging` interface (backed by NATS). The WIT interface definitions live in `wit/world.wit`.

### Project Structure

```
photon-demo/
├── .wash/config.yaml           # wash dev/build configuration
├── Cargo.toml                  # workspace root (http-api, task-photon)
├── wit/world.wit               # WIT world definitions for both components
├── http-api/
│   ├── src/lib.rs              # HTTP routes: /, /api/transforms, /api/transform
│   ├── ui.html                 # Single-file web UI (embedded via include_str!)
│   └── transforms.json         # Transform catalog (drives sidebar + API)
├── task-photon/
│   └── src/
│       ├── lib.rs              # NATS message handler, binary protocol
│       └── transforms.rs       # Dispatch table for ~130 photon-rs functions
└── tests/
    ├── test_all_transforms.sh  # Correctness test suite
    ├── benchmark.sh            # Performance benchmarking framework
    └── tools/                  # Rust CLI helpers (mkimage, mkpayload, check-png)
```

## Prerequisites

- [wash CLI](https://wasmcloud.com/docs/installation) (v2.0+)
- Rust toolchain with the `wasm32-wasip2` target
- `curl` and `jq` (for running tests)

```bash
rustup target add wasm32-wasip2
```

## Quick Start

```bash
# Build both components
wash build

# Start the development server (builds, deploys, and watches for changes)
wash dev

# Open the web UI
open http://localhost:8000
```

`wash dev` starts a local wasmCloud host, deploys both components, wires them together via NATS, and serves the HTTP API on port 8000. Code changes trigger automatic rebuilds.

## Using the Web UI

1. **Upload an image** — drag & drop onto the upload area or click to browse. Images larger than 2048px on either axis are automatically downscaled client-side. All formats (JPEG, PNG, WebP, etc.) are converted to PNG before sending.
2. **Browse transforms** — the left sidebar organizes all transforms into 8 collapsible categories: Effects, Convolution, Filters, Monochrome, Channels, Colour Spaces, Transform, and Noise.
3. **Adjust parameters** — selecting a transform reveals sliders or dropdowns for its parameters in the bottom bar. Each has sensible defaults.
4. **Apply** — click the "Apply" button (or press Enter). The image is sent to the server, processed by `task-photon`, and the result appears in the right panel.
5. **Compare** — original and processed images are shown side-by-side. Processing time is displayed in the status bar and the `X-Processing-Info` response header.
6. **Reset** — click "Reset to Original" to clear the processed image.

## API Reference

### `GET /`

Serves the single-page web UI.

### `GET /api/transforms`

Returns the JSON catalog of all available transforms, organized by category. Each transform includes its name, label, and parameter specifications (type, default, min, max). This endpoint drives the UI sidebar.

### `POST /api/transform`

Applies a transform to an image. Uses a binary-framed protocol to avoid base64 overhead:

**Request** (`Content-Type: application/octet-stream`):
```
[4 bytes: JSON header length as u32 big-endian]
[N bytes: JSON header, e.g. {"transform": "effects.oil", "params": {"int_val": 4, "float_val": 55.0}}]
[remaining bytes: PNG image data]
```

**Response** (`Content-Type: image/png`):
- Body: processed PNG image bytes
- Header `X-Processing-Info`: JSON with `{"width", "height", "processing_time_ms"}`

**Example with curl** (using the test tools):
```bash
# Build test tools first
(cd tests/tools && cargo build --release)

# Generate a test image
tests/tools/target/release/mkimage 200 200 /tmp/test.png

# Build a payload and call the API
tests/tools/target/release/mkpayload effects.oil /tmp/test.png int_val=4 float_val=55 \
  | curl -s -X POST http://localhost:8000/api/transform \
    -H 'Content-Type: application/octet-stream' \
    --data-binary @- \
    -o /tmp/result.png

# Validate the result
tests/tools/target/release/check-png /tmp/result.png
```

## Transform Categories

| Category | Count | Examples |
|----------|-------|---------|
| Effects | 24 | Oil painting, solarize, frosted glass, pixelize, halftone, dither |
| Convolution | 17 | Gaussian blur, sharpen, edge detection, emboss, Sobel, Laplace |
| Filters | 15 | Lo-fi, dramatic, golden, oceanic, vintage (+ 15 named presets) |
| Monochrome | 13 | Grayscale, sepia, threshold, decompose max/min |
| Channels | 12 | Invert, alter/remove/swap individual RGB channels |
| Colour Spaces | 22 | Hue rotate, lighten, darken, saturate across HSL/HSV/LCh/HSLuv |
| Transform | 11 | Flip, rotate, resize, crop, shear, seam carve, padding |
| Noise | 2 | Random noise, pink noise |

## Building

```bash
wash build
```

This runs `cargo build --workspace --target wasm32-wasip2 --release` and produces signed Wasm components at `target/wasm32-wasip2/release/`.

To build only one component:
```bash
cargo build --target wasm32-wasip2 --release -p task-photon
cargo build --target wasm32-wasip2 --release -p http-api
```

## Testing

The test suite validates every transform by sending a test image to the running API and checking:
- HTTP 200 response
- Valid PNG output (parsed and decoded)
- Pixel data actually changed (output differs from input)
- All 15 named filter presets work

### Running the tests

Start the server in one terminal, run tests in another:

```bash
# Terminal 1: start the server
wash dev

# Terminal 2: run the test suite
./tests/test_all_transforms.sh
```

### Test options

```bash
# Save all output images for visual inspection
./tests/test_all_transforms.sh --save

# Test only transforms matching a pattern
./tests/test_all_transforms.sh --filter oil
./tests/test_all_transforms.sh --filter conv

# Use a different image size (default: 200x200)
./tests/test_all_transforms.sh --size 400

# Use your own image
./tests/test_all_transforms.sh --image photo.png

# Point at a different server
./tests/test_all_transforms.sh --url http://localhost:9000
```

Output images (when using `--save`) are written to `tests/output/`.

### Test tools

The test scripts use three small Rust CLI helpers in `tests/tools/`. They are built automatically on first test run, or you can build them manually:

```bash
(cd tests/tools && cargo build --release)
```

| Tool | Purpose |
|------|---------|
| `mkimage <W> <H> [out.png]` | Generate a gradient PNG test image |
| `mkpayload <transform> <image.png> [key=val ...]` | Build a binary-framed API request payload |
| `check-png <file.png>` | Validate a PNG and print `width\theight\tbytes` |

## Benchmarking

The benchmark framework measures both server-side processing time (reported by the worker) and end-to-end round-trip time (including HTTP + NATS overhead) for each transform.

### Running benchmarks

```bash
# Terminal 1
wash dev

# Terminal 2: quick benchmark (3 iterations, 100px image)
./tests/benchmark.sh
```

### Benchmark options

```bash
# More iterations for stable medians
./tests/benchmark.sh --iterations 5

# Test scaling behavior across image sizes
./tests/benchmark.sh --sizes 50,100,200,400

# Benchmark a single transform with many iterations
./tests/benchmark.sh --filter effects.oil --iterations 20

# Benchmark one category
./tests/benchmark.sh --category Convolution

# Export results to CSV
./tests/benchmark.sh --csv results.csv

# Use a real photo instead of generated gradients
./tests/benchmark.sh --image photo.png
```

### Comparing runs

Save a baseline, make changes, then compare:

```bash
# Save baseline
./tests/benchmark.sh --csv baseline.csv

# ... make code changes, rebuild ...

# Compare against baseline (flags >20% regressions)
./tests/benchmark.sh --csv current.csv --compare baseline.csv
```

The comparison report flags transforms that got >20% slower (regressions) or >20% faster (improvements).

### CSV output format

```
transform,category,image_size,input_bytes,output_bytes,server_median_ms,roundtrip_median_ms,iterations
effects.oil,Effects,200x200,61914,37811,117,141,3
```

### Environment variable

Both scripts respect `PHOTON_URL` to override the default server address:

```bash
export PHOTON_URL=http://localhost:9000
./tests/test_all_transforms.sh
./tests/benchmark.sh
```
