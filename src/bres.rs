use byteorder::{BigEndian, ReadBytesExt};

use util;
use resources;
use chr0::*;
use mdl0::*;

pub(crate) fn bres(data: &[u8]) -> Bres {
    let root_offset = (&data[0xc..0xe]).read_u16::<BigEndian>().unwrap();
    bres_group(&data[root_offset as usize ..])
}

fn bres_group(data: &[u8]) -> Bres {
    let mut children = vec!();
    for resource in resources::resources(&data[ROOT_SIZE..]) {
        let child_data = &data[ROOT_SIZE + resource.data_offset as usize .. ];

        let tag = util::parse_tag(child_data);
        let child_data = match tag.as_ref() {
            "CHR0" => BresChildData::Chr0 (chr0(child_data)),
            "MDL0" => BresChildData::Mdl0 (mdl0(child_data)),
            "" => BresChildData::Bres (Box::new(bres_group(&data[resource.data_offset as usize ..]))),
            _  => BresChildData::Unknown (tag),
        };

        children.push(BresChild {
            string_offset: resource.string_offset,
            data_offset:   resource.data_offset,
            name:          resource.string,
            data:          child_data,
        });
    }

    Bres {
        children
    }
}

// Brawlbox has this split into three structs: BRESHeader, BRESEntry and ROOTHeader
// BRESEntry is commented out, so that appears wrong
// BRESHeader and RootHeader are combined because without BRESEntry they appear to be sequential
#[derive(Debug)]
pub struct Bres {
    pub children: Vec<BresChild>
}

const ROOT_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct BresChild {
    string_offset: i32,
    data_offset: i32,
    pub name: String,
    pub data: BresChildData
}

#[derive(Debug)]
pub enum BresChildData {
    Chr0 (Chr0),
    Mdl0 (Mdl0),
    Bres (Box<Bres>),
    Unknown (String)
}

