pub mod fighter_data;
pub mod fighter_data_common;

use byteorder::{BigEndian, ReadBytesExt};

use crate::util;
use fighter_data::ArcFighterData;
use fighter_data_common::ArcFighterDataCommon;

pub(crate) fn arc_sakurai(data: &[u8]) -> ArcSakurai {
    let _size                      = (&data[0x0..]).read_i32::<BigEndian>().unwrap();
    let lookup_entry_offset       = (&data[0x4..]).read_i32::<BigEndian>().unwrap();
    let lookup_entry_count        = (&data[0x8..]).read_i32::<BigEndian>().unwrap();
    let section_count             = (&data[0xc..]).read_i32::<BigEndian>().unwrap();
    let external_subroutine_count = (&data[0x10..]).read_i32::<BigEndian>().unwrap();

    let lookup_entries_offset = ARC_SAKURAI_HEADER_SIZE + lookup_entry_offset as usize;
    let sections_offset = lookup_entries_offset + lookup_entry_count as usize * 4;
    let external_subroutines_offset = sections_offset + section_count as usize * 8;
    let string_table_offset = external_subroutines_offset + external_subroutine_count as usize * 8;

    let mut lookup_entries = vec!();
    for i in 0..lookup_entry_count {
        let offset = lookup_entry_offset as usize + i as usize * 4;
        let entry_offset = (&data[offset..]).read_i32::<BigEndian>().unwrap();
        lookup_entries.push(entry_offset);
    }

    let mut external_subroutines = vec!();
    for i in 0..external_subroutine_count {
        let offset = external_subroutines_offset + i as usize * EXTERNAL_SUBROUTINE_SIZE;
        let data_offset   = (&data[offset     ..]).read_i32::<BigEndian>().unwrap();
        let string_offset = (&data[offset + 4 ..]).read_i32::<BigEndian>().unwrap();
        let name = String::from(util::parse_str(&data[string_table_offset + string_offset as usize ..]).unwrap());

        external_subroutines.push(ExternalSubroutine { name, data_offset });
    }

    let mut sections = vec!();
    for i in 0..section_count {
        let offset = sections_offset + i as usize * ARC_SAKURAI_SECTION_HEADER_SIZE;
        let data_offset   = (&data[offset     ..]).read_i32::<BigEndian>().unwrap();
        let string_offset = (&data[offset + 4 ..]).read_i32::<BigEndian>().unwrap();
        let name = String::from(util::parse_str(&data[string_table_offset + string_offset as usize ..]).unwrap());

        let parent_data = &data[ARC_SAKURAI_HEADER_SIZE ..];
        let data = &data[ARC_SAKURAI_HEADER_SIZE + data_offset as usize..];
        let section_data = match name.as_str() {
            "data"       => SectionData::FighterData(fighter_data::arc_fighter_data(parent_data, data)),
            "dataCommon" => SectionData::FighterDataCommon(fighter_data_common::arc_fighter_data_common(parent_data, data)),
            _            => SectionData::None
        };
        sections.push(ArcSakuraiSection { name, data: section_data });
    }

    ArcSakurai { lookup_entries, sections, external_subroutines }
}

const ARC_SAKURAI_HEADER_SIZE: usize = 0x20;
#[derive(Debug)]
pub struct ArcSakurai {
    lookup_entries:           Vec<i32>,
    pub sections:             Vec<ArcSakuraiSection>,
    pub external_subroutines: Vec<ExternalSubroutine>,
}

const ARC_SAKURAI_SECTION_HEADER_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct ArcSakuraiSection {
    pub name: String,
    pub data: SectionData,
}

#[derive(Debug)]
pub enum SectionData {
    FighterData (ArcFighterData),
    FighterDataCommon (ArcFighterDataCommon),
    None,
}

const EXTERNAL_SUBROUTINE_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct ExternalSubroutine {
    pub name: String,
    pub data_offset: i32,
}
