use crate::bres::*;
use crate::util;
use crate::sakurai;
use crate::sakurai::ArcSakurai;
use crate::wii_memory::WiiMemory;

use fancy_slice::FancySlice;

pub(crate) fn arc(data: FancySlice, wii_memory: &WiiMemory, item: bool) -> Arc {
    //read the main header
    let num_sub_headers = data.u16_be(6);
    let name = data.str(0x10).unwrap().to_string();

    // read the sub headers
    let mut children = vec!();
    let mut header_index = ARC_HEADER_SIZE;
    for i in 0..num_sub_headers {
        let mut arc_child = arc_child(data.relative_fancy_slice(header_index..));
        if arc_child.redirect_index == 0xFF {
            let tag = util::parse_tag(&data.relative_slice(header_index + ARC_CHILD_HEADER_SIZE ..));
            let child_data = data.relative_fancy_slice(header_index + ARC_CHILD_HEADER_SIZE ..);
            arc_child.data = match tag.as_ref() {
                "ARC"  => ArcChildData::Arc(arc(child_data, wii_memory, item)),
                "EFLS" => ArcChildData::Efls,
                "bres" => ArcChildData::Bres(bres(child_data)),
                "ATKD" => ArcChildData::Atkd,
                "REFF" => ArcChildData::Reff,
                "REFT" => ArcChildData::Reft,
                "AIPD" => ArcChildData::Aipd,
                "W"    => ArcChildData::W,
                "" if i == 0 => ArcChildData::Sakurai(sakurai::arc_sakurai(data.relative_fancy_slice(header_index + ARC_CHILD_HEADER_SIZE ..), wii_memory, item)),
                _ => ArcChildData::Unknown
            };

            header_index += ARC_CHILD_HEADER_SIZE + arc_child.size as usize;

            // align to the next ARC_CHILD_HEADER_SIZE
            let offset = header_index % ARC_CHILD_HEADER_SIZE;
            if offset != 0 {
                header_index += ARC_CHILD_HEADER_SIZE - offset;
            }
            children.push(arc_child);
        }
    }

    Arc { name, children }
}

fn arc_child(data: FancySlice) -> ArcChild {
    ArcChild {
        ty:             data.i16_be(0),
        index:          data.i16_be(2),
        size:           data.i32_be(4),
        group_index:    data.u8(8),
        redirect_index: data.i16_be(9),
        data:           ArcChildData::Unknown,
    }
}

const ARC_HEADER_SIZE: usize = 0x40;
/// Arc is for archive not to be confused with an atomic reference count
#[derive(Clone, Debug)]
pub struct Arc {
    pub name: String,
    pub children: Vec<ArcChild>,
}

const ARC_CHILD_HEADER_SIZE: usize = 0x20;
#[derive(Clone, Debug)]
pub struct ArcChild {
    ty: i16,
    index: i16,
    size: i32,
    group_index: u8,
    redirect_index: i16, // The index of a different file to read
    pub data: ArcChildData,
}

#[derive(Clone, Debug)]
pub enum ArcChildData {
    Arc (Arc),
    Sakurai (ArcSakurai),
    Efls,
    Bres (Bres),
    Atkd,
    Reff,
    Reft,
    Aipd,
    W,
    Unknown
}
