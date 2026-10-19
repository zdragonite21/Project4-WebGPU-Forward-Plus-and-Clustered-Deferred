use glam::UVec3;

use crate::{
    render::world::RenderWorld,
    shader_constants::{
        BIND_GROUP_CLUSTERING, BIND_GROUP_SCENE, CLUSTER_WG_SIZE, CLUSTER_XYZ,
        MAX_LIGHTS_PER_CLUSTER,
    },
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Cluster {
    min: [f32; 3],
    _pad0: f32,
    max: [f32; 3],
    _pad1: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ClusterLights {
    num_lights: u32,
    lights: [u32; MAX_LIGHTS_PER_CLUSTER as usize],
}

pub struct GpuClusters {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GpuClusters {
    pub fn new(device: &wgpu::Device) -> Self {
        let num_clusters = CLUSTER_XYZ.0 * CLUSTER_XYZ.1 * CLUSTER_XYZ.2;

        let cluster_lights_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Clusters Lights Buffer"),
            mapped_at_creation: false,
            size: num_clusters as u64 * size_of::<ClusterLights>() as u64,
            usage: wgpu::BufferUsages::STORAGE,
        });

        let clusters_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Clusters Buffer"),
            mapped_at_creation: false,
            size: num_clusters as u64 * size_of::<Cluster>() as u64,
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("clustering_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cluster_lights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: clusters_buffer.as_entire_binding(),
                },
            ],
            label: Some("clustering_bind_group"),
        });

        Self {
            bind_group,
            bind_group_layout,
        }
    }
}

pub struct ClusteringComputePass {
    cluster_bounds: wgpu::ComputePipeline,
    assign_lights: wgpu::ComputePipeline,
    pub resources: GpuClusters,
}

impl ClusteringComputePass {
    pub fn new(device: &wgpu::Device, world: &RenderWorld) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("clustering"),
            source: wgpu::ShaderSource::Wgsl(wesl_core::include_wesl!("clustering_cs").into()),
        });

        let resources = GpuClusters::new(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cluster layout"),
            bind_group_layouts: &[
                Some(&world.scene_bind_group_layout),
                None,
                None,
                Some(&resources.bind_group_layout),
            ],
            immediate_size: 0,
        });

        let cluster_bounds = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cluster bounds"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cluster_bounds"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let assign_lights = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("assign lights"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("assign_lights"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        Self {
            cluster_bounds,
            assign_lights,
            resources,
        }
    }

    pub fn dispatch(&mut self, encoder: &mut wgpu::CommandEncoder, world: &RenderWorld) {
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cluster bounds dispatch"),
                ..Default::default()
            });
            pass.set_pipeline(&self.cluster_bounds);
            pass.set_bind_group(BIND_GROUP_CLUSTERING, &self.resources.bind_group, &[]);
            pass.set_bind_group(BIND_GROUP_SCENE, &world.scene_bind_group, &[]);

            let nwg = (UVec3::from(CLUSTER_XYZ) + UVec3::from(CLUSTER_WG_SIZE) - 1)
                / UVec3::from(CLUSTER_WG_SIZE);
            pass.dispatch_workgroups(nwg.x, nwg.y, nwg.z);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("assign lights dispatch"),
                ..Default::default()
            });
            pass.set_pipeline(&self.assign_lights);
            pass.set_bind_group(BIND_GROUP_CLUSTERING, &self.resources.bind_group, &[]);
            pass.set_bind_group(BIND_GROUP_SCENE, &world.scene_bind_group, &[]);

            let nwg = (UVec3::from(CLUSTER_XYZ) + UVec3::from(CLUSTER_WG_SIZE) - 1)
                / UVec3::from(CLUSTER_WG_SIZE);
            pass.dispatch_workgroups(nwg.x, nwg.y, nwg.z);
        }
    }
}
