use byteorder::{BigEndian, ReadBytesExt};
use cgmath::Vector3;

use crate::resources::Resource;

pub(crate) fn vertices(data: &[u8], resources: Vec<Resource>) -> Vec<Vertices> {
    let mut vertices = vec!();
    for resource in resources {
        let data = &data[resource.data_offset as usize..];

        let size           = (&data[0x00..]).read_i32::<BigEndian>().unwrap(); // including header
        let _mdl0_offset   = (&data[0x04..]).read_i32::<BigEndian>().unwrap();
        let data_offset    = (&data[0x08..]).read_i32::<BigEndian>().unwrap();
        let string_offset  = (&data[0x0c..]).read_i32::<BigEndian>().unwrap(); // 0x40
        let index          = (&data[0x10..]).read_i32::<BigEndian>().unwrap();
        let is_xyz         = (&data[0x14..]).read_i32::<BigEndian>().unwrap();
        let component_type = (&data[0x18..]).read_i32::<BigEndian>().unwrap();
        let divisor        = (&data[0x1c..]).read_u8().unwrap();
        let entry_stride   = (&data[0x1d..]).read_u8().unwrap();
        let num_vertices   = (&data[0x1e..]).read_u16::<BigEndian>().unwrap();
        let e_min = Vector3::<f32>::new(
            (&data[0x20..]).read_f32::<BigEndian>().unwrap(),
            (&data[0x24..]).read_f32::<BigEndian>().unwrap(),
            (&data[0x28..]).read_f32::<BigEndian>().unwrap(),
        );
        let e_max = Vector3::<f32>::new(
            (&data[0x2c..]).read_f32::<BigEndian>().unwrap(),
            (&data[0x30..]).read_f32::<BigEndian>().unwrap(),
            (&data[0x34..]).read_f32::<BigEndian>().unwrap(),
        );
        // 16 bytes of padding before data starts

        let data = (&data[data_offset as usize.. size as usize]).to_vec();

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
    Unknown (i32),
}

impl VertexComponentType {
    fn new(value: i32) -> VertexComponentType {
        match value {
            0 => VertexComponentType::U8,
            1 => VertexComponentType::I8,
            2 => VertexComponentType::U16,
            3 => VertexComponentType::I16,
            4 => VertexComponentType::F32,
            _ => VertexComponentType::Unknown (value),
        }
    }
}
