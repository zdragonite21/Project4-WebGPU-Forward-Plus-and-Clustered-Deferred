#![allow(unused)]
// written by Opus 5.5

//! Files are read with `std::fs` on native and `fetch` on wasm, so `path` is
//! relative to the working directory (native) or the page URL (web).

use anyhow::{Context, anyhow, bail};
use glam::Mat4;

/// Interleaved vertex matching the shared layout: pos @0, nor @12, uv @24, stride 32.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GltfVertex {
    pub pos: [f32; 3],
    pub nor: [f32; 3],
    pub uv: [f32; 2],
}

impl GltfVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct GltfPrimitive {
    pub vertices: Vec<GltfVertex>,
    /// Always u32; u8/u16 indices are widened.
    pub indices: Vec<u32>,
    /// Index into `GltfScene::materials`; `None` means the glTF default material.
    pub material: Option<usize>,
}

pub struct GltfMesh {
    /// Sorted by material so consecutive draws can skip rebinding it.
    pub primitives: Vec<GltfPrimitive>,
}

/// One placement of a mesh in the scene, with the full parent→child transform applied.
pub struct GltfMeshInstance {
    /// Index into `GltfScene::meshes`.
    pub mesh: usize,
    pub world_transform: Mat4,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Filter {
    Nearest,
    Linear,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Wrap {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GltfSampler {
    pub mag: Filter,
    pub min: Filter,
    /// `None` if the glTF min filter doesn't use mipmaps.
    pub mipmap: Option<Filter>,
    pub wrap_u: Wrap,
    pub wrap_v: Wrap,
}

pub struct GltfMaterial {
    pub base_color_factor: [f32; 4],
    /// Index into `GltfScene::images`. Base color is sRGB: upload as `Rgba8UnormSrgb`.
    pub base_color_image: Option<usize>,
    pub sampler: GltfSampler,
}

/// Decoded image, always RGBA8, rows tightly packed (`width * 4` bytes).
pub struct GltfImage {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

pub struct GltfScene {
    pub meshes: Vec<GltfMesh>,
    pub instances: Vec<GltfMeshInstance>,
    pub materials: Vec<GltfMaterial>,
    pub images: Vec<GltfImage>,
}

pub async fn load_gltf(path: &str) -> anyhow::Result<GltfScene> {
    let base_dir = match path.rfind('/') {
        Some(i) => &path[..i],
        None => ".",
    };

    let gltf_bytes = load_bytes(path).await?;
    let gltf::Gltf { document, mut blob } =
        gltf::Gltf::from_slice(&gltf_bytes).with_context(|| format!("parsing {path}"))?;

    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let data = match buffer.source() {
            gltf::buffer::Source::Bin => blob.take().context("glTF references a missing GLB blob")?,
            gltf::buffer::Source::Uri(uri) => load_uri(base_dir, uri).await?,
        };
        if data.len() < buffer.length() {
            bail!("buffer {} is {} bytes, expected {}", buffer.index(), data.len(), buffer.length());
        }
        buffers.push(data);
    }

    let mut images = Vec::new();
    for image in document.images() {
        let encoded = match image.source() {
            gltf::image::Source::View { view, .. } => {
                let buffer = &buffers[view.buffer().index()];
                buffer[view.offset()..view.offset() + view.length()].to_vec()
            }
            gltf::image::Source::Uri { uri, .. } => load_uri(base_dir, uri).await?,
        };
        let decoded = image::load_from_memory(&encoded)
            .with_context(|| format!("decoding image {}", image.index()))?
            .to_rgba8();
        images.push(GltfImage {
            width: decoded.width(),
            height: decoded.height(),
            rgba8: decoded.into_raw(),
        });
    }

    let materials = document
        .materials()
        .map(|material| {
            let pbr = material.pbr_metallic_roughness();
            let texture = pbr.base_color_texture().map(|info| info.texture());
            GltfMaterial {
                base_color_factor: pbr.base_color_factor(),
                base_color_image: texture.as_ref().map(|t| t.source().index()),
                sampler: texture
                    .map(|t| convert_sampler(&t.sampler()))
                    .unwrap_or(DEFAULT_SAMPLER),
            }
        })
        .collect();

    let meshes = document
        .meshes()
        .map(|mesh| read_mesh(&mesh, &buffers))
        .collect::<anyhow::Result<_>>()?;

    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next())
        .context("glTF has no scenes")?;
    let mut instances = Vec::new();
    for node in scene.nodes() {
        collect_instances(&node, Mat4::IDENTITY, &mut instances);
    }

    Ok(GltfScene {
        meshes,
        instances,
        materials,
        images,
    })
}

fn read_mesh(mesh: &gltf::Mesh, buffers: &[Vec<u8>]) -> anyhow::Result<GltfMesh> {
    let mut primitives = Vec::new();
    for primitive in mesh.primitives() {
        if primitive.mode() != gltf::mesh::Mode::Triangles {
            log::warn!("skipping non-triangle primitive in mesh {}", mesh.index());
            continue;
        }

        let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));

        let positions: Vec<[f32; 3]> = reader
            .read_positions()
            .ok_or_else(|| anyhow!("primitive in mesh {} has no positions", mesh.index()))?
            .collect();
        let mut normals = reader.read_normals().map(|it| it.collect::<Vec<_>>());
        let mut uvs = reader.read_tex_coords(0).map(|it| it.into_f32().collect::<Vec<_>>());
        if normals.as_ref().is_some_and(|n| n.len() != positions.len()) {
            normals = None;
        }
        if uvs.as_ref().is_some_and(|u| u.len() != positions.len()) {
            uvs = None;
        }

        let vertices = (0..positions.len())
            .map(|i| GltfVertex {
                pos: positions[i],
                nor: normals.as_ref().map_or([0.0; 3], |n| n[i]),
                uv: uvs.as_ref().map_or([0.0; 2], |u| u[i]),
            })
            .collect();

        let indices = match reader.read_indices() {
            Some(indices) => indices.into_u32().collect(),
            None => (0..positions.len() as u32).collect(),
        };

        primitives.push(GltfPrimitive {
            vertices,
            indices,
            material: primitive.material().index(),
        });
    }
    primitives.sort_by_key(|p| p.material);
    Ok(GltfMesh { primitives })
}

fn collect_instances(node: &gltf::Node, parent: Mat4, out: &mut Vec<GltfMeshInstance>) {
    let world = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        out.push(GltfMeshInstance {
            mesh: mesh.index(),
            world_transform: world,
        });
    }
    for child in node.children() {
        collect_instances(&child, world, out);
    }
}

pub const DEFAULT_SAMPLER: GltfSampler = GltfSampler {
    mag: Filter::Linear,
    min: Filter::Linear,
    mipmap: Some(Filter::Linear),
    wrap_u: Wrap::Repeat,
    wrap_v: Wrap::Repeat,
};

fn convert_sampler(sampler: &gltf::texture::Sampler) -> GltfSampler {
    use gltf::texture::{MagFilter, MinFilter, WrappingMode};

    let wrap = |mode| match mode {
        WrappingMode::Repeat => Wrap::Repeat,
        WrappingMode::MirroredRepeat => Wrap::MirroredRepeat,
        WrappingMode::ClampToEdge => Wrap::ClampToEdge,
    };
    let mag = match sampler.mag_filter() {
        Some(MagFilter::Nearest) => Filter::Nearest,
        Some(MagFilter::Linear) | None => Filter::Linear,
    };
    let (min, mipmap) = match sampler.min_filter() {
        Some(MinFilter::Nearest) => (Filter::Nearest, None),
        Some(MinFilter::Linear) => (Filter::Linear, None),
        Some(MinFilter::NearestMipmapNearest) => (Filter::Nearest, Some(Filter::Nearest)),
        Some(MinFilter::LinearMipmapNearest) => (Filter::Linear, Some(Filter::Nearest)),
        Some(MinFilter::NearestMipmapLinear) => (Filter::Nearest, Some(Filter::Linear)),
        Some(MinFilter::LinearMipmapLinear) | None => (Filter::Linear, Some(Filter::Linear)),
    };

    GltfSampler {
        mag,
        min,
        mipmap,
        wrap_u: wrap(sampler.wrap_s()),
        wrap_v: wrap(sampler.wrap_t()),
    }
}

async fn load_uri(base_dir: &str, uri: &str) -> anyhow::Result<Vec<u8>> {
    if uri.starts_with("data:") {
        bail!("embedded data: URIs are not supported");
    }
    load_bytes(&format!("{base_dir}/{uri}")).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_bytes(path: &str) -> anyhow::Result<Vec<u8>> {
    std::fs::read(path).with_context(|| format!("reading {path}"))
}

#[cfg(target_arch = "wasm32")]
async fn load_bytes(path: &str) -> anyhow::Result<Vec<u8>> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let js_err = |e: wasm_bindgen::JsValue| anyhow!("fetching {path}: {e:?}");

    let window = web_sys::window().context("no window")?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str(path))
        .await
        .map_err(js_err)?
        .dyn_into()
        .map_err(js_err)?;
    if !response.ok() {
        bail!("fetching {path}: HTTP {}", response.status());
    }
    let array_buffer = JsFuture::from(response.array_buffer().map_err(js_err)?)
        .await
        .map_err(js_err)?;
    Ok(js_sys::Uint8Array::new(&array_buffer).to_vec())
}
