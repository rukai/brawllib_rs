pub mod fighter_data;
pub mod fighter_data_common;

use byteorder::{BigEndian, ReadBytesExt};

use crate::script::Script;
use crate::script;
use crate::util;
use crate::wii_memory::WiiMemory;

use fighter_data::ArcFighterData;
use fighter_data_common::ArcFighterDataCommon;

pub(crate) fn arc_sakurai(data: &[u8], wii_memory: &WiiMemory) -> ArcSakurai {
    let size                      = (&data[0x00..]).read_i32::<BigEndian>().unwrap();
    let lookup_entry_offset       = (&data[0x04..]).read_i32::<BigEndian>().unwrap();
    let lookup_entry_count        = (&data[0x08..]).read_i32::<BigEndian>().unwrap();
    let section_count             = (&data[0x0c..]).read_i32::<BigEndian>().unwrap();
    let external_subroutine_count = (&data[0x10..]).read_i32::<BigEndian>().unwrap();

    let lookup_entries_offset = ARC_SAKURAI_HEADER_SIZE + lookup_entry_offset as usize;
    let sections_offset = lookup_entries_offset + lookup_entry_count as usize * 4;
    let external_subroutines_offset = sections_offset + section_count as usize * ARC_SAKURAI_SECTION_HEADER_SIZE;
    let string_table_offset = external_subroutines_offset + external_subroutine_count as usize * EXTERNAL_SUBROUTINE_SIZE;

    let parent_data = &data[ARC_SAKURAI_HEADER_SIZE ..];

    let mut lookup_entries = vec!();
    for i in 0..lookup_entry_count {
        let offset = lookup_entry_offset as usize + i as usize * 4;
        let entry_offset = (&data[offset..]).read_i32::<BigEndian>().unwrap();
        lookup_entries.push(entry_offset);
    }

    let mut external_subroutines = vec!();
    for i in 0..external_subroutine_count {
        let mut offsets = vec!();
        let offset = external_subroutines_offset + i as usize * EXTERNAL_SUBROUTINE_SIZE;
        let mut offset_linked_list = (&data[offset..]).read_i32::<BigEndian>().unwrap();
        let string_offset = (&data[offset + 4 ..]).read_i32::<BigEndian>().unwrap();
        let name = String::from(util::parse_str(&data[string_table_offset + string_offset as usize ..]).unwrap());

        // The offset_linked_list is a pointer to the offset argument used by a subroutine/goto call that is making an external call.
        // However the since the value in subroutine/goto offset argument has no purpose as its an external call, it is instead used to point to another value subroutine/goto offset argument.
        // This forms a linked list between all the subroutine/goto offset arguments that make the same external call.
        while offset_linked_list > 0 && offset_linked_list < size {
            offsets.push(offset_linked_list);
            offset_linked_list = (&data[ARC_SAKURAI_HEADER_SIZE + offset_linked_list as usize..]).read_i32::<BigEndian>().unwrap();
        }

        external_subroutines.push(ExternalSubroutine { name, offsets });
    }

    let mut sections = vec!();
    for i in 0..section_count {
        let offset = sections_offset + i as usize * ARC_SAKURAI_SECTION_HEADER_SIZE;
        let data_offset   = (&data[offset     ..]).read_u32::<BigEndian>().unwrap();
        let string_offset = (&data[offset + 4 ..]).read_i32::<BigEndian>().unwrap();
        let name = String::from(util::parse_str(&data[string_table_offset + string_offset as usize ..]).unwrap());

        let data = &data[ARC_SAKURAI_HEADER_SIZE + data_offset as usize..];
        let mut section_data = match name.as_str() {
            "data"       => SectionData::FighterData(fighter_data::arc_fighter_data(parent_data, data, wii_memory)),
            "dataCommon" => SectionData::FighterDataCommon(fighter_data_common::arc_fighter_data_common(parent_data, data, wii_memory)),
            _            => SectionData::None
        };

        if name.starts_with("gameAnimCmd_") || name.starts_with("effectAnimCmd_") || name.starts_with("statusAnimCmdGroup_") || name.starts_with("statusAnimCmdPre_") {
            section_data = SectionData::Script(SectionScript {
                name:   name.clone(),
                script: script::new_script(parent_data, data_offset, wii_memory),
            });
        }
        sections.push(ArcSakuraiSection { name, data: section_data });
    }

    // locate all script fragments called by subroutines etc.
    let mut all_scripts = vec!();
    let mut all_scripts_sub = vec!();
    for section in &sections {
        match &section.data {
            SectionData::FighterData(data) => {
                all_scripts.push(data.entry_actions.as_slice());
                all_scripts.push(data.exit_actions.as_slice());
                all_scripts.push(data.subaction_main.as_slice());
                all_scripts.push(data.subaction_gfx.as_slice());
                all_scripts.push(data.subaction_sfx.as_slice());
                all_scripts.push(data.subaction_other.as_slice());
            }
            SectionData::FighterDataCommon(data_common) => {
                all_scripts.push(data_common.entry_actions.as_slice());
                all_scripts.push(data_common.exit_actions.as_slice());
            }
            SectionData::Script(script) => {
                all_scripts_sub.push(script.script.clone());
            }
            _ => { }
        }
    }
    all_scripts.push(all_scripts_sub.as_slice());

    let ignore_origins: Vec<_> = external_subroutines.iter().flat_map(|x| x.offsets.iter().cloned()).collect();
    let mut fragment_scripts = script::fragment_scripts(parent_data, all_scripts.as_slice(), ignore_origins.as_slice(), wii_memory);
    fragment_scripts.sort_by_key(|x| x.offset);

    ArcSakurai { lookup_entries, sections, external_subroutines, fragment_scripts }
}

const ARC_SAKURAI_HEADER_SIZE: usize = 0x20;
#[derive(Clone, Debug)]
pub struct ArcSakurai {
    lookup_entries:           Vec<i32>,
    pub sections:             Vec<ArcSakuraiSection>,
    pub external_subroutines: Vec<ExternalSubroutine>,
    pub fragment_scripts:     Vec<Script>,
}

const ARC_SAKURAI_SECTION_HEADER_SIZE: usize = 0x8;
#[derive(Clone, Debug)]
pub struct ArcSakuraiSection {
    /// TODO: Remove this field when all SectionData's are implemented
    pub name: String,
    pub data: SectionData,
}

#[derive(Clone, Debug)]
pub enum SectionData {
    FighterData (ArcFighterData),
    FighterDataCommon (ArcFighterDataCommon),
    Script (SectionScript),
    None,
}

#[derive(Clone, Debug)]
pub struct SectionScript {
    pub name:   String,
    pub script: Script,
}

const EXTERNAL_SUBROUTINE_SIZE: usize = 0x8;
#[derive(Clone, Debug)]
pub struct ExternalSubroutine {
    pub name: String,
    pub offsets: Vec<i32>,
}
