use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::geometry::{
    create_box, create_cone, create_cylinder, create_quad, InstanceRaw, Vertex, COIN_HEIGHT,
    COIN_RADIUS,
};

#[derive(Debug)]
#[allow(dead_code)]
pub enum RenderError {
    Lost,
    OutOfMemory,
    Other(String),
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    light_view_proj: [[f32; 4]; 4],
    cam_pos: [f32; 4],
    light_dir: [f32; 4],
    time_fever: [f32; 4],
}

pub struct VulkanRenderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: (u32, u32),

    opaque_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    shadow_texture: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    #[allow(dead_code)]
    shadow_sampler: wgpu::Sampler,
    shadow_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::TextureView,

    // Static GPU Meshes
    coin_vbuf: wgpu::Buffer,
    coin_ibuf: wgpu::Buffer,
    coin_idx_count: u32,

    box_vbuf: wgpu::Buffer,
    box_ibuf: wgpu::Buffer,
    box_idx_count: u32,

    quad_vbuf: wgpu::Buffer,
    quad_ibuf: wgpu::Buffer,
    quad_idx_count: u32,

    unit_cyl_vbuf: wgpu::Buffer,
    unit_cyl_ibuf: wgpu::Buffer,
    unit_cyl_idx_count: u32,

    cone_vbuf: wgpu::Buffer,
    cone_ibuf: wgpu::Buffer,
    cone_idx_count: u32,

    // Dynamic GPU Instance Buffers
    coin_instance_buffer: wgpu::Buffer,
    coin_capacity: usize,

    opaque_instance_buffer: wgpu::Buffer,
    opaque_cyl_buffer: wgpu::Buffer,

    transparent_instance_buffer: wgpu::Buffer,
    cone_instance_buffer: wgpu::Buffer,

    particle_instance_buffer: wgpu::Buffer,
    particle_capacity: usize,
}

impl VulkanRenderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        // Explicitly prioritize VULKAN backend
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create Vulkan surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower, // Optimized for integrated Iris Xe GPU
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("Failed to find appropriate Vulkan adapter");

        log::info!("Using Vulkan GPU Adapter: {}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Vulkan Device"),
                ..Default::default()
            })
            .await
            .expect("Failed to create Vulkan device");

        let mut config = surface
            .get_default_config(&adapter, width, height)
            .expect("Failed to get surface default config");
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);

        // Light & Shadow setup
        let light_target = Vec3::new(0.0, 0.4, 1.5);
        let light_dir_vec = Vec3::new(0.22, 0.92, 0.32).normalize();
        let light_pos = light_target + light_dir_vec * 20.0;
        let light_view = glam::camera::rh::view::look_at_mat4(light_pos, light_target, Vec3::Y);
        let ortho_h = 5.4;
        let ortho_w = 4.6;
        let light_proj = glam::camera::rh::proj::directx::orthographic(-ortho_w, ortho_w, -ortho_h, ortho_h, 1.0, 36.0);
        let light_view_proj = light_proj * light_view;

        // Camera Uniform Buffer
        let camera_uniform = CameraUniform {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            light_view_proj: light_view_proj.to_cols_array_2d(),
            cam_pos: [0.0, 11.5, 12.8, 1.0],
            light_dir: [light_dir_vec.x, light_dir_vec.y, light_dir_vec.z, 0.0],
            time_fever: [0.0, 0.0, 0.0, 0.0],
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // 2048x2048 Dedicated Shadow Map (Depth32Float)
        let shadow_texture_size: u32 = 2048;
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Texture"),
            size: wgpu::Extent3d {
                width: shadow_texture_size,
                height: shadow_texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
                label: Some("shadow_bind_group_layout"),
            });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
            label: Some("shadow_bind_group"),
        });

        let depth_texture = Self::create_depth_texture(&device, &config);

        // Load WGSL Shader
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // Shadow Pipeline Layout (Uses only camera uniform for light_view_proj)
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout)],
                immediate_size: 0,
            });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Vulkan Shadow Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                buffers: &[Some(Vertex::desc()), Some(InstanceRaw::desc())],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&shadow_bind_group_layout),
                ],
                immediate_size: 0,
            });

        // 1. OPAQUE Pipeline (Depth write enabled, CullMode None so procedural meshes never have holes)
        let opaque_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Vulkan Opaque Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc()), Some(InstanceRaw::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Disables back-face culling to eliminate any mesh holes!
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // 2. TRANSPARENT Pipeline (Depth write disabled, Alpha blending, LessEqual test)
        // Fixes transparency occlusion bug: coins behind acrylic glass are completely visible!
        let transparent_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Vulkan Transparent Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Vertex::desc()), Some(InstanceRaw::desc())],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(false), // Transparent glass does NOT block depth!
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        // 1. Create Cylinder Mesh for Coins (24 radial segments with dual-direction caps)
        let coin_mesh = create_cylinder(COIN_RADIUS, COIN_HEIGHT, 24);
        let coin_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Coin Vertex Buffer"),
            contents: bytemuck::cast_slice(&coin_mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let coin_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Coin Index Buffer"),
            contents: bytemuck::cast_slice(&coin_mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let coin_idx_count = coin_mesh.indices.len() as u32;

        // 2. Create Box Mesh for Machine Structures
        let box_mesh = create_box(1.0, 1.0, 1.0);
        let box_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Box Vertex Buffer"),
            contents: bytemuck::cast_slice(&box_mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let box_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Box Index Buffer"),
            contents: bytemuck::cast_slice(&box_mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let box_idx_count = box_mesh.indices.len() as u32;

        // 3. Create Quad Mesh for Particles & Billboards
        let quad_mesh = create_quad(1.0, 1.0);
        let quad_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(&quad_mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Index Buffer"),
            contents: bytemuck::cast_slice(&quad_mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let quad_idx_count = quad_mesh.indices.len() as u32;

        // 4. Create Unit Cylinder Mesh for Retro Arcade Apparatuses (Columns, Brackets, Spout)
        let unit_cyl_mesh = create_cylinder(1.0, 1.0, 24);
        let unit_cyl_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Unit Cylinder Vertex Buffer"),
            contents: bytemuck::cast_slice(&unit_cyl_mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let unit_cyl_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Unit Cylinder Index Buffer"),
            contents: bytemuck::cast_slice(&unit_cyl_mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let unit_cyl_idx_count = unit_cyl_mesh.indices.len() as u32;

        // 5. Create Cone Mesh for Spout Dropper Downward Target Arrow
        let cone_mesh = create_cone(1.0, 1.0, 16);
        let cone_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cone Vertex Buffer"),
            contents: bytemuck::cast_slice(&cone_mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let cone_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cone Index Buffer"),
            contents: bytemuck::cast_slice(&cone_mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let cone_idx_count = cone_mesh.indices.len() as u32;

        // Dynamic Instance Buffers
        let coin_capacity = 4096;
        let coin_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Coin Instance Buffer"),
            size: (coin_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let opaque_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Opaque Instance Buffer"),
            size: (256 * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let opaque_cyl_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Opaque Cylinder Buffer"),
            size: (64 * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let transparent_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transparent Instance Buffer"),
            size: (256 * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cone_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cone Instance Buffer"),
            size: (16 * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let particle_capacity = 2048;
        let particle_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Instance Buffer"),
            size: (particle_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size: (width, height),
            opaque_pipeline,
            transparent_pipeline,
            shadow_pipeline,
            camera_buffer,
            camera_bind_group,
            shadow_texture,
            shadow_view,
            shadow_sampler,
            shadow_bind_group,
            depth_texture,
            coin_vbuf,
            coin_ibuf,
            coin_idx_count,
            box_vbuf,
            box_ibuf,
            box_idx_count,
            quad_vbuf,
            quad_ibuf,
            quad_idx_count,
            unit_cyl_vbuf,
            unit_cyl_ibuf,
            unit_cyl_idx_count,
            cone_vbuf,
            cone_ibuf,
            cone_idx_count,
            coin_instance_buffer,
            coin_capacity,
            opaque_instance_buffer,
            opaque_cyl_buffer,
            transparent_instance_buffer,
            cone_instance_buffer,
            particle_instance_buffer,
            particle_capacity,
        }
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size = (width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = Self::create_depth_texture(&self.device, &self.config);
        }
    }

    pub fn render(
        &mut self,
        coin_instances: &[(Mat4, [f32; 4], [f32; 4])],
        pusher_mat: Mat4,
        spout_x: f32,
        side_barriers: bool,
        particle_instances: &[(Mat4, [f32; 4])],
        time: f32,
        is_fever: bool,
        fever_meter: f32,
        combo: u32,
        shake_offset: Vec3,
        cam_eye: Vec3,
        cam_target: Vec3,
        cam_fov: f32,
    ) -> Result<(), RenderError> {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                let (w, h) = self.size;
                self.resize(w, h);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => return Err(RenderError::Lost),
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderError::Other("Validation error".to_string()))
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update Camera Matrix with Screen Shake
        let aspect = self.size.0 as f32 / self.size.1 as f32;
        let final_cam_eye = cam_eye + shake_offset;
        let final_cam_target = cam_target + shake_offset * 0.4;
        let view_mat = glam::camera::rh::view::look_at_mat4(final_cam_eye, final_cam_target, Vec3::Y);
        let proj_mat = glam::camera::rh::proj::directx::perspective(
            cam_fov,
            aspect,
            0.1,
            100.0,
        );
        let view_proj = proj_mat * view_mat;

        // Update Light & Shadow Matrix
        let light_target = Vec3::new(0.0, 0.4, 1.5);
        let light_dir_vec = Vec3::new(0.22, 0.92, 0.32).normalize();
        let light_pos = light_target + light_dir_vec * 20.0;
        let light_view = glam::camera::rh::view::look_at_mat4(light_pos, light_target, Vec3::Y);
        let ortho_h = 5.4;
        let ortho_w = 4.6;
        let light_proj = glam::camera::rh::proj::directx::orthographic(-ortho_w, ortho_w, -ortho_h, ortho_h, 1.0, 36.0);
        let light_view_proj = light_proj * light_view;

        let cam_uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            light_view_proj: light_view_proj.to_cols_array_2d(),
            cam_pos: [final_cam_eye.x, final_cam_eye.y, final_cam_eye.z, 1.0],
            light_dir: [light_dir_vec.x, light_dir_vec.y, light_dir_vec.z, 0.0],
            time_fever: [time, if is_fever { 1.0 } else { 0.0 }, fever_meter / 100.0, combo as f32],
        };
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[cam_uniform]),
        );

        // Upload Coin Instances in 1 single copy
        let coin_raws: Vec<InstanceRaw> = coin_instances
            .iter()
            .map(|(mat, color, extra)| InstanceRaw {
                model_matrix: mat.to_cols_array_2d(),
                color: *color,
                extra: *extra,
            })
            .collect();

        if coin_raws.len() > self.coin_capacity {
            self.coin_capacity = (coin_raws.len() * 2).max(4096);
            self.coin_instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Resized Coin Instance Buffer"),
                size: (self.coin_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if !coin_raws.is_empty() {
            self.queue.write_buffer(
                &self.coin_instance_buffer,
                0,
                bytemuck::cast_slice(&coin_raws),
            );
        }

        // Upload Particle Instances
        let particle_raws: Vec<InstanceRaw> = particle_instances
            .iter()
            .map(|(mat, color)| InstanceRaw {
                model_matrix: mat.to_cols_array_2d(),
                color: *color,
                extra: [0.2, 0.1, 0.9, 0.0], // Sparkling emission
            })
            .collect();

        if particle_raws.len() > self.particle_capacity {
            self.particle_capacity = (particle_raws.len() * 2).max(2048);
            self.particle_instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Resized Particle Buffer"),
                size: (self.particle_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if !particle_raws.is_empty() {
            self.queue.write_buffer(
                &self.particle_instance_buffer,
                0,
                bytemuck::cast_slice(&particle_raws),
            );
        }

        // ==========================================
        // 1. OPAQUE MACHINE APPARATUS GEOMETRY
        // ==========================================
        let mut opaque_raws: Vec<InstanceRaw> = Vec::with_capacity(64);
        let mut opaque_cyl_raws: Vec<InstanceRaw> = Vec::with_capacity(16);

        // (a) Main Lower Deck Base (width 6.4, height 0.4, depth 5.4 at Z=1.5, Y=-0.2)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(6.4, 0.4, 5.4),
                Quat::IDENTITY,
                Vec3::new(0.0, -0.2, 1.5),
            )
            .to_cols_array_2d(),
            color: [0.12, 0.14, 0.20, 1.0],
            extra: [0.8, 0.35, 0.0, 0.0],
        });

        // (b) Polished Mirror Deck Surface Top (Y = 0.005, depth 5.25, Z: -1.20 to 4.05)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(6.36, 0.02, 5.25),
                Quat::IDENTITY,
                Vec3::new(0.0, 0.005, 1.425),
            )
            .to_cols_array_2d(),
            color: [0.86, 0.88, 0.94, 1.0],
            extra: [0.95, 0.10, 0.0, 0.0], // Chrome mirror floor!
        });

        // (c) Flush Golden Front Edge Bevel (Top Y=0.006, depth 0.15, Z: 4.05 to 4.20)
        // Prominent arcade golden bevel where coins slide and teeter before the drop
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(6.40, 0.022, 0.15),
                Quat::IDENTITY,
                Vec3::new(0.0, 0.005, 4.125),
            )
            .to_cols_array_2d(),
            color: [1.0, 0.84, 0.12, 1.0],
            extra: [1.0, 0.15, 0.3, 0.0],
        });

        // (c-lip) Golden Front Edge Bullnose Trim (Prominent golden cliff drop trim along Z=4.20)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(6.40, 0.08, 0.03),
                Quat::IDENTITY,
                Vec3::new(0.0, -0.035, 4.20),
            )
            .to_cols_array_2d(),
            color: [0.96, 0.80, 0.10, 1.0],
            extra: [1.0, 0.15, 0.25, 0.0],
        });

        // (c2) Left Edge Chamfer Accent Strip
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(0.03, 0.04, 5.38),
                Quat::IDENTITY,
                Vec3::new(-3.20, 0.03, 1.50),
            )
            .to_cols_array_2d(),
            color: [0.96, 0.62, 0.07, 1.0],
            extra: [0.0, 0.0, 1.2, 0.0],
        });

        // (c3) Right Edge Chamfer Accent Strip
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(0.03, 0.04, 5.38),
                Quat::IDENTITY,
                Vec3::new(3.20, 0.03, 1.50),
            )
            .to_cols_array_2d(),
            color: [0.96, 0.62, 0.07, 1.0],
            extra: [0.0, 0.0, 1.2, 0.0],
        });

        if side_barriers {
            // (a) Left Solid Mechanical Barrier Plate (Height 0.44, opaque metal)
            opaque_raws.push(InstanceRaw {
                model_matrix: Mat4::from_scale_rotation_translation(
                    Vec3::new(0.06, 0.44, 5.4),
                    Quat::IDENTITY,
                    Vec3::new(-3.24, 0.22, 1.5),
                )
                .to_cols_array_2d(),
                color: [0.25, 0.30, 0.40, 1.0],
                extra: [0.9, 0.2, 0.0, 0.0],
            });

            // (b) Right Solid Mechanical Barrier Plate (Height 0.44, opaque metal)
            opaque_raws.push(InstanceRaw {
                model_matrix: Mat4::from_scale_rotation_translation(
                    Vec3::new(0.06, 0.44, 5.4),
                    Quat::IDENTITY,
                    Vec3::new(3.24, 0.22, 1.5),
                )
                .to_cols_array_2d(),
                color: [0.25, 0.30, 0.40, 1.0],
                extra: [0.9, 0.2, 0.0, 0.0],
            });

            // (c) Left Glowing Cyan Indicator Edge Strip
            opaque_raws.push(InstanceRaw {
                model_matrix: Mat4::from_scale_rotation_translation(
                    Vec3::new(0.08, 0.04, 5.4),
                    Quat::IDENTITY,
                    Vec3::new(-3.24, 0.45, 1.5),
                )
                .to_cols_array_2d(),
                color: [0.0, 0.94, 1.0, 1.0],
                extra: [0.0, 0.0, 2.0, 0.0],
            });

            // (d) Right Glowing Magenta Indicator Edge Strip
            opaque_raws.push(InstanceRaw {
                model_matrix: Mat4::from_scale_rotation_translation(
                    Vec3::new(0.08, 0.04, 5.4),
                    Quat::IDENTITY,
                    Vec3::new(3.24, 0.45, 1.5),
                )
                .to_cols_array_2d(),
                color: [1.0, 0.1, 0.65, 1.0],
                extra: [0.0, 0.0, 2.0, 0.0],
            });
        } else {
            // Retracted Barrier Slots along Side Edges (When barriers are lowered)
            opaque_raws.push(InstanceRaw {
                model_matrix: Mat4::from_scale_rotation_translation(
                    Vec3::new(0.06, 0.02, 5.4),
                    Quat::IDENTITY,
                    Vec3::new(-3.24, 0.01, 1.5),
                )
                .to_cols_array_2d(),
                color: [0.15, 0.18, 0.24, 1.0],
                extra: [0.9, 0.2, 0.0, 0.0],
            });
            opaque_raws.push(InstanceRaw {
                model_matrix: Mat4::from_scale_rotation_translation(
                    Vec3::new(0.06, 0.02, 5.4),
                    Quat::IDENTITY,
                    Vec3::new(3.24, 0.01, 1.5),
                )
                .to_cols_array_2d(),
                color: [0.15, 0.18, 0.24, 1.0],
                extra: [0.9, 0.2, 0.0, 0.0],
            });
        }

        // (d) Slanted Lower Base Cabinet (Arcade Chassis) - strictly behind cliff face at Z <= 4.05
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(7.6, 2.2, 5.6),
                Quat::IDENTITY,
                Vec3::new(0.0, -1.4, 1.25),
            )
            .to_cols_array_2d(),
            color: [0.04, 0.06, 0.10, 1.0],
            extra: [0.7, 0.4, 0.0, 0.0],
        });

        // (e) Cliff Drop Front Face (Vertical drop wall where coins tumble past at Z=4.19)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(6.4, 1.5, 0.04),
                Quat::IDENTITY,
                Vec3::new(0.0, -0.80, 4.19),
            )
            .to_cols_array_2d(),
            color: [0.06, 0.08, 0.12, 1.0],
            extra: [0.7, 0.4, 0.0, 0.0],
        });

        // (f) Front Coin Recovery Chute (Slanted Slide receiving coins under cliff)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(5.8, 0.08, 1.6),
                Quat::from_rotation_x(0.48),
                Vec3::new(0.0, -1.30, 4.80),
            )
            .to_cols_array_2d(),
            color: [0.20, 0.25, 0.33, 1.0],
            extra: [0.9, 0.2, 0.0, 0.0],
        });

        // (g) Winner Coin Bin Lip (Front Barrier of Prize Tray)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(5.8, 0.50, 0.15),
                Quat::IDENTITY,
                Vec3::new(0.0, -1.25, 5.55),
            )
            .to_cols_array_2d(),
            color: [0.88, 0.91, 0.94, 1.0],
            extra: [0.95, 0.1, 0.0, 0.0],
        });

        // (h) Winner Bin Bottom Floor
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(5.8, 0.08, 1.1),
                Quat::IDENTITY,
                Vec3::new(0.0, -1.50, 5.00),
            )
            .to_cols_array_2d(),
            color: [0.10, 0.13, 0.18, 1.0],
            extra: [0.8, 0.3, 0.0, 0.0],
        });

        // (i) Left Side Lost Coin Gutter (Outward Slanted Gutter)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(0.85, 0.08, 5.4),
                Quat::from_rotation_z(-0.4),
                Vec3::new(-3.55, -0.45, 1.5),
            )
            .to_cols_array_2d(),
            color: [0.12, 0.16, 0.23, 1.0],
            extra: [0.8, 0.3, 0.0, 0.0],
        });

        // (j) Right Side Lost Coin Gutter (Outward Slanted Gutter)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(0.85, 0.08, 5.4),
                Quat::from_rotation_z(0.4),
                Vec3::new(3.55, -0.45, 1.5),
            )
            .to_cols_array_2d(),
            color: [0.12, 0.16, 0.23, 1.0],
            extra: [0.8, 0.3, 0.0, 0.0],
        });

        // (k) Pusher Block (Taller pusher platform height Y=0.88, aligned with physics)
        opaque_raws.push(InstanceRaw {
            model_matrix: (pusher_mat
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(5.40, 0.88, 3.4),
                    Quat::IDENTITY,
                    Vec3::new(0.0, 0.0, -0.6),
                ))
            .to_cols_array_2d(),
            color: [0.20, 0.25, 0.35, 1.0],
            extra: [0.7, 0.4, 0.0, 0.0],
        });

        // Pusher Top Chrome Plate
        opaque_raws.push(InstanceRaw {
            model_matrix: (pusher_mat
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(5.36, 0.03, 3.36),
                    Quat::IDENTITY,
                    Vec3::new(0.0, 0.44, -0.6),
                ))
            .to_cols_array_2d(),
            color: [0.92, 0.94, 0.98, 1.0],
            extra: [0.98, 0.15, 0.0, 0.0],
        });

        // Pusher Front Edge Gold Strip
        opaque_raws.push(InstanceRaw {
            model_matrix: (pusher_mat
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(5.38, 0.12, 0.06),
                    Quat::IDENTITY,
                    Vec3::new(0.0, 0.22, 1.10),
                ))
            .to_cols_array_2d(),
            color: [1.0, 0.85, 0.2, 1.0],
            extra: [1.0, 0.1, 0.4, 0.0],
        });

        // Pusher Chrome Front Blade
        opaque_raws.push(InstanceRaw {
            model_matrix: (pusher_mat
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(5.40, 0.86, 0.04),
                    Quat::IDENTITY,
                    Vec3::new(0.0, 0.0, 1.105),
                ))
            .to_cols_array_2d(),
            color: [0.97, 0.98, 1.0, 1.0],
            extra: [0.98, 0.08, 0.0, 0.0],
        });

        // (l) Scraper Barrier Wall (Z = -1.1, zero-gap with raised pusher top)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(5.5, 1.30, 0.16),
                Quat::IDENTITY,
                Vec3::new(0.0, 1.48, -1.1),
            )
            .to_cols_array_2d(),
            color: [0.08, 0.10, 0.15, 1.0],
            extra: [0.6, 0.4, 0.0, 0.0],
        });

        // Left Upper Side Guard (Encloses rear compartment behind scraper wall)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(0.16, 1.30, 2.70),
                Quat::IDENTITY,
                Vec3::new(-2.74, 1.48, -2.45),
            )
            .to_cols_array_2d(),
            color: [0.12, 0.16, 0.23, 1.0],
            extra: [0.85, 0.2, 0.0, 0.0],
        });

        // Right Upper Side Guard (Encloses rear compartment behind scraper wall)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(0.16, 1.30, 2.70),
                Quat::IDENTITY,
                Vec3::new(2.74, 1.48, -2.45),
            )
            .to_cols_array_2d(),
            color: [0.12, 0.16, 0.23, 1.0],
            extra: [0.85, 0.2, 0.0, 0.0],
        });

        // (m) Rear Machinery Roof (Z = -2.45, Y = 2.05)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(5.5, 0.12, 2.7),
                Quat::IDENTITY,
                Vec3::new(0.0, 2.05, -2.45),
            )
            .to_cols_array_2d(),
            color: [0.10, 0.13, 0.18, 1.0],
            extra: [0.7, 0.5, 0.0, 0.0],
        });

        // (n) Back Mirror Wall Main Chassis (Z = -3.8)
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(7.6, 5.2, 0.4),
                Quat::IDENTITY,
                Vec3::new(0.0, 2.6, -3.8),
            )
            .to_cols_array_2d(),
            color: [0.04, 0.06, 0.10, 1.0],
            extra: [0.85, 0.2, 0.0, 0.0],
        });

        // Reflective Inner Mirror Surface Plate
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(7.4, 4.8, 0.04),
                Quat::IDENTITY,
                Vec3::new(0.0, 2.6, -3.59),
            )
            .to_cols_array_2d(),
            color: [0.12, 0.11, 0.29, 1.0],
            extra: [0.95, 0.08, 0.0, 0.0],
        });

        // (p) Spout Shooter Rails & Slider
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(5.2, 0.08, 0.08),
                Quat::IDENTITY,
                Vec3::new(0.0, 3.8, -0.75),
            )
            .to_cols_array_2d(),
            color: [0.39, 0.45, 0.55, 1.0],
            extra: [0.9, 0.2, 0.0, 0.0],
        });

        // Moving Spout Slider Chrome Bracket
        opaque_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(0.5, 0.16, 0.3),
                Quat::IDENTITY,
                Vec3::new(spout_x, 3.6, -0.75),
            )
            .to_cols_array_2d(),
            color: [0.97, 0.98, 1.0, 1.0],
            extra: [0.95, 0.1, 0.0, 0.0],
        });

        // (q) Opaque Cylinders: Spout Entry Funnel
        // Spout Golden Entry Funnel
        opaque_cyl_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(0.22, 0.24, 0.22),
                Quat::IDENTITY,
                Vec3::new(spout_x, 3.44, -0.75),
            )
            .to_cols_array_2d(),
            color: if is_fever {
                [1.0, 0.2, 0.6, 1.0]
            } else {
                [0.98, 0.75, 0.14, 1.0]
            },
            extra: [0.9, 0.15, if is_fever { 1.5 } else { 0.5 }, 0.0],
        });

        self.queue.write_buffer(
            &self.opaque_instance_buffer,
            0,
            bytemuck::cast_slice(&opaque_raws),
        );
        self.queue.write_buffer(
            &self.opaque_cyl_buffer,
            0,
            bytemuck::cast_slice(&opaque_cyl_raws),
        );

        // ==========================================
        // 2. TRANSPARENT & NEON GLOW GEOMETRY
        // ==========================================
        let mut trans_raws: Vec<InstanceRaw> = Vec::with_capacity(64);
        let mut cone_raws: Vec<InstanceRaw> = Vec::with_capacity(4);

        // (g) Winner Coin Bin Glowing Emerald Green Accent Strip ("WINNER" lip)
        trans_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(4.2, 0.08, 0.04),
                Quat::IDENTITY,
                Vec3::new(0.0, -1.00, 5.56),
            )
            .to_cols_array_2d(),
            color: [0.06, 0.73, 0.51, 0.95],
            extra: [0.0, 0.0, 2.8, 0.0], // Emerald Green Glow!
        });

        // (h) Scraper Wall Glowing Pink Neon Accent Strip
        trans_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(5.4, 0.06, 0.04),
                Quat::IDENTITY,
                Vec3::new(0.0, 1.98, -1.01),
            )
            .to_cols_array_2d(),
            color: [0.92, 0.28, 0.6, 0.95],
            extra: [0.0, 0.0, 2.5, 0.0],
        });

        // (i) Pusher Moving Blade Cyan Arcade Neon Bar
        trans_raws.push(InstanceRaw {
            model_matrix: (pusher_mat
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(5.2, 0.05, 0.04),
                    Quat::IDENTITY,
                    Vec3::new(0.0, 0.30, 1.13),
                ))
            .to_cols_array_2d(),
            color: [0.02, 0.71, 0.83, 0.95],
            extra: [0.0, 0.0, 2.8, 0.0],
        });

        // (j) Pusher Moving Blade Bottom Amber Skid Strip
        trans_raws.push(InstanceRaw {
            model_matrix: (pusher_mat
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(5.2, 0.04, 0.04),
                    Quat::IDENTITY,
                    Vec3::new(0.0, -0.30, 1.13),
                ))
            .to_cols_array_2d(),
            color: [0.96, 0.62, 0.07, 0.95],
            extra: [0.0, 0.0, 1.8, 0.0],
        });

        // (k) Spout Overhead Rail Neon Backing Strip
        trans_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(4.8, 0.02, 0.06),
                Quat::IDENTITY,
                Vec3::new(0.0, 3.85, -0.75),
            )
            .to_cols_array_2d(),
            color: [0.22, 0.74, 0.97, 0.95],
            extra: [0.0, 0.0, 2.2, 0.0],
        });

        // (l) Moving Backboard Chase Lights
        let chase_phase = (time * 8.0).sin() * 0.5 + 0.5;
        let chase_color = if is_fever {
            [1.0, 0.2, 0.8, 0.95]
        } else {
            [1.0, 0.85, 0.2, 0.8 + chase_phase * 0.2]
        };
        trans_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(5.3, 0.08, 0.08),
                Quat::IDENTITY,
                Vec3::new(0.0, 2.0, -1.05),
            )
            .to_cols_array_2d(),
            color: chase_color,
            extra: [0.0, 0.0, 2.2, 0.0],
        });

        // (m) Retro Neon Grid Lines on Back Mirror Wall
        // Horizontal Cyan Lines
        for &y in &[0.8, 1.8, 2.8, 3.8] {
            trans_raws.push(InstanceRaw {
                model_matrix: Mat4::from_scale_rotation_translation(
                    Vec3::new(7.2, 0.035, 0.02),
                    Quat::IDENTITY,
                    Vec3::new(0.0, y, -3.56),
                )
                .to_cols_array_2d(),
                color: [0.22, 0.74, 0.97, 0.9],
                extra: [0.0, 0.0, 2.0, 0.0],
            });
        }
        // Vertical Magenta/Purple Lines
        for &x in &[-2.5, -1.25, 0.0, 1.25, 2.5] {
            trans_raws.push(InstanceRaw {
                model_matrix: Mat4::from_scale_rotation_translation(
                    Vec3::new(0.035, 4.4, 0.02),
                    Quat::IDENTITY,
                    Vec3::new(x, 2.6, -3.56),
                )
                .to_cols_array_2d(),
                color: [0.66, 0.33, 0.97, 0.9],
                extra: [0.0, 0.0, 2.0, 0.0],
            });
        }

        // (o) Spout Downward Aim Indicator Cone (Pointing down)
        let aim_pulse = (time * 6.0).sin() * 0.15 + 0.85;
        let aim_color = if is_fever {
            [1.0, 0.2, 0.6, 1.0]
        } else {
            [0.94, 0.27, 0.27, 1.0]
        };
        cone_raws.push(InstanceRaw {
            model_matrix: Mat4::from_scale_rotation_translation(
                Vec3::new(0.10 * aim_pulse, 0.22, 0.10 * aim_pulse),
                Quat::IDENTITY,
                Vec3::new(spout_x, 3.20, -0.63),
            )
            .to_cols_array_2d(),
            color: aim_color,
            extra: [0.0, 0.0, 3.5, 0.0],
        });

        self.queue.write_buffer(
            &self.transparent_instance_buffer,
            0,
            bytemuck::cast_slice(&trans_raws),
        );
        self.queue.write_buffer(
            &self.cone_instance_buffer,
            0,
            bytemuck::cast_slice(&cone_raws),
        );

        // ==========================================
        // 3. RECORD COMMAND ENCODER & SUBMIT
        // ==========================================
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Vulkan Render Encoder"),
            });

        // ------------------------------------------
        // PASS 0: SHADOW PASS (Depth-Only from Light Source)
        // ------------------------------------------
        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Vulkan Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            // Draw Opaque Boxes (Machine Chassis, Decks, Walls, Pusher)
            shadow_pass.set_vertex_buffer(0, self.box_vbuf.slice(..));
            shadow_pass.set_vertex_buffer(1, self.opaque_instance_buffer.slice(..));
            shadow_pass.set_index_buffer(self.box_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            shadow_pass.draw_indexed(0..self.box_idx_count, 0, 0..opaque_raws.len() as u32);

            // Draw Opaque Cylinders (Chrome Brackets, Spout Funnel)
            shadow_pass.set_vertex_buffer(0, self.unit_cyl_vbuf.slice(..));
            shadow_pass.set_vertex_buffer(1, self.opaque_cyl_buffer.slice(..));
            shadow_pass.set_index_buffer(self.unit_cyl_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            shadow_pass.draw_indexed(0..self.unit_cyl_idx_count, 0, 0..opaque_cyl_raws.len() as u32);

            // Draw All Coins (Casts realistic contact and stacking shadows)
            if !coin_raws.is_empty() {
                shadow_pass.set_vertex_buffer(0, self.coin_vbuf.slice(..));
                shadow_pass.set_vertex_buffer(1, self.coin_instance_buffer.slice(..));
                shadow_pass.set_index_buffer(self.coin_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..self.coin_idx_count, 0, 0..coin_raws.len() as u32);
            }
        }

        // ------------------------------------------
        // PASS 1: MAIN COLOR PASS
        // ------------------------------------------
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Vulkan Main Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.03,
                            b: 0.06,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // ------------------------------------------
            // SUBPASS 1: OPAQUE PASS (Full Depth Write + Dynamic Shadows)
            // ------------------------------------------
            render_pass.set_pipeline(&self.opaque_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.shadow_bind_group, &[]);

            // Draw Opaque Boxes (Chassis, Decks, Walls, Sliders)
            render_pass.set_vertex_buffer(0, self.box_vbuf.slice(..));
            render_pass.set_vertex_buffer(1, self.opaque_instance_buffer.slice(..));
            render_pass.set_index_buffer(self.box_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.box_idx_count, 0, 0..opaque_raws.len() as u32);

            // Draw Opaque Cylinders (Chrome Brackets, Spout Funnel)
            render_pass.set_vertex_buffer(0, self.unit_cyl_vbuf.slice(..));
            render_pass.set_vertex_buffer(1, self.opaque_cyl_buffer.slice(..));
            render_pass.set_index_buffer(self.unit_cyl_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.unit_cyl_idx_count, 0, 0..opaque_cyl_raws.len() as u32);

            // Draw ALL Coins (Draw Call - The 1-Draw-Call Miracle)
            if !coin_raws.is_empty() {
                render_pass.set_vertex_buffer(0, self.coin_vbuf.slice(..));
                render_pass.set_vertex_buffer(1, self.coin_instance_buffer.slice(..));
                render_pass.set_index_buffer(self.coin_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.coin_idx_count, 0, 0..coin_raws.len() as u32);
            }

            // ------------------------------------------
            // SUBPASS 2: TRANSPARENT & GLOW PASS (Depth Write Off)
            // ------------------------------------------
            render_pass.set_pipeline(&self.transparent_pipeline);

            // Draw Neon Edge Tubes & Retro Grids
            render_pass.set_vertex_buffer(0, self.box_vbuf.slice(..));
            render_pass.set_vertex_buffer(1, self.transparent_instance_buffer.slice(..));
            render_pass.set_index_buffer(self.box_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.box_idx_count, 0, 0..trans_raws.len() as u32);

            // Draw Spout Aim Indicator Cone
            render_pass.set_vertex_buffer(0, self.cone_vbuf.slice(..));
            render_pass.set_vertex_buffer(1, self.cone_instance_buffer.slice(..));
            render_pass.set_index_buffer(self.cone_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.cone_idx_count, 0, 0..cone_raws.len() as u32);

            // Draw Celebration Particles
            if !particle_raws.is_empty() {
                render_pass.set_vertex_buffer(0, self.quad_vbuf.slice(..));
                render_pass.set_vertex_buffer(1, self.particle_instance_buffer.slice(..));
                render_pass.set_index_buffer(self.quad_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.quad_idx_count, 0, 0..particle_raws.len() as u32);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_uniform_layout() {
        assert_eq!(std::mem::size_of::<CameraUniform>(), 176);
        assert_eq!(std::mem::align_of::<CameraUniform>(), 16);
    }

    #[test]
    fn test_shader_compilation() {
        let shader_source = include_str!("shader.wgsl");
        let module = wgpu::naga::front::wgsl::parse_str(shader_source);
        assert!(module.is_ok(), "Shader WGSL failed to parse: {:?}", module.err());
    }
}

