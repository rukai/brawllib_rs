use byteorder::{BigEndian, ReadBytesExt};

use crate::resources::Resource;

pub(crate) fn textures(data: &[u8], resources: Vec<Resource>) -> Vec<Vec<TextureRef>> {
    let mut textures = vec!();
    for resource in resources {
        let mut texture = vec!();

        let num_children = (&data[resource.data_offset as usize..]).read_i32::<BigEndian>().unwrap();
        for i in 0..num_children as usize {
            let data = &data[resource.data_offset as usize + 4 + TEXTURE_REF_SIZE * i..];
            let material_offset  = (&data[0x00..]).read_i32::<BigEndian>().unwrap();
            let reference_offset = (&data[0x04..]).read_i32::<BigEndian>().unwrap();

            texture.push(TextureRef { material_offset, reference_offset });
        }
        textures.push(texture);
    }
    textures
}

const TEXTURE_REF_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct TextureRef {
    pub material_offset: i32,
    pub reference_offset: i32,
}
