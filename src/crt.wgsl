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

// Barrel distortion and a small rounded mask reproduce the face of a CRT.
@fragment
fn fragment(input: VertexOut) -> @location(0) vec4<f32> {
    let centered = input.uv * 2.0 - vec2(1.0);
    let warped = centered * (vec2(1.0) + 0.035 * centered.yx * centered.yx);
    let uv = warped * 0.5 + vec2(0.5);
    let corner = abs(warped) - vec2(0.93);
    let distance = length(max(corner, vec2(0.0))) + min(max(corner.x, corner.y), 0.0) - 0.07;
    let screen = 1.0 - smoothstep(-0.004, 0.002, distance);
    let inside = select(0.0, 1.0, all(uv >= vec2(0.0)) && all(uv <= vec2(1.0)));
    let pixel = 1.0 / vec2<f32>(textureDimensions(source));
    let shift = vec2(pixel.x * 0.7, 0.0);
    let center = textureSample(source, source_sampler, uv).rgb;
    let red = textureSample(source, source_sampler, uv + shift).r;
    let blue = textureSample(source, source_sampler, uv - shift).b;
    let glow = (textureSample(source, source_sampler, uv + vec2(pixel.x * 2.0, 0.0)).rgb
        + textureSample(source, source_sampler, uv - vec2(pixel.x * 2.0, 0.0)).rgb
        + textureSample(source, source_sampler, uv + vec2(0.0, pixel.y * 2.0)).rgb
        + textureSample(source, source_sampler, uv - vec2(0.0, pixel.y * 2.0)).rgb) * 0.055;
    let scanline = 0.75 + 0.25 * cos(input.position.y * 1.5707963);
    let stripe = 0.92 + 0.08 * cos(input.position.x * 2.0943951);
    let edge = input.uv * (vec2(1.0) - input.uv);
    let vignette = clamp(pow(16.0 * edge.x * edge.y, 0.18), 0.55, 1.0);
    let signal = vec3(red, center.g, blue) + glow;
    let bright = pow(max(signal, vec3(0.0)), vec3(0.92)) * 1.18;
    return vec4(bright * scanline * stripe * vignette * screen * inside, 1.0);
}
