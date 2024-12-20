use vks::{cmd_create_device_local_buffer_with_data, Buffer, Context};

use super::{generate_tangents, IndexBuffer, Material, ModelVertex, VertexBuffer};
use vks::ash::vk;
use cgmath::Vector3;
use gltf::{
    buffer::{Buffer as GltfBuffer, Data},
    mesh::{Bounds, Reader, Semantic},
    Document,
};
use math::*;
use std::{mem::size_of, sync::Arc};

pub struct Mesh {
    primitives: Vec<Primitive>,
    aabb: Aabb<f32>,
}

impl Mesh {
    fn new(primitives: Vec<Primitive>) -> Self {
        let aabbs = primitives.iter().map(|p| p.aabb()).collect::<Vec<_>>();
        let aabb = Aabb::union(&aabbs).unwrap();
        Mesh { primitives, aabb }
    }
}

impl Mesh {
    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }

    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    pub fn aabb(&self) -> Aabb<f32> {
        self.aabb
    }
}

pub struct Primitive {
    index: usize,
    vertices: VertexBuffer,
    indices: Option<IndexBuffer>,
    material: Material,
    material_index: Option<usize>,
    aabb: Aabb<f32>,
}

impl Primitive {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn vertices(&self) -> &VertexBuffer {
        &self.vertices
    }

    pub fn indices(&self) -> &Option<IndexBuffer> {
        &self.indices
    }

    pub fn material(&self) -> Material {
        self.material
    }

    pub fn material_index(&self) -> Option<usize> {
        self.material_index
    }

    pub fn aabb(&self) -> Aabb<f32> {
        self.aabb
    }
}

/// Vertex buffer byte offset / element count
type VertexBufferPart = (usize, usize);

/// Index buffer byte offset / element count
type IndexBufferPart = (usize, usize);

struct PrimitiveData {
    index: usize,
    indices: Option<IndexBufferPart>,
    vertices: VertexBufferPart,
    material: Material,
    material_index: Option<usize>,
    aabb: Aabb<f32>,
}

pub struct Meshes {
    pub meshes: Vec<Mesh>,
    pub vertices: Buffer,
    pub indices: Option<Buffer>,
}

pub fn create_meshes_from_gltf(
    context: &Arc<Context>,
    command_buffer: vk::CommandBuffer,
    document: &Document,
    buffers: &[Data],
) -> Option<Meshes> {
    let mut meshes_data = Vec::<Vec<PrimitiveData>>::new();
    let mut all_vertices = Vec::<ModelVertex>::new();
    let mut all_indices = Vec::<u32>::new();

    let mut primitive_count = 0;

    // Gather vertices and indices from all the meshes in the document
    for mesh in document.meshes() {
        let mut primitives_buffers = Vec::<PrimitiveData>::new();

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            if let Some(accessor) = primitive.get(&Semantic::Positions) {
                let aabb = get_aabb(&primitive.bounding_box());
                let positions = read_positions(&reader);
                let normals = read_normals(&reader);
                let tex_coords_0 = read_tex_coords(&reader, 0);
                let tex_coords_1 = read_tex_coords(&reader, 1);
                let tangents = read_tangents(&reader);
                let weights = read_weights(&reader);
                let joints = read_joints(&reader);
                let colors = read_colors(&reader);

                let mut vertices = positions
                    .iter()
                    .enumerate()
                    .map(|(index, position)| {
                        let position = *position;
                        let normal = *normals.get(index).unwrap_or(&[1.0, 1.0, 1.0]);
                        let tex_coords_0 = *tex_coords_0.get(index).unwrap_or(&[0.0, 0.0]);
                        let tex_coords_1 = *tex_coords_1.get(index).unwrap_or(&[0.0, 0.0]);
                        let tangent = *tangents.get(index).unwrap_or(&[1.0, 1.0, 1.0, 1.0]);
                        let weights = *weights.get(index).unwrap_or(&[0.0, 0.0, 0.0, 0.0]);
                        let joints = *joints.get(index).unwrap_or(&[0, 0, 0, 0]);
                        let colors = *colors.get(index).unwrap_or(&[1.0, 1.0, 1.0, 1.0]);

                        ModelVertex {
                            position,
                            normal,
                            tex_coords_0,
                            tex_coords_1,
                            tangent,
                            weights,
                            joints,
                            colors,
                        }
                    })
                    .collect::<Vec<_>>();

                let indices = read_indices(&reader);

                if !positions.is_empty()
                    && !normals.is_empty()
                    && !tex_coords_0.is_empty()
                    && tangents.is_empty()
                {
                    generate_tangents(indices.as_deref(), &mut vertices);
                }

                let indices = indices.map(|indices| {
                    let offset = all_indices.len() * size_of::<u32>();
                    all_indices.extend_from_slice(&indices);
                    (offset, indices.len())
                });

                let offset = all_vertices.len() * size_of::<ModelVertex>();
                all_vertices.extend_from_slice(&vertices);

                let material = primitive.material().into();

                let index = primitive_count;
                primitive_count += 1;

                primitives_buffers.push(PrimitiveData {
                    index,
                    indices,
                    vertices: (offset, accessor.count()),
                    material,
                    material_index: primitive.material().index(),
                    aabb,
                });
            }
        }

        meshes_data.push(primitives_buffers);
    }

    if !meshes_data.is_empty() {
        let indices = if all_indices.is_empty() {
            None
        } else {
            let (indices, staged_indices) = cmd_create_device_local_buffer_with_data::<u8, _>(
                context,
                command_buffer,
                vk::BufferUsageFlags::INDEX_BUFFER,
                &all_indices,
            );
            Some((Arc::new(indices), staged_indices))
        };

        let (vertices, staged_vertices) = cmd_create_device_local_buffer_with_data::<u8, _>(
            context,
            command_buffer,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            &all_vertices,
        );
        let vertices = Arc::new(vertices);

        let meshes = meshes_data
            .iter()
            .map(|primitives_buffers| {
                let primitives = primitives_buffers
                    .iter()
                    .map(|buffers| {
                        let mesh_vertices = buffers.vertices;
                        let vertex_buffer = VertexBuffer::new(
                            Arc::clone(&vertices),
                            mesh_vertices.0 as _,
                            mesh_vertices.1 as _,
                        );

                        let index_buffer = buffers.indices.map(|mesh_indices| {
                            IndexBuffer::new(
                                Arc::clone(indices.as_ref().map(|(indices, _)| indices).unwrap()),
                                mesh_indices.0 as _,
                                mesh_indices.1 as _,
                            )
                        });

                        Primitive {
                            index: buffers.index,
                            vertices: vertex_buffer,
                            indices: index_buffer,
                            material: buffers.material,
                            material_index: buffers.material_index,
                            aabb: buffers.aabb,
                        }
                    })
                    .collect::<Vec<_>>();
                Mesh::new(primitives)
            })
            .collect();

        return Some(Meshes {
            meshes,
            vertices: staged_vertices,
            indices: indices.map(|(_, staged_indices)| staged_indices),
        });
    }

    None
}

fn read_indices<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Option<Vec<u32>>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_indices()
        .map(|indices| indices.into_u32().collect::<Vec<_>>())
}

fn get_aabb(bounds: &Bounds<[f32; 3]>) -> Aabb<f32> {
    let min = bounds.min;
    let min = Vector3::new(min[0], min[1], min[2]);

    let max = bounds.max;
    let max = Vector3::new(max[0], max[1], max[2]);

    Aabb::new(min, max)
}

fn read_positions<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<[f32; 3]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_positions()
        .expect("Position primitives should be present")
        .collect()
}

fn read_normals<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<[f32; 3]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_normals()
        .map_or(vec![], |normals| normals.collect())
}

fn read_tex_coords<'a, 's, F>(reader: &Reader<'a, 's, F>, channel: u32) -> Vec<[f32; 2]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_tex_coords(channel)
        .map_or(vec![], |coords| coords.into_f32().collect())
}

fn read_tangents<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<[f32; 4]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_tangents()
        .map_or(vec![], |tangents| tangents.collect())
}

fn read_weights<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<[f32; 4]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_weights(0)
        .map_or(vec![], |weights| weights.into_f32().collect())
}

fn read_joints<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<[u32; 4]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader.read_joints(0).map_or(vec![], |joints| {
        joints
            .into_u16()
            .map(|[x, y, z, w]| [u32::from(x), u32::from(y), u32::from(z), u32::from(w)])
            .collect()
    })
}

fn read_colors<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<[f32; 4]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_colors(0)
        .map_or(vec![], |colors| colors.into_rgba_f32().collect())
}
