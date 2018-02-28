use byteorder::{BigEndian, ReadBytesExt};
use cgmath::Vector3;

pub fn mbox(data: &[u8]) -> MBox {
    MBox {
        min: Vector3::<f32>::new(
            (&data[0x00..]).read_f32::<BigEndian>().unwrap(),
            (&data[0x04..]).read_f32::<BigEndian>().unwrap(),
            (&data[0x08..]).read_f32::<BigEndian>().unwrap(),
        ),
        max: Vector3::<f32>::new(
            (&data[0x0c..]).read_f32::<BigEndian>().unwrap(),
            (&data[0x10..]).read_f32::<BigEndian>().unwrap(),
            (&data[0x14..]).read_f32::<BigEndian>().unwrap(),
        )
    }
}

// named MBox because Box is used in std lib
#[derive(Debug)]
pub struct MBox {
    min: Vector3<f32>,
    max: Vector3<f32>,
}
