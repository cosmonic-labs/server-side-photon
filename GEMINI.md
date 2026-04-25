# Gemini Context: Photon wasmCloud Demo

This project is a reference application demonstrating server-side image processing with WebAssembly using [wasmCloud](https://wasmcloud.com) and [photon-rs](https://github.com/silvia-odwyer/photon).

## Project Overview

The application follows a microservices architecture implemented as Wasm components communicating via NATS:

-   **`http-api`**: An HTTP gateway component. It serves a single-page web UI and exposes REST endpoints. It acts as a producer, forwarding image transformation requests to a NATS subject (`tasks.photon`).
-   **`task-photon`**: A worker component that performs the actual image processing. It consumes messages from NATS, applies the requested `photon-rs` transform, and returns the result.
-   **Communication**: Components use the `wasmcloud:messaging` interface. Data is exchanged using a custom binary-framed protocol: `[4 bytes: header length (BE u32)][N bytes: JSON header][Remaining bytes: PNG data]`.

## Building and Running

The project uses the `wash` (wasmCloud Shell) CLI for orchestration.

-   **Build all components**: `wash build` (runs `cargo build` with `wasm32-wasip2` target).
-   **Start development server**: `wash dev`. This starts a local wasmCloud host, deploys components, and watches for changes.
-   **Web UI**: Accessible at `http://localhost:8000` when running.

## Testing and Benchmarking

-   **Full Test Suite**: `./tests/test_all_transforms.sh` validates all ~130 transforms.
-   **Benchmarking**: `./tests/benchmark.sh` measures server-side and round-trip performance.
-   **Test Tools**: Located in `tests/tools/`.
    -   `mkimage`: Generates test PNGs.
    -   `mkpayload`: Constructs the binary-framed request payloads.
    -   `check-png`: Validates PNG integrity.

## Development Conventions

-   **Component Model**: Targets `wasm32-wasip2`.
-   **WIT**: Interfaces are defined in `wit/world.wit`. `http-api` exports `wasi:http/incoming-handler` (via `wstd`), while `task-photon` exports `wasmcloud:messaging/handler`.
-   **Transform Dispatch**: `task-photon/src/transforms.rs` contains a large `match` statement mapping string names (e.g., `"effects.oil"`) to `photon-rs` function calls.
-   **Dependencies**: Uses `wstd` for standard library-like functionality in a Wasm environment and `wit-bindgen` for interface glue.
-   **UI**: The web UI is a single-file `ui.html` in `http-api/`, embedded into the binary using `include_str!`.
-   **Transform Metadata**: `http-api/transforms.json` defines the available transforms, their categories, and parameters used by both the UI and the test suite.
