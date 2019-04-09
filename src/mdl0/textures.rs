use fancy_slice::FancySlice;

use crate::resources::Resource;

pub(crate) fn textures(data: FancySlice, resources: Vec<Resource>) -> Vec<Texture> {
    let mut textures = vec!();
    for resource in resources {
        let mut references = vec!();

        let num_children = data.i32_be(resource.data_offset as usize);
        for i in 0..num_children as usize {
            let data = data.relative_fancy_slice(resource.data_offset as usize + 4 + TEXTURE_REF_SIZE * i..);
            let material_offset  = data.i32_be(0x00);
            let reference_offset = data.i32_be(0x04);

            references.push(TextureRef { material_offset, reference_offset });
        }
        let name = resource.string;
        textures.push(Texture { name, references });
    }
    textures
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub name: String,
    pub references: Vec<TextureRef>,
}

const TEXTURE_REF_SIZE: usize = 0x8;
#[derive(Clone, Debug)]
pub struct TextureRef {
    pub material_offset: i32,
    pub reference_offset: i32,
}
