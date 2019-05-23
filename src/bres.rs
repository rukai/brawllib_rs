use fancy_slice::FancySlice;

use crate::util;
use crate::resources;
use crate::chr0::*;
use crate::mdl0::*;

pub(crate) fn bres(data: FancySlice) -> Bres {
    let root_offset = data.u16_be(0xc);
    bres_group(data.relative_fancy_slice(root_offset as usize ..))
}

fn bres_group(data: FancySlice) -> Bres {
    let mut children = vec!();
    for resource in resources::resources(data.relative_fancy_slice(ROOT_SIZE..)) {
        let child_data = data.relative_fancy_slice(ROOT_SIZE + resource.data_offset as usize ..);

        let tag = util::parse_tag(child_data.relative_slice(..));
        let child_data = match tag.as_ref() {
            "CHR0" => BresChildData::Chr0 (chr0(child_data)),
            "MDL0" => BresChildData::Mdl0 (mdl0(child_data)),
            "" => BresChildData::Bres (Box::new(bres_group(data.relative_fancy_slice(resource.data_offset as usize ..)))),
            _  => BresChildData::Unknown (tag),
        };

        children.push(BresChild {
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
#[derive(Clone, Debug)]
pub struct Bres {
    pub children: Vec<BresChild>
}

impl Bres {
    pub fn compile(&self) -> Vec<u8> {
        let mut output = vec!();

        // create bres header
        output.extend("bres".chars().map(|x| x as u8));
        output.extend(&[0xfe, 0xff, 0x00, 0x00, 0x00, 0x06, 0xbf, 0x80, 0x00, 0x10, 0x00, 0x02]); // TODO: I just copied these from one value, check what they mean on brawlbox

        for child in &self.children {
            match &child.data {
                BresChildData::Bres (bres) => output.extend(bres.compile()),
                _ => { }
            }
        }

        output
    }
}

const ROOT_SIZE: usize = 0x8;
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
    Bres (Box<Bres>),
    Unknown (String)
}

