struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) opacity: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) opacity: f32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.local = input.local;
    output.opacity = input.opacity;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(input.local);
    if distance > 1.0 {
        discard;
    }
    let edge = 1.0 - smoothstep(0.84, 1.0, distance);
    return vec4<f32>(0.035, 0.04, 0.045, input.opacity * edge);
}
