use byteorder::{BigEndian, ReadBytesExt};

use crate::resources::Resource;

pub(crate) fn palettes(data: &[u8], resources: Vec<Resource>) -> Vec<Palette> {
    let mut palettes = vec!();
    for resource in resources {
        let mut references = vec!();

        let num_children = (&data[resource.data_offset as usize..]).read_i32::<BigEndian>().unwrap();
        for i in 0..num_children as usize {
            let data = &data[resource.data_offset as usize + 4 + PALETTE_REF_SIZE * i..];
            let material_offset  = (&data[0x00..]).read_i32::<BigEndian>().unwrap();
            let reference_offset = (&data[0x04..]).read_i32::<BigEndian>().unwrap();

            references.push(PaletteRef { material_offset, reference_offset });
        }

        let name = resource.string;
        palettes.push(Palette { name, references });
    }
    palettes
}

#[derive(Clone, Debug)]
pub struct Palette {
    pub name: String,
    pub references: Vec<PaletteRef>,
}

const PALETTE_REF_SIZE: usize = 0x8;
#[derive(Clone, Debug)]
pub struct PaletteRef {
    pub material_offset: i32,
    pub reference_offset: i32,
}
