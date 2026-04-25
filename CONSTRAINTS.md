# Constraints and Technical Risks

This document outlines identified security vulnerabilities, architectural weaknesses, and operational risks discovered during the code review of the Photon wasmCloud Demo.

## 1. Security Constraints

### 1.1 Resource Exhaustion (DoS)
- **Risk**: The binary-framed protocol (`[4-byte header length][JSON][PNG bytes]`) is parsed without any length validation.
- **Impact**: A malicious actor could send a very large header length value (e.g., `u32::MAX`). The component would attempt a massive memory allocation, potentially crashing the Wasm module or the host.
- **Location**: `http-api/src/lib.rs` and `task-photon/src/lib.rs`.

### 1.2 Memory Pressure
- **Risk**: Per-request operations involve multiple heavy allocations (`to_vec()`, `get_bytes()`).
- **Impact**: Large images (e.g., 4K resolution) may exceed the Wasm memory limit (default is often 4GB, but can be configured lower). Concurrent requests for large images can lead to `OOM` (Out of Memory) failures.
- **Location**: Implementation of `apply_transform` in both components.

### 1.3 Unvalidated Input
- **Risk**: The component assumes the input bytes are a valid PNG without pre-validation of dimensions or metadata.
- **Impact**: Processing "pixel bombs" or extremely high-resolution images can cause long-tail latency or panics within the `photon-rs` library.

## 2. Architectural Weaknesses

### 2.1 Synchronous Request-Response
- **Risk**: `http-api` uses a synchronous `consumer::request` call with a 30-second timeout.
- **Impact**: The HTTP worker is blocked for the duration of the transform. Under high load or during expensive operations (e.g., `effects.oil`), this can exhaust the HTTP handler pool and limit overall system throughput.

### 2.2 Monolithic Worker Design
- **Risk**: All 130+ transforms are compiled into a single `task-photon` component.
- **Impact**: Scaling is "all-or-nothing." You cannot scale computationally expensive categories (like `Convolution`) independently from cheap ones (like `Channels`), leading to inefficient resource utilization.

### 2.3 Direct Error Exposure
- **Risk**: Internal error messages from `task-photon` are passed directly through to the client via HTTP.
- **Impact**: Potential leakage of internal implementation details or library-specific stack traces that are not user-friendly.

## 3. Operational Risks

### 3.1 Parameter Overflow/Underflow
- **Risk**: User-provided `int_val` and `float_val` are cast using `as` (e.g., `as u8`, `as i16`) without range checking against the defaults in `transforms.json`.
- **Impact**: Unexpected visual artifacts or logic errors if values exceed expected bounds.

### 3.2 Deployment Lifecycle
- **Risk**: `wash dev` is excellent for development but lacks the isolation needed for production deployments (e.g., lack of specific resource quotas per component).

## Recommendations

1.  **Implement Bound Checks**: Add a `MAX_HEADER_SIZE` (e.g., 64KB) and `MAX_IMAGE_SIZE` (e.g., 10MB) check in `http-api`.
2.  **Sanitize Parameters**: Validate `TransformParams` against the min/max values defined in `transforms.json` before processing.
3.  **Asynchronous Patterns**: For production use, consider a job-queue pattern where `http-api` returns a `202 Accepted` and the client polls for completion.
4.  **Component Splitting**: Consider splitting `task-photon` into multiple smaller components by category to allow for heterogeneous scaling.
