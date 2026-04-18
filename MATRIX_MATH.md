# Matrix Math in WebAssembly: How Photon Processes Images

This document explains the categories of matrix and linear algebra operations that photon-rs performs on images inside a WebAssembly component. Every image is represented as a matrix of RGBA pixel values — a `W × H × 4` array of bytes — and every transform is a mathematical operation on that matrix.

## Image as a Matrix

A digital image is a 2D matrix where each element is a pixel with four channels:

```
Image[y][x] = (R, G, B, A)    where R, G, B, A ∈ [0, 255]
```

For a 200×150 image, this is a 150×200 matrix with 120,000 elements, each storing 4 bytes. photon-rs stores this as a flat `Vec<u8>` of raw RGBA pixels, with pixel `(x, y)` at byte offset `(y * width + x) * 4`.

Every transform in this demo reads that byte array, applies mathematical operations to it, and writes back the result — all inside a Wasm component running on wasmCloud.

---

## 1. Convolution Kernels

Convolution is the core operation behind blurring, sharpening, edge detection, and embossing. It works by sliding a small matrix (the **kernel**) across the image and computing a weighted sum of each pixel's neighborhood.

For a 3×3 kernel `K`, the output pixel at position `(x, y)` is:

```
Output[y][x] = Σ Σ Input[y+j][x+i] × K[j][i]     for i,j ∈ {-1, 0, 1}
```

This is a **matrix dot product** between the kernel and the local patch of the image, computed for every pixel. For an image with `N` pixels and a 3×3 kernel, this requires `9N` multiply-accumulate operations per channel.

### Specific kernels used in photon-rs

**Sharpen** (`conv.sharpen`) — enhances edges by subtracting a blurred version from the original:
```
 0  -1   0
-1   5  -1
 0  -1   0
```
The center weight of 5 amplifies the current pixel while the negative neighbors subtract the local average, increasing local contrast.

**Edge Detection** (`conv.edge_detection`) — produces a Laplacian-like response that highlights boundaries:
```
-1  -1  -1
-1   8  -1
-1  -1  -1
```
This kernel sums to zero: uniform regions produce zero output, while edges produce large responses. The result is a map of intensity gradients in all directions.

**Sobel Horizontal** (`conv.sobel_horizontal`) — detects horizontal edges using a first-derivative approximation:
```
-1  -2  -1
 0   0   0
 1   2   1
```
This is a separable filter: it is equivalent to smoothing horizontally `[1, 2, 1]` then differencing vertically `[-1, 0, 1]`. The output magnitude approximates `∂I/∂y`.

**Emboss** (`conv.emboss`) — creates a 3D relief effect by computing a directional gradient:
```
-2  -1   0
-1   1   1
 0   1   2
```
The asymmetric weights simulate light coming from the top-left, making edges appear raised or sunken.

**Box Blur** (`conv.box_blur`) — the simplest spatial low-pass filter:
```
1  1  1
1  1  1
1  1  1
```
Each output pixel is the unweighted mean of its 3×3 neighborhood. This is equivalent to convolution with a uniform kernel, a basic form of spatial averaging.

**Gaussian Blur** (`conv.gaussian_blur`) — uses a multi-pass box blur approximation to simulate a Gaussian kernel. For larger radii, photon-rs implements the linear-time algorithm from Kovacs (three sequential box blur passes approximate a Gaussian), operating as separable 1D horizontal and vertical passes over the pixel buffer.

---

## 2. Element-wise Scalar Arithmetic

Many transforms operate independently on each pixel — they iterate the flat pixel array and apply arithmetic operations to individual R, G, or B values. These are element-wise operations on the image matrix with no spatial dependencies between pixels.

### Brightness (`effects.inc_brightness`, `effects.dec_brightness`)

Adds or subtracts a constant from every channel of every pixel, clamped to `[0, 255]`:

```
Output[i] = clamp(Input[i] + brightness, 0, 255)    for each R, G, B channel
```

This is **scalar addition** on the image matrix — the simplest possible pixel operation.

### Contrast (`effects.adjust_contrast`)

Applies an affine transformation to each channel using a precomputed lookup table:

```
factor = (259 × (contrast + 255)) / (255 × (259 - contrast))
Output[c] = clamp(Input[c] × factor - 128 × factor + 128, 0, 255)
```

This is a **multiply-add** per channel. The lookup table optimization reduces it to a single table lookup per channel value, making it `O(N)` with a constant factor close to a memory copy.

### Invert (`channels.invert`)

Subtracts each channel from 255:

```
Output[i] = 255 - Input[i]    for each R, G, B channel
```

This is a **bitwise complement** on the color channels — equivalent to XOR with 0xFF.

### Solarize (`effects.solarize`)

Conditionally inverts the red channel based on a threshold:

```
if (200 - R) > 0:
    Output.R = 200 - R
```

This is an element-wise **conditional subtraction** — a non-linear point operation applied only to pixels below a threshold.

---

## 3. Color Space Transformations

Color space operations convert each pixel from RGB to an alternate representation (HSL, HSV, LCh, HSLuv), modify components in that space, then convert back. Each conversion involves non-trivial floating-point math per pixel.

### HSL Hue Rotation (`colour_spaces.hue_rotate_hsl`)

For each pixel:
1. **Convert** `(R, G, B)` → normalize to `[0, 1]` → transform to `(H, S, L)` via the standard hexagonal model
2. **Rotate**: `H' = H + degrees × 360`
3. **Convert back** `(H', S, L)` → `(R, G, B)` → scale to `[0, 255]`

The RGB-to-HSL conversion involves computing `min(R,G,B)`, `max(R,G,B)`, chroma, and a piecewise hue calculation. The inverse requires mapping the hue sector back through a piecewise linear function. Per pixel, this is approximately 15-20 floating-point operations.

### Luminance-based Grayscale (`monochrome.grayscale_human_corrected`)

Applies a **weighted dot product** to each pixel's RGB vector using coefficients that match human perception (ITU-R BT.601):

```
Gray = 0.30 × R + 0.59 × G + 0.11 × B
Output = (Gray, Gray, Gray)
```

This is a **matrix-vector multiplication** where every pixel's `[R, G, B]` vector is projected onto the luminance vector `[0.30, 0.59, 0.11]`.

### Sepia Tone (`monochrome.sepia`)

Computes a weighted average (same luminance projection), then applies channel-specific offsets:

```
Lum = 0.3R + 0.59G + 0.11B
Output = (min(Lum + 100, 255), min(Lum + 50, 255), Lum)
```

This is a **projection followed by translation** — the luminance calculation is a dot product, and the sepia tint is a constant vector addition `(+100, +50, 0)` in the output space.

### Saturation and Lightness (`colour_spaces.saturate_hsl`, `lighten_hsl`, `darken_hsl`)

These operate in HSL space: convert, scale the S or L component by a factor, convert back. The saturation adjustment is a **scalar multiply** on a single component of the color vector in a transformed coordinate system.

---

## 4. Affine and Geometric Transformations

Geometric transforms change the spatial layout of the image matrix by computing new pixel coordinates through matrix multiplication.

### Flip (`transform.fliph`)

Mirrors the image horizontally by reversing the x-coordinate:

```
Output[y][x] = Input[y][width - 1 - x]
```

This is a **reflection matrix** applied to pixel coordinates: `[x'] = [-1, 0; 0, 1] × [x, y]^T + [width-1, 0]^T`.

### Resize (`transform.resize`)

Resamples the image to new dimensions using an interpolation filter. With Lanczos3 resampling, each output pixel is computed from a weighted sum of a 6×6 neighborhood of source pixels, where the weights come from the sinc-windowed Lanczos kernel:

```
L(x) = sinc(x) × sinc(x/3)    for |x| < 3
```

This is **2D interpolation** — equivalent to applying a continuous convolution kernel at non-integer sample positions. The coordinate mapping is a **scaling transformation**: `src_x = x × (src_width / dst_width)`.

### Rotation (`transform.rotate`)

Applies a rotation matrix to map output coordinates to source coordinates:

```
[src_x]   [cos θ   sin θ] [x - cx]   [cx]
[src_y] = [-sin θ  cos θ] [y - cy] + [cy]
```

where `(cx, cy)` is the image center. This is a standard **2D affine transformation** with bilinear interpolation for sub-pixel source positions.

### Shear (`transform.shearx`, `transform.sheary`)

Applies a shear matrix that skews the image along one axis:

```
Shear X: [x'] = [1, tan(θ); 0, 1] × [x, y]^T
Shear Y: [x'] = [1, 0; tan(θ), 1] × [x, y]^T
```

These are **non-orthogonal linear transforms** — the columns of the transformation matrix are no longer perpendicular, producing the characteristic parallelogram distortion.

---

## 5. Nonlinear and Stochastic Operations

Some transforms apply nonlinear or stochastic operations that don't fit neatly into the linear algebra categories above.

### Oil Painting (`effects.oil`)

For each pixel, examines a neighborhood of radius `r`, bins pixel intensities into buckets, finds the most common intensity bin, and averages the colors of pixels in that bin. This is a **nonlinear spatial filter** — it replaces each pixel with a neighborhood mode rather than a weighted sum, producing the characteristic flat-color regions of an oil painting.

### Frosted Glass (`effects.frosted_glass`)

Replaces each pixel with a randomly-selected neighbor within a fixed radius. This is a **stochastic permutation** of the image matrix — each output pixel samples a random input pixel from its local neighborhood.

### Dither (`effects.dither`)

Reduces color depth by quantizing each channel to fewer levels, distributing the quantization error to neighboring pixels (Floyd-Steinberg style). This is an **error-diffusion algorithm** — a causal feedback system where each pixel's rounding error is propagated spatially.

---

## Why This Works in WebAssembly

All of these operations — convolutions, element-wise arithmetic, coordinate transforms, color space conversions — are pure computation over arrays of bytes. They require:

- No filesystem access (images arrive as byte arrays over NATS)
- No GPU or SIMD intrinsics (photon-rs uses scalar Rust code)
- No system calls beyond basic memory allocation

This makes them ideal for the WebAssembly component model. The `task-photon` worker component is a pure function: bytes in, math applied, bytes out. wasmCloud orchestrates the routing (HTTP → NATS → worker → response) while the Wasm runtime provides sandboxed execution of the compute-intensive math.

The benchmark results demonstrate that even computationally heavy transforms like oil painting (nonlinear spatial filtering) and Gaussian blur (multi-pass separable convolution) complete in milliseconds for typical web images, confirming that Wasm's near-native performance is sufficient for real-time image processing workloads.
