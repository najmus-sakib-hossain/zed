// GPU Compute Shader for Video Frame Processing
// Performs parallel image operations: resize, color correction, dithering

@group(0) @binding(0)
var<storage, read> input_buffer: array<u32>;

@group(0) @binding(1)
var<storage, read_write> output_buffer: array<u32>;

@group(0) @binding(2)
var<uniform> params: ProcessParams;

struct ProcessParams {
    input_width: u32,
    input_height: u32,
    output_width: u32,
    output_height: u32,
    brightness: f32,
    contrast: f32,
}

// Bilinear interpolation for smooth resizing
fn bilinear_sample(x: f32, y: f32) -> vec4<f32> {
    let x0 = u32(floor(x));
    let y0 = u32(floor(y));
    let x1 = min(x0 + 1u, params.input_width - 1u);
    let y1 = min(y0 + 1u, params.input_height - 1u);
    
    let fx = fract(x);
    let fy = fract(y);
    
    let idx00 = y0 * params.input_width + x0;
    let idx10 = y0 * params.input_width + x1;
    let idx01 = y1 * params.input_width + x0;
    let idx11 = y1 * params.input_width + x1;
    
    let c00 = unpack_color(input_buffer[idx00]);
    let c10 = unpack_color(input_buffer[idx10]);
    let c01 = unpack_color(input_buffer[idx01]);
    let c11 = unpack_color(input_buffer[idx11]);
    
    let c0 = mix(c00, c10, fx);
    let c1 = mix(c01, c11, fx);
    
    return mix(c0, c1, fy);
}

// Unpack RGBA from u32
fn unpack_color(packed: u32) -> vec4<f32> {
    let r = f32((packed >> 24u) & 0xFFu) / 255.0;
    let g = f32((packed >> 16u) & 0xFFu) / 255.0;
    let b = f32((packed >> 8u) & 0xFFu) / 255.0;
    let a = f32(packed & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

// Pack RGBA to u32
fn pack_color(color: vec4<f32>) -> u32 {
    let r = u32(clamp(color.r * 255.0, 0.0, 255.0));
    let g = u32(clamp(color.g * 255.0, 0.0, 255.0));
    let b = u32(clamp(color.b * 255.0, 0.0, 255.0));
    let a = u32(clamp(color.a * 255.0, 0.0, 255.0));
    return (r << 24u) | (g << 16u) | (b << 8u) | a;
}

// Apply color correction
fn color_correct(color: vec4<f32>) -> vec4<f32> {
    var result = color;
    
    // Brightness
    result = result + vec4<f32>(params.brightness, params.brightness, params.brightness, 0.0);
    
    // Contrast
    result = (result - 0.5) * params.contrast + 0.5;
    
    return clamp(result, vec4<f32>(0.0), vec4<f32>(1.0));
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= params.output_width || y >= params.output_height) {
        return;
    }
    
    // Calculate source coordinates
    let src_x = f32(x) * f32(params.input_width) / f32(params.output_width);
    let src_y = f32(y) * f32(params.input_height) / f32(params.output_height);
    
    // Sample with bilinear interpolation
    var color = bilinear_sample(src_x, src_y);
    
    // Apply color correction
    color = color_correct(color);
    
    // Write output
    let output_idx = y * params.output_width + x;
    output_buffer[output_idx] = pack_color(color);
}
