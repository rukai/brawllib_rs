use fancy_slice::FancySlice;

use crate::resources;
use crate::resources::Resource;

#[derive(Clone, Debug)]
pub struct Definitions {
    pub values: Vec<Definition>,
}

impl Definitions {
    pub fn compile(&self) -> Vec<u8> {
        let mut output = vec![];

        // create resources header
        let resources_size =
            (self.values.len() + 1) * resources::RESOURCE_SIZE + resources::RESOURCE_HEADER_SIZE; // includes the dummy child
        output.extend(i32::to_be_bytes(resources_size as i32));
        output.extend(i32::to_be_bytes(self.values.len() as i32)); // num_children

        // insert the dummy child
        output.extend([
            0xff, 0xff, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ]);

        //let data_offset = 0; // TODO

        for _definition in &self.values {
            //output.extend(&u16::to_be_bytes(child.id));
            //output.extend(&u16::to_be_bytes(child.flag));
            //output.extend(&u16::to_be_bytes(child.left_index));
            //output.extend(&u16::to_be_bytes(child.right_index));
            //output.extend(&i32::to_be_bytes(4)); // TODO: string_offset
            //output.extend(&i32::to_be_bytes(data_offset));
        }

        output
    }
}

#[rustfmt::skip]
pub(crate) fn definitions(data: FancySlice, resources: Vec<Resource>) -> Definitions {
    let mut definitions = vec!();
    for resource in resources {
        let data = data.relative_fancy_slice(resource.data_offset as usize ..);
        let name = resource.string;

        let mut offset = 0;
        let mut draw_calls = vec!();
        // TODO: Looks like data[offset] specifies what type the child is.
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
    Definitions { values: definitions }
}

impl Definition {
    pub fn compile(&self) -> Vec<u8> {
        let output = vec![];

        output
    }
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
