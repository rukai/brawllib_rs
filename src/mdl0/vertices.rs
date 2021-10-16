use cgmath::Vector3;
use fancy_slice::FancySlice;

use crate::resources::Resource;

#[rustfmt::skip]
pub(crate) fn vertices(data: FancySlice, resources: Vec<Resource>) -> Vec<Vertices> {
    let mut vertices = vec!();
    for resource in resources {
        let data = data.relative_fancy_slice(resource.data_offset as usize..);

        let size           = data.i32_be(0x00); // including header
        let _mdl0_offset   = data.i32_be(0x04);
        let data_offset    = data.i32_be(0x08);
        let string_offset  = data.i32_be(0x0c); // 0x40
        let index          = data.i32_be(0x10);
        let is_xyz         = data.i32_be(0x14);
        let component_type = data.i32_be(0x18);
        let divisor        = data.u8    (0x1c);
        let entry_stride   = data.u8    (0x1d);
        let num_vertices   = data.u16_be(0x1e);
        let e_min = Vector3::<f32>::new(
            data.f32_be(0x20),
            data.f32_be(0x24),
            data.f32_be(0x28),
        );
        let e_max = Vector3::<f32>::new(
            data.f32_be(0x2c),
            data.f32_be(0x30),
            data.f32_be(0x34),
        );

        // 16 bytes of padding before data starts

        let data = data.relative_slice(data_offset as usize .. size as usize).to_vec();

        vertices.push(Vertices {
            name: resource.string,
            data,
            string_offset,
            index,
            is_xyz: is_xyz != 0,
            component_type: VertexComponentType::new(component_type),
            divisor,
            entry_stride,
            num_vertices,
            e_min,
            e_max,
        });
    }
    vertices
}

const _VERTICES_SIZE: usize = 0x40;
#[derive(Clone, Debug)]
pub struct Vertices {
    pub name: String,
    pub data: Vec<u8>,
    pub string_offset: i32,
    pub index: i32,
    pub is_xyz: bool,
    pub component_type: VertexComponentType,
    pub divisor: u8,
    pub entry_stride: u8,
    pub num_vertices: u16,
    pub e_min: Vector3<f32>,
    pub e_max: Vector3<f32>,
}

#[derive(Clone, Debug)]
pub enum VertexComponentType {
    U8,
    I8,
    U16,
    I16,
    F32,
    Unknown(i32),
}

impl VertexComponentType {
    fn new(value: i32) -> VertexComponentType {
        match value {
            0 => VertexComponentType::U8,
            1 => VertexComponentType::I8,
            2 => VertexComponentType::U16,
            3 => VertexComponentType::I16,
            4 => VertexComponentType::F32,
            _ => VertexComponentType::Unknown(value),
        }
    }
}
