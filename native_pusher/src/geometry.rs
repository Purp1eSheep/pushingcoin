use bytemuck::{Pod, Zeroable};

pub const COIN_RADIUS: f32 = 0.28;
pub const COIN_HEIGHT: f32 = 0.055;



#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceRaw {
    pub model_matrix: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub extra: [f32; 4], // [metallic, roughness, emission, unused]
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 80,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// Generates a smooth cylinder centered at origin with radius and height
pub fn create_cylinder(radius: f32, height: f32, segments: u32) -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let half_h = height * 0.5;

    // Top center
    let top_center_idx = vertices.len() as u32;
    vertices.push(Vertex {
        position: [0.0, half_h, 0.0],
        normal: [0.0, 1.0, 0.0],
    });

    // Top rim
    let top_rim_start = vertices.len() as u32;
    for i in 0..segments {
        let theta = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = theta.cos() * radius;
        let z = theta.sin() * radius;
        vertices.push(Vertex {
            position: [x, half_h, z],
            normal: [0.0, 1.0, 0.0],
        });
    }

    // Top triangles (both directions to guarantee visibility)
    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(top_center_idx);
        indices.push(top_rim_start + i);
        indices.push(top_rim_start + next);

        indices.push(top_center_idx);
        indices.push(top_rim_start + next);
        indices.push(top_rim_start + i);
    }

    // Bottom center
    let bottom_center_idx = vertices.len() as u32;
    vertices.push(Vertex {
        position: [0.0, -half_h, 0.0],
        normal: [0.0, -1.0, 0.0],
    });

    // Bottom rim
    let bottom_rim_start = vertices.len() as u32;
    for i in 0..segments {
        let theta = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = theta.cos() * radius;
        let z = theta.sin() * radius;
        vertices.push(Vertex {
            position: [x, -half_h, z],
            normal: [0.0, -1.0, 0.0],
        });
    }

    // Bottom triangles
    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(bottom_center_idx);
        indices.push(bottom_rim_start + next);
        indices.push(bottom_rim_start + i);

        indices.push(bottom_center_idx);
        indices.push(bottom_rim_start + i);
        indices.push(bottom_rim_start + next);
    }

    // Side wall
    let side_start = vertices.len() as u32;
    for i in 0..=segments {
        let theta = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = theta.cos() * radius;
        let z = theta.sin() * radius;
        let nx = theta.cos();
        let nz = theta.sin();

        // Top vertex
        vertices.push(Vertex {
            position: [x, half_h, z],
            normal: [nx, 0.0, nz],
        });
        // Bottom vertex
        vertices.push(Vertex {
            position: [x, -half_h, z],
            normal: [nx, 0.0, nz],
        });
    }

    for i in 0..segments {
        let base = side_start + i * 2;
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);

        indices.push(base + 1);
        indices.push(base + 3);
        indices.push(base + 2);

        // Reverse side
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 1);

        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base + 3);
    }

    MeshData { vertices, indices }
}

/// Generates a cuboid centered at origin
pub fn create_box(width: f32, height: f32, depth: f32) -> MeshData {
    let hw = width * 0.5;
    let hh = height * 0.5;
    let hd = depth * 0.5;

    let vertices = vec![
        // Front (+Z)
        Vertex { position: [-hw, -hh,  hd], normal: [ 0.0,  0.0,  1.0] },
        Vertex { position: [ hw, -hh,  hd], normal: [ 0.0,  0.0,  1.0] },
        Vertex { position: [ hw,  hh,  hd], normal: [ 0.0,  0.0,  1.0] },
        Vertex { position: [-hw,  hh,  hd], normal: [ 0.0,  0.0,  1.0] },
        // Back (-Z)
        Vertex { position: [ hw, -hh, -hd], normal: [ 0.0,  0.0, -1.0] },
        Vertex { position: [-hw, -hh, -hd], normal: [ 0.0,  0.0, -1.0] },
        Vertex { position: [-hw,  hh, -hd], normal: [ 0.0,  0.0, -1.0] },
        Vertex { position: [ hw,  hh, -hd], normal: [ 0.0,  0.0, -1.0] },
        // Top (+Y)
        Vertex { position: [-hw,  hh,  hd], normal: [ 0.0,  1.0,  0.0] },
        Vertex { position: [ hw,  hh,  hd], normal: [ 0.0,  1.0,  0.0] },
        Vertex { position: [ hw,  hh, -hd], normal: [ 0.0,  1.0,  0.0] },
        Vertex { position: [-hw,  hh, -hd], normal: [ 0.0,  1.0,  0.0] },
        // Bottom (-Y)
        Vertex { position: [-hw, -hh, -hd], normal: [ 0.0, -1.0,  0.0] },
        Vertex { position: [ hw, -hh, -hd], normal: [ 0.0, -1.0,  0.0] },
        Vertex { position: [ hw, -hh,  hd], normal: [ 0.0, -1.0,  0.0] },
        Vertex { position: [-hw, -hh,  hd], normal: [ 0.0, -1.0,  0.0] },
        // Right (+X)
        Vertex { position: [ hw, -hh,  hd], normal: [ 1.0,  0.0,  0.0] },
        Vertex { position: [ hw, -hh, -hd], normal: [ 1.0,  0.0,  0.0] },
        Vertex { position: [ hw,  hh, -hd], normal: [ 1.0,  0.0,  0.0] },
        Vertex { position: [ hw,  hh,  hd], normal: [ 1.0,  0.0,  0.0] },
        // Left (-X)
        Vertex { position: [-hw, -hh, -hd], normal: [-1.0,  0.0,  0.0] },
        Vertex { position: [-hw, -hh,  hd], normal: [-1.0,  0.0,  0.0] },
        Vertex { position: [-hw,  hh,  hd], normal: [-1.0,  0.0,  0.0] },
        Vertex { position: [-hw,  hh, -hd], normal: [-1.0,  0.0,  0.0] },
    ];

    let mut indices = Vec::new();
    for f in 0..6 {
        let b = f * 4;
        indices.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        indices.extend_from_slice(&[b, b + 2, b + 1, b, b + 3, b + 2]);
    }

    MeshData { vertices, indices }
}

/// Generates a 2D quad for particles, billboards, and UI
pub fn create_quad(width: f32, height: f32) -> MeshData {
    let hw = width * 0.5;
    let hh = height * 0.5;

    let vertices = vec![
        Vertex { position: [-hw, -hh, 0.0], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [ hw, -hh, 0.0], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [ hw,  hh, 0.0], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [-hw,  hh, 0.0], normal: [0.0, 0.0, 1.0] },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2];
    MeshData { vertices, indices }
}

/// Generates a cone centered at origin pointing downward (-Y)
pub fn create_cone(radius: f32, height: f32, segments: u32) -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let half_h = height * 0.5;

    // Apex pointing downward at -half_h
    let apex_idx = vertices.len() as u32;
    vertices.push(Vertex {
        position: [0.0, -half_h, 0.0],
        normal: [0.0, -1.0, 0.0],
    });

    // Base center at +half_h
    let base_center_idx = vertices.len() as u32;
    vertices.push(Vertex {
        position: [0.0, half_h, 0.0],
        normal: [0.0, 1.0, 0.0],
    });

    // Base rim
    let rim_start = vertices.len() as u32;
    for i in 0..segments {
        let theta = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = theta.cos() * radius;
        let z = theta.sin() * radius;
        let nx = theta.cos();
        let nz = theta.sin();
        let len = (nx * nx + 0.25 + nz * nz).sqrt();
        vertices.push(Vertex {
            position: [x, half_h, z],
            normal: [nx / len, 0.5 / len, nz / len],
        });
    }

    // Cone sides
    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(apex_idx);
        indices.push(rim_start + i);
        indices.push(rim_start + next);

        indices.push(apex_idx);
        indices.push(rim_start + next);
        indices.push(rim_start + i);
    }

    // Base cap
    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push(base_center_idx);
        indices.push(rim_start + next);
        indices.push(rim_start + i);

        indices.push(base_center_idx);
        indices.push(rim_start + i);
        indices.push(rim_start + next);
    }

    MeshData { vertices, indices }
}
