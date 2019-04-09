use fancy_slice::FancySlice;

use crate::script::Script;
use crate::script;
use crate::util;
use crate::wii_memory::WiiMemory;

pub(crate) fn arc_fighter_data_common(parent_data: FancySlice, data: FancySlice, wii_memory: &WiiMemory) -> ArcFighterDataCommon {
    let global_ics           = data.i32_be(0x00);
    let global_ics_sse       = data.i32_be(0x04);
    let ics                  = data.i32_be(0x08);
    let ics_sse              = data.i32_be(0x0c);
    let entry_actions_start  = data.i32_be(0x10);
    let exit_actions_start   = data.i32_be(0x14);
    let flash_overlay_array  = data.i32_be(0x18);
    let unk1                 = data.i32_be(0x1c);
    let unk2                 = data.i32_be(0x20);
    let unk3                 = data.i32_be(0x24);
    let unk4                 = data.i32_be(0x28);
    let unk5                 = data.i32_be(0x2c);
    let unk6                 = data.i32_be(0x30);
    let unk7                 = data.i32_be(0x34);
    let unk8                 = data.i32_be(0x38);
    let unk9                 = data.i32_be(0x3c);
    let unk10                = data.i32_be(0x40);
    let unk11                = data.i32_be(0x44);
    let unk12                = data.i32_be(0x48);
    let flash_overlay_offset = data.i32_be(0x4c);
    let screen_tints         = data.i32_be(0x50);
    let leg_bones            = data.i32_be(0x54);
    let unk13                = data.i32_be(0x58);
    let unk14                = data.i32_be(0x5c);
    let unk15                = data.i32_be(0x60);
    let unk16                = data.i32_be(0x64);

    let sizes = get_sizes(data);

    let entry_actions_num = sizes.iter().find(|x| x.offset == entry_actions_start as usize).unwrap().size / 4; // divide by integer size
    let entry_actions = script::scripts(parent_data, parent_data.relative_fancy_slice(entry_actions_start as usize ..), entry_actions_num, wii_memory);
    let exit_actions = script::scripts(parent_data, parent_data.relative_fancy_slice(exit_actions_start as usize ..), entry_actions_num, wii_memory);

    let leg_bones_left_list = util::list_offset(parent_data.relative_fancy_slice(leg_bones as usize..));
    let mut leg_bones_left = vec!();
    for i in 0..leg_bones_left_list.count as usize {
        let string_offset = parent_data.i32_be(leg_bones_left_list.start_offset as usize + i * 4);
        leg_bones_left.push(parent_data.str(string_offset as usize).unwrap().to_string());
    }

    let leg_bones_right_list = util::list_offset(parent_data.relative_fancy_slice(leg_bones as usize + util::LIST_OFFSET_SIZE ..));
    let mut leg_bones_right = vec!();
    for i in 0..leg_bones_right_list.count as usize {
        let string_offset = parent_data.i32_be(leg_bones_right_list.start_offset as usize + i * 4);
        leg_bones_right.push(parent_data.str(string_offset as usize).unwrap().to_string());
    }

    ArcFighterDataCommon {
        global_ics,
        global_ics_sse,
        ics,
        ics_sse,
        entry_actions,
        exit_actions,
        flash_overlay_array,
        unk1,
        unk2,
        unk3,
        unk4,
        unk5,
        unk6,
        unk7,
        unk8,
        unk9,
        unk10,
        unk11,
        unk12,
        flash_overlay_offset,
        screen_tints,
        leg_bones_left,
        leg_bones_right,
        unk13,
        unk14,
        unk15,
        unk16,
    }
}

const _ARC_FIGHTER_DATA_COMMON_HEADER_SIZE: usize = 0x68;
#[derive(Clone, Debug)]
pub struct ArcFighterDataCommon {
    pub global_ics: i32,
    pub global_ics_sse: i32,
    pub ics: i32,
    pub ics_sse: i32,
    pub entry_actions: Vec<Script>,
    pub exit_actions: Vec<Script>,
    pub flash_overlay_array: i32,
    pub unk1: i32,
    pub unk2: i32,
    pub unk3: i32,
    pub unk4: i32,
    pub unk5: i32,
    pub unk6: i32,
    pub unk7: i32,
    pub unk8: i32,
    pub unk9: i32,
    pub unk10: i32,
    pub unk11: i32,
    pub unk12: i32,
    pub flash_overlay_offset: i32,
    pub screen_tints: i32,
    pub leg_bones_left: Vec<String>,
    pub leg_bones_right: Vec<String>,
    pub unk13: i32,
    pub unk14: i32,
    pub unk15: i32,
    pub unk16: i32,
}

struct OffsetSizePair {
    offset: usize,
    size: usize,
}

fn get_sizes(data: FancySlice) -> Vec<OffsetSizePair> {
    let mut pairs = vec!();
    for i in 0..26 {
        let offset = data.i32_be(i * 4) as usize;
        if offset != 0 {
            pairs.push(OffsetSizePair { offset, size: 0 });
        }
    }

    // TODO: Document WHY we modify these offsets, I just copied it from brawlbox
    pairs[2].offset = 1; // Set ICs offset to 1
    pairs.sort_by_key(|x| x.offset);
    pairs[2].offset -= 1;  // Set unk4 offset to -= 1

    // fill in size for most elements
    for i in 0..pairs.len()-1 {
        pairs[i].size = pairs[i + 1].offset - pairs[i].offset
    }

    // Just pop the last element, so if we try to access it we get a panic
    pairs.pop();

    pairs
}
