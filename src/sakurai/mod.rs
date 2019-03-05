pub mod fighter_data;
pub mod fighter_data_common;

use byteorder::{BigEndian, ReadBytesExt};

use crate::script::Script;
use crate::script;
use crate::util;
use fighter_data::ArcFighterData;
use fighter_data_common::ArcFighterDataCommon;

pub(crate) fn arc_sakurai(data: &[u8]) -> ArcSakurai {
    let _size                     = (&data[0x00..]).read_i32::<BigEndian>().unwrap();
    let lookup_entry_offset       = (&data[0x04..]).read_i32::<BigEndian>().unwrap();
    let lookup_entry_count        = (&data[0x08..]).read_i32::<BigEndian>().unwrap();
    let section_count             = (&data[0x0c..]).read_i32::<BigEndian>().unwrap();
    let external_subroutine_count = (&data[0x10..]).read_i32::<BigEndian>().unwrap();

    let lookup_entries_offset = ARC_SAKURAI_HEADER_SIZE + lookup_entry_offset as usize;
    let sections_offset = lookup_entries_offset + lookup_entry_count as usize * 4;
    let external_subroutines_offset = sections_offset + section_count as usize * ARC_SAKURAI_SECTION_HEADER_SIZE;
    let string_table_offset = external_subroutines_offset + external_subroutine_count as usize * EXTERNAL_SUBROUTINE_SIZE;

    let mut lookup_entries = vec!();
    for i in 0..lookup_entry_count {
        let offset = lookup_entry_offset as usize + i as usize * 4;
        let entry_offset = (&data[offset..]).read_i32::<BigEndian>().unwrap();
        lookup_entries.push(entry_offset);
    }

    let mut sections = vec!();
    for i in 0..section_count {
        let offset = sections_offset + i as usize * ARC_SAKURAI_SECTION_HEADER_SIZE;
        let data_offset   = (&data[offset     ..]).read_i32::<BigEndian>().unwrap();
        let string_offset = (&data[offset + 4 ..]).read_i32::<BigEndian>().unwrap();
        let name = String::from(util::parse_str(&data[string_table_offset + string_offset as usize ..]).unwrap());

        let parent_data = &data[ARC_SAKURAI_HEADER_SIZE ..];
        let data = &data[ARC_SAKURAI_HEADER_SIZE + data_offset as usize..];
        let mut section_data = match name.as_str() {
            "data"       => SectionData::FighterData(fighter_data::arc_fighter_data(parent_data, data)),
            "dataCommon" => SectionData::FighterDataCommon(fighter_data_common::arc_fighter_data_common(parent_data, data)),
            _            => SectionData::None
        };

        if name.starts_with("gameAnimCmd_") || name.starts_with("effectAnimCmd_") || name.starts_with("statusAnimCmdGroup_") || name.starts_with("statusAnimCmdPre_") {
            section_data = SectionData::Script(SectionScript {
                name:   name.clone(),
                script: script::new_script(parent_data, data_offset)
            });
        }
        sections.push(ArcSakuraiSection { name, data: section_data });
    }

    let mut external_subroutines = vec!();
    for i in 0..external_subroutine_count {
        let offset = external_subroutines_offset + i as usize * EXTERNAL_SUBROUTINE_SIZE;
        let data_offset   = (&data[offset     ..]).read_i32::<BigEndian>().unwrap();
        let string_offset = (&data[offset + 4 ..]).read_i32::<BigEndian>().unwrap();
        let name = String::from(util::parse_str(&data[string_table_offset + string_offset as usize ..]).unwrap());

        // Some of these point directly at the offset argument used by a script, we compare it with the offsets location later on.
        // Others do not, I don't know what they are pointing at.
        // Use this to investigate what the other data is.
        //let data = &data[ARC_SAKURAI_HEADER_SIZE + data_offset as usize - 4..]; // start 4 bytes behind to see the argument type
        //info!("{} {} {}", util::hex_dump(&data[..0x50]), data_offset, name);

        external_subroutines.push(ExternalSubroutine { name, offset: data_offset });
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
    /// TODO: Remove this field when all SectionData's are implemented
    pub name: String,
    pub data: SectionData,
}

#[derive(Debug)]
pub enum SectionData {
    FighterData (ArcFighterData),
    FighterDataCommon (ArcFighterDataCommon),
    Script (SectionScript),
    None,
}

#[derive(Debug)]
pub struct SectionScript {
    pub name:   String,
    pub script: Script,
}

const EXTERNAL_SUBROUTINE_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct ExternalSubroutine {
    pub name: String,
    pub offset: i32,
}
