use crate::bres::*;
use crate::sakurai;
use crate::sakurai::ArcSakurai;
use crate::util;
use crate::wii_memory::WiiMemory;

use fancy_slice::FancySlice;

pub(crate) fn arc(data: FancySlice, wii_memory: &WiiMemory, item: bool) -> Arc {
    // read the main header
    let num_sub_headers = data.u16_be(6);
    let name = data.str(0x10).unwrap().to_string();

    // read the sub headers
    let mut children = vec![];
    let mut header_index = ARC_HEADER_SIZE;
    for i in 0..num_sub_headers {
        let mut arc_child = arc_child(data.relative_fancy_slice(header_index..));
        if arc_child.redirect_index == -1 {
            let tag = util::parse_tag(data.relative_slice(header_index + ARC_CHILD_HEADER_SIZE..));
            let child_data = data.relative_fancy_slice(header_index + ARC_CHILD_HEADER_SIZE..);
            arc_child.data = match tag.as_ref() {
                "ARC" => ArcChildData::Arc(arc(child_data, wii_memory, item)),
                "EFLS" => ArcChildData::Efls,
                "bres" => ArcChildData::Bres(bres(child_data)),
                "ATKD" => ArcChildData::Atkd,
                "REFF" => ArcChildData::Reff,
                "REFT" => ArcChildData::Reft,
                "AIPD" => ArcChildData::Aipd,
                "W" => ArcChildData::W,
                "" if i == 0 => ArcChildData::Sakurai(sakurai::arc_sakurai(
                    data.relative_fancy_slice(header_index + ARC_CHILD_HEADER_SIZE..),
                    wii_memory,
                    item,
                )),
                _ => ArcChildData::Unknown,
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

#[rustfmt::skip]
fn arc_child(data: FancySlice) -> ArcChild {
    ArcChild {
        ty:             data.i16_be(0),
        index:          data.i16_be(2),
        size:           data.i32_be(4),
        group_index:    data.u8(8),
        redirect_index: data.i16_be(10),
        data:           ArcChildData::Unknown,
    }
}

impl Arc {
    pub fn compile(&self) -> Vec<u8> {
        // TODO: Would be more efficient to allocate once, then overwrite the bytes at specific offsets.
        // However, for now, having each section create its own vec which get `extend`ed together makes for a cleaner implementation.
        let mut output = Vec::with_capacity(1024 * 1024); // Preallocate 1MB, we will likely need more, but dont want to overdo it as we have arcs in arcs.

        // create arc header
        output.extend("ARC".chars().map(|x| x as u8));
        output.extend([0x00, 0x01, 0x01]); // TODO: ??
        output.extend(u16::to_be_bytes(self.children.len() as u16));
        output.extend([0x00; 8]);
        output.extend(self.name.chars().map(|x| x as u8));
        while output.len() < ARC_HEADER_SIZE {
            output.push(0x00);
        }

        for child in &self.children {
            // create arc child header
            let start = output.len();
            output.extend(i16::to_be_bytes(child.ty));
            output.extend(i16::to_be_bytes(child.index));
            output.extend(i32::to_be_bytes(child.size)); // TODO: remove this field and calculate it instead
            output.push(child.group_index);
            output.push(0x00);
            output.extend(i16::to_be_bytes(child.redirect_index));
            while output.len() < start + ARC_CHILD_HEADER_SIZE {
                output.push(0x00);
            }

            match &child.data {
                ArcChildData::Arc(arc) => output.extend(arc.compile()),
                ArcChildData::Bres(bres) => output.extend(bres.compile()),
                // TODO
                _ => {}
            }
        }

        output
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
    Arc(Arc),
    Sakurai(ArcSakurai),
    Efls,
    Bres(Bres),
    Atkd,
    Reff,
    Reft,
    Aipd,
    W,
    Unknown,
}
