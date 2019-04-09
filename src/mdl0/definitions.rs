use fancy_slice::FancySlice;

use crate::resources::Resource;

pub(crate) fn definitions(data: FancySlice, resources: Vec<Resource>) -> Vec<Definition> {
    let mut definitions = vec!();
    for resource in resources {
        let data = data.relative_fancy_slice(resource.data_offset as usize ..);
        let name = resource.string;

        let mut offset = 0;
        let mut draw_calls = vec!();
        // TODO: Looks like data[offset] specifys what type the child is.
        //       If so draw_calls should be renamed children and store an enum of all possible children types
        //       Alternatively it might be branching on the names "DrawOpa" and "DrawXlu" - this is what brawlbox does, but brawlbox's implementation looks hacky.
        while data.u8(offset) == 4 {
            let material             = data.u16_be(offset + 0x01);
            let object               = data.u16_be(offset + 0x03);
            let visibility_bone_node = data.u16_be(offset + 0x05);
            let draw_order           = data.u8    (offset + 0x07);
            draw_calls.push(DrawCall { material, object, visibility_bone_node, draw_order });
            offset += DEFINITION_SIZE;
        }
        definitions.push(Definition { name, draw_calls });
    }
    definitions
}

const DEFINITION_SIZE: usize = 0x8;

#[derive(Clone, Debug)]
pub struct Definition {
    pub name: String,

    pub draw_calls: Vec<DrawCall>,
}

#[derive(Clone, Debug)]
pub struct DrawCall {
    pub material: u16,
    pub object: u16,
    pub visibility_bone_node: u16,
    pub draw_order: u8,
}
