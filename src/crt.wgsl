struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> VertexOut {
    let points = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    let point = points[index];
    var out: VertexOut;
    out.position = vec4(point, 0.0, 1.0);
    out.uv = vec2(point.x * 0.5 + 0.5, 0.5 - point.y * 0.5);
    return out;
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var previous: texture_2d<f32>;
@group(0) @binding(3) var glow_sampler: sampler;

fn sample_linear(uv: vec2<f32>) -> vec3<f32> {
    return pow(max(textureSampleLevel(source, source_sampler, uv, 0.0).rgb, vec3(0.0)), vec3(2.2));
}

fn sample_glow(uv: vec2<f32>) -> vec3<f32> {
    return pow(max(textureSampleLevel(source, glow_sampler, uv, 0.0).rgb, vec3(0.0)), vec3(2.2));
}

// Glow follows the brightest phosphors, rather than blurring the whole picture.
fn phosphor_glow(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    let near = sample_glow(uv + vec2(1.5 * pixel.x, 0.0))
        + sample_glow(uv - vec2(1.5 * pixel.x, 0.0))
        + sample_glow(uv + vec2(0.0, 1.5 * pixel.y))
        + sample_glow(uv - vec2(0.0, 1.5 * pixel.y));
    let far = sample_glow(uv + vec2(3.5 * pixel.x, 0.0))
        + sample_glow(uv - vec2(3.5 * pixel.x, 0.0))
        + sample_glow(uv + vec2(0.0, 3.5 * pixel.y))
        + sample_glow(uv - vec2(0.0, 3.5 * pixel.y));
    return max(near * 0.25 - vec3(0.025), vec3(0.0)) * 0.20
        + max(far * 0.25 - vec3(0.08), vec3(0.0)) * 0.055;
}

fn shade(input: VertexOut, vector_display: bool) -> vec4<f32> {
    let centered = input.uv * 2.0 - vec2(1.0);
    let warped = centered * (vec2(1.0) + 0.035 * centered.yx * centered.yx);
    let uv = warped * 0.5 + vec2(0.5);
    let corner = abs(warped) - vec2(0.93);
    let distance = length(max(corner, vec2(0.0))) + min(max(corner.x, corner.y), 0.0) - 0.07;
    let screen = 1.0 - smoothstep(-0.004, 0.002, distance);
    let inside = select(0.0, 1.0, all(uv >= vec2(0.0)) && all(uv <= vec2(1.0)));
    let dimensions = vec2<f32>(textureDimensions(source));
    let pixel = 1.0 / dimensions;

    // Misconvergence increases towards the edges and follows the screen radius.
    let convergence = centered * length(centered) * pixel * 0.55;
    let center = sample_linear(uv);
    let convergence_strength = clamp(length(centered), 0.0, 1.0);
    let red = mix(center.r, sample_glow(uv + convergence).r, convergence_strength);
    let blue = mix(center.b, sample_glow(uv - convergence).b, convergence_strength);
    var signal = vec3(red, center.g, blue);
    let glow = phosphor_glow(uv, pixel);
    var mask = vec3(1.0);
    var beam = 1.0;
    if vector_display {
        // A one-frame phosphor afterglow preserves a short trail on moving traces.
        let old = pow(max(textureSampleLevel(previous, source_sampler, uv, 0.0).rgb, vec3(0.0)), vec3(2.2));
        signal += max(old - vec3(0.04), vec3(0.0)) * 0.14;
        signal += glow * 1.4;
    } else {
        signal += glow;
        // Bright electron beams are wider, filling more of the gap between rows.
        let luminance = dot(center, vec3(0.2126, 0.7152, 0.0722));
        let width = mix(0.34, 0.62, sqrt(clamp(luminance, 0.0, 1.0)));
        let row = abs(fract(uv.y * min(dimensions.y, 240.0)) - 0.5);
        beam = 0.52 + 0.48 * exp(-pow(row / width, 2.0));
        // Aperture grille: adjacent physical columns favor different phosphors.
        let phase = input.position.x * 2.0943951;
        mask = vec3(0.84) + 0.16 * cos(vec3(phase, phase - 2.0943951, phase + 2.0943951));
    }
    let edge = input.uv * (vec2(1.0) - input.uv);
    let vignette = clamp(pow(16.0 * edge.x * edge.y, 0.18), 0.55, 1.0);
    // Soft-clip in linear light before encoding the final signal.
    let exposed = signal * select(1.3, 1.12, vector_display);
    let compressed = vec3(1.0) - exp(-exposed * 1.6);
    let output = pow(max(compressed * mask * beam * vignette, vec3(0.0)), vec3(1.0 / 2.2));
    return vec4(output * screen * inside, 1.0);
}

@fragment
fn raster(input: VertexOut) -> @location(0) vec4<f32> {
    return shade(input, false);
}

@fragment
fn vector(input: VertexOut) -> @location(0) vec4<f32> {
    return shade(input, true);
}
