use byteorder::{BigEndian, ReadBytesExt};

use util;
use misc_section::MiscSection;
use misc_section;

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
        let section_data = if &name == "data" {
            SectionData::FighterData(arc_fighter_data(&data[ARC_SAKURAI_HEADER_SIZE ..], &data[ARC_SAKURAI_HEADER_SIZE + data_offset as usize ..]))
        } else {
            SectionData::None
        };
        sections.push(ArcSakuraiSection { data_offset, string_offset, name, data: section_data });
    }

    ArcSakurai { size, lookup_offset, lookup_entry_count, section_count, external_subroutine_count, sections }
}

struct OffsetSizePair {
    offset: usize,
    size: usize,
}

fn get_sizes(data: &[u8]) -> Vec<OffsetSizePair> {
    let mut pairs = vec!();
    for i in 0..27 {
        pairs.push(OffsetSizePair {
            offset: (&data[i * 4 ..]).read_i32::<BigEndian>().unwrap() as usize,
            size: 0
        });
    }

    pairs.sort_by_key(|x| x.offset);

    for i in 0..26 {
        pairs[i].size = pairs[i + 1].offset - pairs[i].offset;
    }

    pairs
}

fn arc_fighter_data(parent_data: &[u8], data: &[u8]) -> ArcFighterData {
    let sub_action_flags_start     = (&data[0..]).read_i32::<BigEndian>().unwrap();
    let model_visibility_start     = (&data[4..]).read_i32::<BigEndian>().unwrap();
    let attribute_start            = (&data[8..]).read_i32::<BigEndian>().unwrap();
    let sse_attribute_start        = (&data[12..]).read_i32::<BigEndian>().unwrap();
    let misc_section_offset        = (&data[16..]).read_i32::<BigEndian>().unwrap();
    let common_action_flags_start  = (&data[20..]).read_i32::<BigEndian>().unwrap();
    let action_flags_start         = (&data[24..]).read_i32::<BigEndian>().unwrap();
    let _unknown0                  = (&data[28..]).read_i32::<BigEndian>().unwrap();
    let action_interrupts          = (&data[32..]).read_i32::<BigEndian>().unwrap();
    let entry_actions_start        = (&data[36..]).read_i32::<BigEndian>().unwrap();
    let exit_actions_start         = (&data[40..]).read_i32::<BigEndian>().unwrap();
    let action_pre_start           = (&data[44..]).read_i32::<BigEndian>().unwrap();
    let sub_action_main_start      = (&data[48..]).read_i32::<BigEndian>().unwrap();
    let sub_action_gfx_start       = (&data[52..]).read_i32::<BigEndian>().unwrap();
    let sub_action_sfx_start       = (&data[56..]).read_i32::<BigEndian>().unwrap();
    let sub_action_other_start     = (&data[60..]).read_i32::<BigEndian>().unwrap();
    let anchored_item_positions    = (&data[64..]).read_i32::<BigEndian>().unwrap();
    let gooey_bomb_positions       = (&data[58..]).read_i32::<BigEndian>().unwrap();
    let bone_ref1                  = (&data[72..]).read_i32::<BigEndian>().unwrap();
    let bone_ref2                  = (&data[76..]).read_i32::<BigEndian>().unwrap();
    let entry_action_overrides     = (&data[80..]).read_i32::<BigEndian>().unwrap();
    let exit_action_overrides      = (&data[84..]).read_i32::<BigEndian>().unwrap();
    let _unknown1                  = (&data[88..]).read_i32::<BigEndian>().unwrap();
    let samus_arm_cannon_positions = (&data[92..]).read_i32::<BigEndian>().unwrap();
    let _unknown2                  = (&data[96..]).read_i32::<BigEndian>().unwrap();
    let static_articles_start      = (&data[100..]).read_i32::<BigEndian>().unwrap();
    let entry_articles_start       = (&data[104..]).read_i32::<BigEndian>().unwrap();
    let flags1                     = (&data[116..]).read_u32::<BigEndian>().unwrap();
    let flags2                     = (&data[120..]).read_i32::<BigEndian>().unwrap();

    let sizes = get_sizes(data);

    let sub_action_flags_num = sizes.iter().find(|x| x.offset == sub_action_flags_start as usize).unwrap().size / SUB_ACTION_FLAGS_SIZE;
    let sub_action_flags = sub_action_flags(parent_data, &parent_data[sub_action_flags_start as usize ..], sub_action_flags_num);
    let action_flags_num = sizes.iter().find(|x| x.offset == action_flags_start as usize).unwrap().size / ACTION_FLAGS_SIZE;
    let action_flags = action_flags(&parent_data[action_flags_start as usize ..], action_flags_num);
    let attributes = fighter_attributes(&parent_data[attribute_start as usize ..]);
    let misc = misc_section::misc_section(&parent_data[misc_section_offset as usize ..], parent_data);

    ArcFighterData {
        sub_action_flags,
        attributes,
        misc,
        action_flags,
        model_visibility_start,
        sse_attribute_start,
        common_action_flags_start,
        action_interrupts,
        entry_actions_start,
        exit_actions_start,
        action_pre_start,
        sub_action_main_start,
        sub_action_gfx_start,
        sub_action_sfx_start,
        sub_action_other_start,
        anchored_item_positions,
        gooey_bomb_positions,
        bone_ref1,
        bone_ref2,
        entry_action_overrides,
        exit_action_overrides,
        samus_arm_cannon_positions,
        static_articles_start,
        entry_articles_start,
        flags1,
        flags2,
    }
}

fn fighter_attributes(data: &[u8]) -> FighterAttributes {
    FighterAttributes {
        walk_init_vel:                     (&data[0x00..]).read_f32::<BigEndian>().unwrap(),
        walk_acc:                          (&data[0x04..]).read_f32::<BigEndian>().unwrap(),
        walk_max_vel:                      (&data[0x08..]).read_f32::<BigEndian>().unwrap(),
        ground_friction:                   (&data[0x0c..]).read_f32::<BigEndian>().unwrap(),
        dash_init_vel:                     (&data[0x10..]).read_f32::<BigEndian>().unwrap(),
        dash_run_acc_a:                    (&data[0x14..]).read_f32::<BigEndian>().unwrap(),
        dash_run_acc_b:                    (&data[0x18..]).read_f32::<BigEndian>().unwrap(),
        dash_run_term_vel:                 (&data[0x1c..]).read_f32::<BigEndian>().unwrap(),
        grounded_max_x_vel:                (&data[0x24..]).read_f32::<BigEndian>().unwrap(),
        dash_cancel_frame_window:          (&data[0x28..]).read_i32::<BigEndian>().unwrap(),
        guard_on_max_momentum:             (&data[0x2c..]).read_f32::<BigEndian>().unwrap(),
        jump_squat_frames:                 (&data[0x30..]).read_i32::<BigEndian>().unwrap(),
        jump_x_init_vel:                   (&data[0x34..]).read_f32::<BigEndian>().unwrap(),
        jump_y_init_vel:                   (&data[0x38..]).read_f32::<BigEndian>().unwrap(),
        jump_x_vel_ground_mult:            (&data[0x3c..]).read_f32::<BigEndian>().unwrap(),
        jump_x_init_term_vel:              (&data[0x40..]).read_f32::<BigEndian>().unwrap(),
        jump_y_init_vel_short:             (&data[0x44..]).read_f32::<BigEndian>().unwrap(),
        air_jump_x_mult:                   (&data[0x48..]).read_f32::<BigEndian>().unwrap(),
        air_jump_y_mult:                   (&data[0x4c..]).read_f32::<BigEndian>().unwrap(),
        footstool_init_vel:                (&data[0x50..]).read_f32::<BigEndian>().unwrap(),
        footstool_init_vel_short:          (&data[0x54..]).read_f32::<BigEndian>().unwrap(),
        meteor_cancel_delay:               (&data[0x5c..]).read_f32::<BigEndian>().unwrap(),
        num_jumps:                         (&data[0x60..]).read_u32::<BigEndian>().unwrap(),
        gravity:                           (&data[0x64..]).read_f32::<BigEndian>().unwrap(),
        term_vel:                          (&data[0x68..]).read_f32::<BigEndian>().unwrap(),
        air_friction_y:                    (&data[0x6c..]).read_f32::<BigEndian>().unwrap(),
        air_y_term_vel:                    (&data[0x70..]).read_f32::<BigEndian>().unwrap(),
        air_mobility_a:                    (&data[0x74..]).read_f32::<BigEndian>().unwrap(),
        air_mobility_b:                    (&data[0x78..]).read_f32::<BigEndian>().unwrap(),
        air_x_term_mobility:               (&data[0x7c..]).read_f32::<BigEndian>().unwrap(),
        air_friction_x:                    (&data[0x80..]).read_f32::<BigEndian>().unwrap(),
        fastfall_velocity:                 (&data[0x84..]).read_f32::<BigEndian>().unwrap(),
        air_x_term_vel:                    (&data[0x88..]).read_f32::<BigEndian>().unwrap(),
        glide_frame_window:                (&data[0x8c..]).read_u32::<BigEndian>().unwrap(),
        jab2_window:                       (&data[0x94..]).read_f32::<BigEndian>().unwrap(),
        jab3_window:                       (&data[0x98..]).read_f32::<BigEndian>().unwrap(),
        ftilt2_window:                     (&data[0x9c..]).read_f32::<BigEndian>().unwrap(),
        ftilt3_window:                     (&data[0xa0..]).read_f32::<BigEndian>().unwrap(),
        fsmash2_window:                    (&data[0xa4..]).read_f32::<BigEndian>().unwrap(),
        flip_dir_frame:                    (&data[0xa8..]).read_f32::<BigEndian>().unwrap(),
        weight:                            (&data[0xb0..]).read_f32::<BigEndian>().unwrap(),
        size:                              (&data[0xb4..]).read_f32::<BigEndian>().unwrap(),
        results_screen_size:               (&data[0xb8..]).read_f32::<BigEndian>().unwrap(),
        shield_size:                       (&data[0xc4..]).read_f32::<BigEndian>().unwrap(),
        shield_break_vel:                  (&data[0xc8..]).read_f32::<BigEndian>().unwrap(),
        shield_strength:                   (&data[0xcc..]).read_f32::<BigEndian>().unwrap(),
        respawn_platform_size:             (&data[0xd4..]).read_f32::<BigEndian>().unwrap(),
        edge_jump_x_vel:                   (&data[0xf4..]).read_f32::<BigEndian>().unwrap(),
        edge_jump_y_vel:                   (&data[0xfc..]).read_f32::<BigEndian>().unwrap(),
        item_throw_strength:               (&data[0x118..]).read_f32::<BigEndian>().unwrap(),
        projectile_item_move_speed:        (&data[0x128..]).read_f32::<BigEndian>().unwrap(),
        projectile_item_move_speed_dash_f: (&data[0x12c..]).read_f32::<BigEndian>().unwrap(),
        projectile_item_move_speed_dash_b: (&data[0x130..]).read_f32::<BigEndian>().unwrap(),
        light_landing_lag:                 (&data[0x138..]).read_f32::<BigEndian>().unwrap(),
        normal_landing_lag:                (&data[0x13c..]).read_f32::<BigEndian>().unwrap(),
        nair_landing_lag:                  (&data[0x140..]).read_f32::<BigEndian>().unwrap(),
        fair_landing_lag:                  (&data[0x144..]).read_f32::<BigEndian>().unwrap(),
        bair_landing_lag:                  (&data[0x148..]).read_f32::<BigEndian>().unwrap(),
        uair_landing_lag:                  (&data[0x14c..]).read_f32::<BigEndian>().unwrap(),
        dair_landing_lag:                  (&data[0x150..]).read_f32::<BigEndian>().unwrap(),
        term_vel_hard_frames:              (&data[0x154..]).read_u32::<BigEndian>().unwrap(),
        hip_n_bone:                        (&data[0x158..]).read_u32::<BigEndian>().unwrap(),
        tag_height_value:                  (&data[0x15c..]).read_f32::<BigEndian>().unwrap(),
        walljump_x_vel:                    (&data[0x164..]).read_f32::<BigEndian>().unwrap(),
        walljump_y_vel:                    (&data[0x168..]).read_f32::<BigEndian>().unwrap(),
        lhand_n_bone:                      (&data[0x180..]).read_u32::<BigEndian>().unwrap(),
        rhand_n_bone:                      (&data[0x184..]).read_u32::<BigEndian>().unwrap(),
        water_y_acc:                       (&data[0x18c..]).read_f32::<BigEndian>().unwrap(),
        spit_star_size:                    (&data[0x1a4..]).read_f32::<BigEndian>().unwrap(),
        spit_star_damage:                  (&data[0x1a8..]).read_u32::<BigEndian>().unwrap(),
        egg_size:                          (&data[0x1ac..]).read_f32::<BigEndian>().unwrap(),
        hip_n_bone2:                       (&data[0x1cc..]).read_u32::<BigEndian>().unwrap(),
        x_rot_n_bone:                      (&data[0x1e0..]).read_u32::<BigEndian>().unwrap(),
    }
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
    None,
}

const _ARC_FIGHTER_DATA_HEADER_SIZE: usize = 0x7c;
#[derive(Debug)]
pub struct ArcFighterData {
    pub sub_action_flags: Vec<SubActionFlags>,
    pub attributes: FighterAttributes,
    pub misc: MiscSection,
    pub action_flags: Vec<ActionFlags>,
    model_visibility_start: i32,
    sse_attribute_start: i32,
    common_action_flags_start: i32,
    action_interrupts: i32,
    entry_actions_start: i32,
    exit_actions_start: i32,
    action_pre_start: i32,
    sub_action_main_start: i32,
    sub_action_gfx_start: i32,
    sub_action_sfx_start: i32,
    sub_action_other_start: i32,
    anchored_item_positions: i32,
    gooey_bomb_positions: i32,
    bone_ref1: i32,
    bone_ref2: i32,
    entry_action_overrides: i32,
    exit_action_overrides: i32,
    samus_arm_cannon_positions: i32,
    static_articles_start: i32,
    entry_articles_start: i32,
    flags1: u32,
    flags2: i32,
}

#[derive(Clone, Debug)]
pub struct FighterAttributes {
    pub walk_init_vel: f32,
    pub walk_acc: f32,
    pub walk_max_vel: f32,
    pub ground_friction: f32,
    pub dash_init_vel: f32,
    pub dash_run_acc_a: f32,
    pub dash_run_acc_b: f32,
    pub dash_run_term_vel: f32,
    pub grounded_max_x_vel: f32,
    pub dash_cancel_frame_window: i32, // spreadsheet is unsure
    pub guard_on_max_momentum: f32,
    pub jump_squat_frames: i32,
    pub jump_x_init_vel: f32,
    pub jump_y_init_vel: f32,
    pub jump_x_vel_ground_mult: f32,
    pub jump_x_init_term_vel: f32, // TODO: does melee include this max in name?
    pub jump_y_init_vel_short: f32,
    pub air_jump_x_mult: f32,
    pub air_jump_y_mult: f32,
    pub footstool_init_vel: f32,
    pub footstool_init_vel_short: f32,
    pub meteor_cancel_delay: f32,
    pub num_jumps: u32,
    pub gravity: f32,
    pub term_vel: f32,
    pub air_friction_y: f32,
    pub air_y_term_vel: f32,
    pub air_mobility_a: f32,
    pub air_mobility_b: f32,
    pub air_x_term_mobility: f32,
    pub air_friction_x: f32,
    pub fastfall_velocity: f32,
    pub air_x_term_vel: f32,
    pub glide_frame_window: u32,
    pub jab2_window: f32,
    pub jab3_window: f32,
    pub ftilt2_window: f32,
    pub ftilt3_window: f32,
    pub fsmash2_window: f32,
    pub flip_dir_frame: f32,
    pub weight: f32,
    pub size: f32,
    pub results_screen_size: f32,
    pub shield_size: f32,
    pub shield_break_vel: f32,
    pub shield_strength: f32,
    pub respawn_platform_size: f32,
    pub edge_jump_x_vel: f32,
    pub edge_jump_y_vel: f32,
    pub item_throw_strength: f32,
    pub projectile_item_move_speed: f32,
    pub projectile_item_move_speed_dash_f: f32,
    pub projectile_item_move_speed_dash_b: f32,
    pub light_landing_lag: f32,
    pub normal_landing_lag: f32,
    pub nair_landing_lag: f32,
    pub fair_landing_lag: f32,
    pub bair_landing_lag: f32,
    pub uair_landing_lag: f32,
    pub dair_landing_lag: f32,
    pub term_vel_hard_frames: u32,
    pub hip_n_bone: u32, // spreadsheet is unsure
    pub tag_height_value: f32,
    pub walljump_x_vel: f32, // used for normal walljumps and walljump techs
    pub walljump_y_vel: f32, // used for normal walljumps and walljump techs
    pub lhand_n_bone: u32,
    pub rhand_n_bone: u32,
    pub water_y_acc: f32,
    pub spit_star_size: f32,
    pub spit_star_damage: u32,
    pub egg_size: f32,
    pub hip_n_bone2: u32,
    pub x_rot_n_bone: u32, // bone to be grabbed from?
}

bitflags! {
    pub struct AnimationFlags: u8 {
        const NONE                      = 0x0;
        const NO_OUT_TRANSITION         = 0x1;
        const LOOP                      = 0x2;
        const MOVES_CHARACTER           = 0x4;
        const FIXED_TRANSLATION         = 0x8;
        const FIXED_ROTATION            = 0x10;
        const FIXED_SCALE               = 0x20;
        const TRANSITION_OUT_FROM_START = 0x40;
        const UNKNOWN                   = 0x80;
    }
}

fn sub_action_flags(parent_data: &[u8], data: &[u8], num: usize) -> Vec<SubActionFlags> {
    let mut result = vec!();
    let num = num + 1;
    for i in 0..num {
        let in_translation_time = data[i * SUB_ACTION_FLAGS_SIZE + 0];
        let animation_flags_int = data[i * SUB_ACTION_FLAGS_SIZE + 1];
        //  padding             (&data[i * SUB_ACTION_FLAGS_SIZE + 2..]).read_u16
        let string_offset =     (&data[i * SUB_ACTION_FLAGS_SIZE + 4..]).read_i32::<BigEndian>().unwrap();

        let animation_flags = AnimationFlags::from_bits(animation_flags_int).unwrap();
        let name = if string_offset == 0 {
            String::new()
        } else {
            util::parse_str(&parent_data[string_offset as usize ..]).unwrap().to_string()
        };

        result.push(SubActionFlags {
            in_translation_time,
            animation_flags,
            name,
        });
    }
    result
}

const SUB_ACTION_FLAGS_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct SubActionFlags {
    pub in_translation_time: u8,
    pub animation_flags:     AnimationFlags,
    pub name:                String,
}

// TODO: This is also a thing but I wont worry about it for now.
// I think it will go in a top level mod script
//struct SubActionEntry {
//    animation_flags:     AnimationFlags,
//    in_translation_time: u8,
//    string_offset:       i32,
//    main:                Script,
//    sfx:                 Script,
//    gfx:                 Script,
//    other:               Script,
//}
//
//struct Script { }

fn action_flags(data: &[u8], num: usize) -> Vec<ActionFlags> {
    let mut result = vec!();
    for i in 0..num {
        result.push(ActionFlags {
            flag1: (&data[i * ACTION_FLAGS_SIZE + 0x0..]).read_u32::<BigEndian>().unwrap(),
            flag2: (&data[i * ACTION_FLAGS_SIZE + 0x4..]).read_u32::<BigEndian>().unwrap(),
            flag3: (&data[i * ACTION_FLAGS_SIZE + 0x8..]).read_u32::<BigEndian>().unwrap(),
            flag4: (&data[i * ACTION_FLAGS_SIZE + 0xc..]).read_u32::<BigEndian>().unwrap(),
        });
    }
    result
}

const ACTION_FLAGS_SIZE: usize = 0x10;
#[derive(Debug)]
pub struct ActionFlags {
    pub flag1: u32,
    pub flag2: u32,
    pub flag3: u32,
    pub flag4: u32,
}
