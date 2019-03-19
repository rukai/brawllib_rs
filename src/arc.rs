use byteorder::{BigEndian, ReadBytesExt};

use crate::bres::*;
use crate::util;
use crate::sakurai;
use crate::sakurai::ArcSakurai;
use crate::wii_memory::WiiMemory;

pub(crate) fn arc(data: &[u8], wii_memory: &WiiMemory) -> Arc {
    //read the main header
    let num_sub_headers = (&data[6..8]).read_u16::<BigEndian>().unwrap();
    let name = String::from(util::parse_str(&data[0x10..]).unwrap());

    // read the sub headers
    let mut children = vec!();
    let mut header_index = ARC_HEADER_SIZE;
    for i in 0..num_sub_headers {
        let mut arc_child = arc_child(&data[header_index ..]);
        if arc_child.redirect_index == 0xFF {
            let tag = util::parse_tag(&data[header_index + ARC_CHILD_HEADER_SIZE .. ]);
            let child_data = &data[header_index + ARC_CHILD_HEADER_SIZE ..];
            arc_child.data = match tag.as_ref() {
                "ARC"  => ArcChildData::Arc(arc(&child_data, wii_memory)),
                "EFLS" => ArcChildData::Efls,
                "bres" => ArcChildData::Bres(bres(&child_data)),
                "ATKD" => ArcChildData::Atkd,
                "REFF" => ArcChildData::Reff,
                "REFT" => ArcChildData::Reft,
                "AIPD" => ArcChildData::Aipd,
                "W"    => ArcChildData::W,
                "" if i == 0 => ArcChildData::Sakurai(sakurai::arc_sakurai(&data[header_index + ARC_CHILD_HEADER_SIZE ..], wii_memory)),
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

fn arc_child(data: &[u8]) -> ArcChild {
    ArcChild {
        ty:             (&data[0..2]).read_i16::<BigEndian>().unwrap(),
        index:          (&data[2..4]).read_i16::<BigEndian>().unwrap(),
        size:           (&data[4..8]).read_i32::<BigEndian>().unwrap(),
        group_index:      data[8],
        redirect_index: (&data[9..11]).read_i16::<BigEndian>().unwrap(),
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
