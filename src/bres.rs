use fancy_slice::FancySlice;

use crate::util;
use crate::resources;
use crate::chr0::*;
use crate::mdl0::*;

pub(crate) fn bres(data: FancySlice) -> Bres {
    let endian         = data.u16_be(0x4);
    let version        = data.u16_be(0x6);
    //let size         = data.u32_be(0x8);
    let root_offset    = data.u16_be(0xc);
    //let num_sections = data.u16_be(0xe);

    let children = bres_group(data.relative_fancy_slice(root_offset as usize ..));
    Bres { endian, version, children }
}

fn bres_group(data: FancySlice) -> Vec<BresChild> {
    let mut children = vec!();
    for resource in resources::resources(data.relative_fancy_slice(ROOT_HEADER_SIZE..)) {
        let child_data = data.relative_fancy_slice(ROOT_HEADER_SIZE + resource.data_offset as usize ..);

        let tag = util::parse_tag(child_data.relative_slice(..));
        let child_data = match tag.as_ref() {
            "CHR0" => BresChildData::Chr0 (chr0(child_data)),
            "MDL0" => BresChildData::Mdl0 (mdl0(child_data)),
            "" => BresChildData::Bres (bres_group(data.relative_fancy_slice(resource.data_offset as usize ..))), // TODO: I suspect the match on "" is succeeding by accident
            _  => BresChildData::Unknown (tag),
        };

        children.push(BresChild {
            data_offset: resource.data_offset,
            name:        resource.string,
            data:        child_data,
        });
    }

    children
}

// Brawlbox has this split into three structs: BRESHeader, BRESEntry and ROOTHeader
// BRESEntry is commented out, so that appears wrong
// BRESHeader and RootHeader are combined because without BRESEntry they appear to be sequential
const BRES_HEADER_SIZE: usize = 0x10;
#[derive(Clone, Debug)]
pub struct Bres {
    pub endian:   u16,
    pub version:  u16,
    pub children: Vec<BresChild>
}

impl Bres {
    pub fn compile(&self) -> Vec<u8> {
        let mut output = vec!();
        let mut root_output: Vec<u8> = vec!();

        let root_size = ROOT_HEADER_SIZE
            + resources::RESOURCE_HEADER_SIZE
            + resources::RESOURCE_SIZE
            + self.children.iter().map(|x| x.bres_size()).sum::<usize>();

        let bres_size_leafless = BRES_HEADER_SIZE + root_size;

        let mut bres_size_leafless_buffered = bres_size_leafless;
        while bres_size_leafless_buffered % 0x20 != 0 {
            bres_size_leafless_buffered += 1; // TODO: arithmeticize the loop
        }

        // compile children
        let mut leaf_children_output: Vec<Vec<u8>> = vec!();
        let mut leaf_children_size = 0;
        let mut to_process = vec!(&self.children);
        while to_process.len() > 0 {
            let children = to_process.pop().unwrap();
            let resource_header_offset = BRES_HEADER_SIZE + ROOT_HEADER_SIZE + root_output.len();

            // create resources header
            let resources_size = (children.len() + 1) * resources::RESOURCE_SIZE + resources::RESOURCE_HEADER_SIZE; // includes the dummy child
            root_output.extend(&i32::to_be_bytes(resources_size as i32));
            root_output.extend(&i32::to_be_bytes(children.len() as i32)); // num_children

            // insert the dummy child
            root_output.extend(&[0xff, 0xff, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

            let mut data_offset_current = resources_size;
            for child in children.iter() {
                let data_offset = match child.data {
                    BresChildData::Bres (_) => data_offset_current as i32,
                    _ =>
                        bres_size_leafless_buffered as i32
                        - resource_header_offset as i32
                        + leaf_children_size,
                };

                match child.data {
                    BresChildData::Bres (ref children) => {
                        to_process.push(children);
                        data_offset_current += (children.len() + 1) * resources::RESOURCE_SIZE + resources::RESOURCE_HEADER_SIZE;
                    }
                    _ => {
                        // calculate offset to child
                        let mut child_offset = bres_size_leafless as i32;
                        while child_offset % 0x20 != 0 {
                            child_offset += 1; // TODO: arithmeticize the loop
                        }
                        child_offset += leaf_children_size;

                        let child_output = child.compile(-child_offset);

                        leaf_children_size = if let Some(result) = leaf_children_size.checked_add(child_output.len() as i32) {
                            result
                        } else {
                            panic!("BRES over 2 ^ 32 bytes"); // TODO: Make this an Err(_)
                        };

                        leaf_children_output.push(child_output);
                    }
                }

                // create each resource
                root_output.extend(&u16::to_be_bytes(0)); // TODO: id
                root_output.extend(&u16::to_be_bytes(1)); // TODO: flag
                root_output.extend(&u16::to_be_bytes(2)); // TODO: left_index
                root_output.extend(&u16::to_be_bytes(3)); // TODO: right_index
                root_output.extend(&i32::to_be_bytes(4)); // TODO: string_offset
                root_output.extend(&i32::to_be_bytes(data_offset));
            }
        }

        let bres_size = bres_size_leafless as u32 + leaf_children_size as u32;
        let leaf_count: usize = self.children.iter().map(|x| x.count_leaves()).sum();

        // create bres header
        output.extend("bres".chars().map(|x| x as u8));
        output.extend(&u16::to_be_bytes(self.endian));
        output.extend(&u16::to_be_bytes(self.version));
        output.extend(&u32::to_be_bytes(bres_size as u32));
        output.extend(&u16::to_be_bytes(0x10)); // root_offset
        output.extend(&u16::to_be_bytes(leaf_count as u16 + 1)); // +1 for the root entry

        // create bres root child header
        output.extend("root".chars().map(|x| x as u8));
        output.extend(&i32::to_be_bytes(root_size as i32));

        // create bres root child contents
        output.extend(root_output);
        while output.len() % 0x20 != 0 {
            output.push(0x00);
        }

        // create bres leaf children
        let mut size: u32 = 0;
        for child_output in leaf_children_output {
            size = if let Some(result) = size.checked_add(child_output.len() as u32) {
                result
            } else {
                panic!("BRES over 2 ^ 32 bytes"); // TODO: Make this an Err(_)
            };

            output.extend(child_output);
        }

        output
    }
}

impl BresChild {
    pub fn compile(&self, bres_offset: i32) -> Vec<u8> {
        match &self.data {
            BresChildData::Bres (children) => {
                let mut output = vec!();

                for child in children {
                    output.extend(child.compile(bres_offset));
                }

                output
            }
            BresChildData::Mdl0 (child) => child.compile(bres_offset),
            _ => vec!(),
        }
    }

    // Calculates the size taken up by non-leaf data
    // Doesnt include the root data
    fn bres_size(&self) -> usize {
        resources::RESOURCE_SIZE + // the resource entry
            match &self.data {
                // its pointing to a group of children
                BresChildData::Bres (children) =>
                    resources::RESOURCE_HEADER_SIZE
                    + resources::RESOURCE_SIZE // dummy child
                    + children.iter().map(|x| x.bres_size()).sum::<usize>(),

                // its pointing to a leaf node
                _ => 0,
            }
    }

    fn count_leaves(&self) -> usize {
        match &self.data {
            BresChildData::Bres (children) => children.iter().map(|x| x.count_leaves()).sum::<usize>(),
            _ => 1,
        }
    }
}

const ROOT_HEADER_SIZE: usize = 0x8;
#[derive(Clone, Debug)]
pub struct BresChild {
    data_offset: i32,
    pub name: String,
    pub data: BresChildData
}

#[derive(Clone, Debug)]
pub enum BresChildData {
    Chr0 (Chr0),
    Mdl0 (Mdl0),
    Bres (Vec<BresChild>),
    Unknown (String)
}

