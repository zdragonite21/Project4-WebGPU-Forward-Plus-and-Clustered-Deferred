use glam::Vec3;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Light {
    pos: [f32; 3],
    _pad0: f32,
    color: [f32; 3],
    _pad1: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightsHeader {
    num_lights: u32,
    _pad: [u32; 3],
}

pub struct GpuLights {
    pub num_lights: u32,
    pub buffer: wgpu::Buffer,
}

fn hue_to_rgb(h: f32) -> Vec3 {
    let f = |n: f32| {
        let k = (n + h * 6.0) % 6.0;
        1.0 - k.min(4.0 - k).min(1.0).max(0.0)
    };
    Vec3::ONE.lerp(Vec3::new(f(5.0), f(3.0), f(1.0)), 0.8)
}

impl GpuLights {
    pub const NUM_LIGHTS: u32 = 500;
    pub const MAX_NUM_LIGHTS: u32 = 5000;
    pub const LIGHT_INTENSITY: f32 = 0.1;

    pub fn new(device: &wgpu::Device) -> Self {
        let mut lights_array = vec![Light::default(); Self::MAX_NUM_LIGHTS as usize];

        for light_idx in 0..(Self::MAX_NUM_LIGHTS as usize) {
            lights_array[light_idx].color =
                (hue_to_rgb(rand::random()) * Self::LIGHT_INTENSITY).into();
        }

        let light_set = LightsHeader {
            num_lights: Self::NUM_LIGHTS,
            _pad: [0; 3],
        };

        let mut contents = bytemuck::bytes_of(&light_set).to_vec();
        contents.extend_from_slice(bytemuck::cast_slice(&lights_array));

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lights Buffer"),
            contents: &contents,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            buffer,
            num_lights: Self::NUM_LIGHTS,
        }
    }
}
