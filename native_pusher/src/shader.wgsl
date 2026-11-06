struct CameraUniform {
    view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    light_dir: vec4<f32>,
    time_fever: vec4<f32>, // x: time, y: is_fever, z: fever_meter (0..1), w: combo
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var shadow_texture: texture_depth_2d;

@group(1) @binding(1)
var shadow_sampler: sampler_comparison;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct InstanceInput {
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,
    @location(6) color: vec4<f32>,
    @location(7) extra: vec4<f32>, // [metallic, roughness, emission, scale]
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) extra: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    // Exact scale-invariant inverse-transpose normal calculation
    let col0 = instance.model_matrix_0.xyz;
    let col1 = instance.model_matrix_1.xyz;
    let col2 = instance.model_matrix_2.xyz;

    let inv_scale_sq = 1.0 / max(
        vec3<f32>(dot(col0, col0), dot(col1, col1), dot(col2, col2)),
        vec3<f32>(0.0001, 0.0001, 0.0001)
    );

    let normal_matrix = mat3x3<f32>(col0, col1, col2);
    let world_normal = normalize(normal_matrix * (model.normal * inv_scale_sq));

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_pos = world_position.xyz;
    out.world_normal = world_normal;
    out.color = instance.color;
    out.extra = instance.extra;
    return out;
}

@vertex
fn vs_shadow(
    model: VertexInput,
    instance: InstanceInput,
) -> @builtin(position) vec4<f32> {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let world_position = model_matrix * vec4<f32>(model.position, 1.0);
    return camera.light_view_proj * world_position;
}

fn fetch_shadow(world_pos: vec3<f32>, normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    let shadow_clip = camera.light_view_proj * vec4<f32>(world_pos, 1.0);
    let proj_coords = shadow_clip.xyz / shadow_clip.w;

    let shadow_uv = vec2<f32>(
        proj_coords.x * 0.5 + 0.5,
        proj_coords.y * -0.5 + 0.5
    );

    let in_bounds = shadow_uv.x >= 0.0 && shadow_uv.x <= 1.0 &&
                    shadow_uv.y >= 0.0 && shadow_uv.y <= 1.0 &&
                    proj_coords.z >= 0.0 && proj_coords.z <= 1.0;

    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let bias = max(0.0032 * (1.0 - n_dot_l), 0.0008);
    let compare_depth = proj_coords.z - bias;

    let clamped_uv = clamp(shadow_uv, vec2<f32>(0.001), vec2<f32>(0.999));
    let texel_size = 1.0 / 2048.0;
    var shadow_acc = 0.0;

    // 3x3 PCF filter for soft shadow penumbra
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow_acc += textureSampleCompare(
                shadow_texture,
                shadow_sampler,
                clamped_uv + offset,
                compare_depth
            );
        }
    }
    let pcf_shadow = shadow_acc / 9.0;
    return select(1.0, pcf_shadow, in_bounds);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(camera.cam_pos.xyz - in.world_pos);
    let N = in.world_normal;
    let metallic = in.extra.x;
    let roughness = max(in.extra.y, 0.04);
    let emission = in.extra.z;

    // 1. Key Directional Shadow Light (Synchronized with shadow map)
    let key_dir = camera.light_dir.xyz;
    let key_diff = max(dot(N, key_dir), 0.0);
    let key_half = normalize(key_dir + view_dir);
    let key_spec = pow(max(dot(N, key_half), 0.0), 48.0 / roughness);

    // Compute shadow factor with PCF (0.0 = deep shadow, 1.0 = fully lit)
    let shadow = fetch_shadow(in.world_pos, N, key_dir);
    let shadow_diffuse = mix(0.08, 1.0, shadow);

    let key_color = vec3<f32>(1.0, 0.96, 0.90) * key_diff * 0.90 * shadow_diffuse;
    let spec_tint = mix(vec3<f32>(1.0, 0.98, 0.92), in.color.rgb, metallic);
    let key_spec_color = spec_tint * key_spec * (0.45 + 0.95 * metallic) * shadow;

    // 2. Left Neon Rail Light (Cyan)
    let left_light_pos = vec3<f32>(-3.2, 2.0, 2.5);
    let left_dir = normalize(left_light_pos - in.world_pos);
    let left_diff = max(dot(N, left_dir), 0.0) * 0.35;
    let left_color = vec3<f32>(0.1, 0.85, 1.0) * left_diff;

    // 3. Right Neon Rail Light (Hot Magenta)
    let right_light_pos = vec3<f32>(3.2, 2.0, 2.5);
    let right_dir = normalize(right_light_pos - in.world_pos);
    let right_diff = max(dot(N, right_dir), 0.0) * 0.35;
    let right_color = vec3<f32>(1.0, 0.18, 0.65) * right_diff;

    // 4. Dynamic Rainbow Fever Strobe (When in Fever Mode)
    var fever_light = vec3<f32>(0.0);
    if (camera.time_fever.y > 0.5) {
        let t = camera.time_fever.x * 6.0;
        let rainbow = vec3<f32>(
            sin(t) * 0.5 + 0.5,
            sin(t + 2.094) * 0.5 + 0.5,
            sin(t + 4.188) * 0.5 + 0.5
        );
        let fever_dir = normalize(vec3<f32>(0.0, 1.0, 0.2));
        fever_light = rainbow * max(dot(N, fever_dir), 0.0) * 0.75;
    }

    // 5. Fake Arcade Environment Reflection (Simulates reflective ceiling neon tubes)
    let R = reflect(-view_dir, N);
    let ceiling_refl = pow(max(R.y, 0.0), 3.0) * metallic * 0.55;
    let refl_color = mix(vec3<f32>(0.9, 0.95, 1.0), in.color.rgb, metallic) * ceiling_refl;

    // 6. Fresnel Edge Glow (Shimmers on coin borders)
    let NdotV = max(dot(N, view_dir), 0.0);
    let fresnel = pow(1.0 - NdotV, 3.5);
    let rim_color = vec3<f32>(1.0, 0.85, 0.5) * fresnel * (0.35 + 0.65 * metallic);

    // 7. Base Ambient & Emission
    let ambient = vec3<f32>(0.22, 0.24, 0.28) * (1.0 - metallic * 0.5);
    let emissive = in.color.rgb * emission * 1.6;

    // Combine all lighting passes
    let diffuse_sum = ambient + key_color + left_color + right_color + fever_light;
    let final_rgb = in.color.rgb * diffuse_sum + key_spec_color + refl_color + rim_color + emissive;

    return vec4<f32>(final_rgb, in.color.a);
}
