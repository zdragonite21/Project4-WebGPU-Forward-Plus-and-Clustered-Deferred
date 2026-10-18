use glam::Vec3;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Light {
    pos: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightSet {
    num_lights: u32,
    lights: Vec<Light>,
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
    pub const NUM_FLOATS_PER_LIGHT: u32 = 8;
    pub const LIGHT_INTENSITY: f32 = 0.1;

    fn new(device: &wgpu::Device, cam_rig: &CameraRig) -> Self {
        let lights_array = Vec::new();

        for light_idx in 0..Self::MAX_NUM_LIGHTS {
            lights_array.push(Light {
                pos: [0.0; 3],
                color: (hue_to_rgb(rand::Random()) * Self::LIGHT_INTENSITY).into(),
            })
        }

        let light_set = LightSet {
            num_lights: Self::NUM_LIGHTS,
            lights: lights_array,
        };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lights Buffer"),
            contents: bytemuck::cast_slice(&[cast_slice(light_set)]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            buffer,
            num_lights: Self::NUM_LIGHTS,
        }
    }
}
