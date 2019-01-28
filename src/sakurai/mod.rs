pub mod fighter_data;
pub mod fighter_data_common;

use byteorder::{BigEndian, ReadBytesExt};

use crate::util;
use fighter_data::ArcFighterData;
use fighter_data_common::ArcFighterDataCommon;

pub(crate) fn arc_sakurai(data: &[u8]) -> ArcSakurai {
    let size                      = (&data[0x0..]).read_i32::<BigEndian>().unwrap();
    let lookup_offset             = (&data[0x4..]).read_i32::<BigEndian>().unwrap();
    let lookup_entry_count        = (&data[0x8..]).read_i32::<BigEndian>().unwrap();
    let section_count             = (&data[0xc..]).read_i32::<BigEndian>().unwrap();
    let external_subroutine_count = (&data[0x10..]).read_i32::<BigEndian>().unwrap();
    let mut sections = vec!();

    let lookup_entries_index = ARC_SAKURAI_HEADER_SIZE + lookup_offset as usize;
    let sections_index = lookup_entries_index + lookup_entry_count as usize * 4;
    let external_subroutines_index = sections_index + section_count as usize * 8;
    let string_table_index = external_subroutines_index + external_subroutine_count as usize * 8;

    for i in 0..section_count {
        let offset = sections_index + i as usize * ARC_SAKURAI_SECTION_HEADER_SIZE;
        let data_offset   = (&data[offset     ..]).read_i32::<BigEndian>().unwrap();
        let string_offset = (&data[offset + 4 ..]).read_i32::<BigEndian>().unwrap();
        let name = String::from(util::parse_str(&data[string_table_index + string_offset as usize ..]).unwrap());

        let parent_data = &data[ARC_SAKURAI_HEADER_SIZE ..];
        let data = &data[ARC_SAKURAI_HEADER_SIZE + data_offset as usize..];
        let section_data = match name.as_str() {
            "data"       => SectionData::FighterData(fighter_data::arc_fighter_data(parent_data, data)),
            "dataCommon" => SectionData::FighterDataCommon(fighter_data_common::arc_fighter_data_common(parent_data, data)),
            _            => SectionData::None
        };
        sections.push(ArcSakuraiSection { data_offset, string_offset, name, data: section_data });
    }

    ArcSakurai { size, lookup_offset, lookup_entry_count, section_count, external_subroutine_count, sections }
}

const ARC_SAKURAI_HEADER_SIZE: usize = 0x20;
#[derive(Debug)]
pub struct ArcSakurai {
    size: i32,
    lookup_offset: i32,
    lookup_entry_count: i32,
    section_count: i32,
    external_subroutine_count: i32,
    pub sections: Vec<ArcSakuraiSection>,
}

const ARC_SAKURAI_SECTION_HEADER_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct ArcSakuraiSection {
    data_offset: i32,
    string_offset: i32,
    name: String,
    pub data: SectionData,
}

#[derive(Debug)]
pub enum SectionData {
    FighterData (ArcFighterData),
    FighterDataCommon (ArcFighterDataCommon),
    None,
}

