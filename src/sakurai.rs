use byteorder::{BigEndian, ReadBytesExt};

use util;
use misc_section::MiscSection;
use misc_section;

pub(crate) fn arc_sakurai(data: &[u8]) -> ArcSakurai {
    let size                      = (&data[0..4]).read_i32::<BigEndian>().unwrap();
    let lookup_offset             = (&data[4..8]).read_i32::<BigEndian>().unwrap();
    let lookup_entry_count        = (&data[8..12]).read_i32::<BigEndian>().unwrap();
    let section_count             = (&data[12..16]).read_i32::<BigEndian>().unwrap();
    let external_subroutine_count = (&data[16..20]).read_i32::<BigEndian>().unwrap();
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
        let mut section_data = SectionData::None;

        if &name == "data" {
            let header = arc_fighter_data(&data[ARC_SAKURAI_HEADER_SIZE + data_offset as usize ..]);
            let attributes = fighter_attributes(&data[ARC_SAKURAI_HEADER_SIZE + header.attribute_start as usize ..]);
            let misc = misc_section::misc_section(&data[ARC_SAKURAI_HEADER_SIZE + header.misc_section_offset as usize ..], &data[ARC_SAKURAI_HEADER_SIZE ..]);
            section_data = SectionData::FighterData { header, attributes, misc };
        }
        sections.push(ArcSakuraiSection { data_offset, string_offset, name, data: section_data });
    }

    ArcSakurai { size, lookup_offset, lookup_entry_count, section_count, external_subroutine_count, sections }
}

fn arc_fighter_data(data: &[u8]) -> ArcFighterData {
    ArcFighterData {
        subaction_flags_start:      (&data[0..4]).read_i32::<BigEndian>().unwrap(),
        model_visibility_start:     (&data[4..8]).read_i32::<BigEndian>().unwrap(),
        attribute_start:            (&data[8..12]).read_i32::<BigEndian>().unwrap(),
        sse_attribute_start:        (&data[12..16]).read_i32::<BigEndian>().unwrap(),
        misc_section_offset:        (&data[16..20]).read_i32::<BigEndian>().unwrap(),
        common_action_flags_start:  (&data[20..24]).read_i32::<BigEndian>().unwrap(),
        action_flags_start:         (&data[24..28]).read_i32::<BigEndian>().unwrap(),
        action_interrupts:          (&data[32..36]).read_i32::<BigEndian>().unwrap(),
        entry_actions_start:        (&data[36..40]).read_i32::<BigEndian>().unwrap(),
        exit_actions_start:         (&data[40..44]).read_i32::<BigEndian>().unwrap(),
        action_pre_start:           (&data[44..48]).read_i32::<BigEndian>().unwrap(),
        subaction_main_start:       (&data[48..52]).read_i32::<BigEndian>().unwrap(),
        subaction_gfx_start:        (&data[52..56]).read_i32::<BigEndian>().unwrap(),
        subaction_sfx_start:        (&data[56..60]).read_i32::<BigEndian>().unwrap(),
        subaction_other_start:      (&data[60..64]).read_i32::<BigEndian>().unwrap(),
        anchored_item_positions:    (&data[64..68]).read_i32::<BigEndian>().unwrap(),
        gooey_bomb_positions:       (&data[58..72]).read_i32::<BigEndian>().unwrap(),
        bone_ref1:                  (&data[72..76]).read_i32::<BigEndian>().unwrap(),
        bone_ref2:                  (&data[76..80]).read_i32::<BigEndian>().unwrap(),
        entry_action_overrides:     (&data[80..84]).read_i32::<BigEndian>().unwrap(),
        exit_action_overrides:      (&data[84..88]).read_i32::<BigEndian>().unwrap(),
        samus_arm_cannon_positions: (&data[92..96]).read_i32::<BigEndian>().unwrap(),
        static_articles_start:      (&data[100..104]).read_i32::<BigEndian>().unwrap(),
        entry_articles_start:       (&data[104..108]).read_i32::<BigEndian>().unwrap(),
        flags1:                     (&data[116..120]).read_u32::<BigEndian>().unwrap(),
        flags2:                     (&data[120..124]).read_i32::<BigEndian>().unwrap(),
    }
}

fn fighter_attributes(data: &[u8]) -> FighterAttributes {
    FighterAttributes {
        walk_init_vel:                     (&data[0x00..0x04]).read_f32::<BigEndian>().unwrap(),
        walk_acc:                          (&data[0x04..0x08]).read_f32::<BigEndian>().unwrap(),
        walk_max_vel:                      (&data[0x08..0x0c]).read_f32::<BigEndian>().unwrap(),
        ground_friction:                   (&data[0x0c..0x10]).read_f32::<BigEndian>().unwrap(),
        dash_init_vel:                     (&data[0x10..0x14]).read_f32::<BigEndian>().unwrap(),
        dash_run_acc_a:                    (&data[0x14..0x18]).read_f32::<BigEndian>().unwrap(),
        dash_run_acc_b:                    (&data[0x18..0x1c]).read_f32::<BigEndian>().unwrap(),
        dash_run_term_vel:                 (&data[0x1c..0x20]).read_f32::<BigEndian>().unwrap(),
        grounded_max_x_vel:                (&data[0x24..0x28]).read_f32::<BigEndian>().unwrap(),
        dash_cancel_frame_window:          (&data[0x28..0x2c]).read_i32::<BigEndian>().unwrap(),
        guard_on_max_momentum:             (&data[0x2c..0x30]).read_f32::<BigEndian>().unwrap(),
        jump_squat_frames:                 (&data[0x30..0x34]).read_i32::<BigEndian>().unwrap(),
        jump_x_init_vel:                   (&data[0x34..0x38]).read_f32::<BigEndian>().unwrap(),
        jump_y_init_vel:                   (&data[0x38..0x3c]).read_f32::<BigEndian>().unwrap(),
        jump_x_vel_ground_mult:            (&data[0x3c..0x40]).read_f32::<BigEndian>().unwrap(),
        jump_x_init_term_vel:              (&data[0x40..0x44]).read_f32::<BigEndian>().unwrap(),
        jump_y_init_vel_short:             (&data[0x44..0x48]).read_f32::<BigEndian>().unwrap(),
        air_jump_x_mult:                   (&data[0x48..0x4c]).read_f32::<BigEndian>().unwrap(),
        air_jump_y_mult:                   (&data[0x4c..0x50]).read_f32::<BigEndian>().unwrap(),
        footstool_init_vel:                (&data[0x50..0x54]).read_f32::<BigEndian>().unwrap(),
        footstool_init_vel_short:          (&data[0x54..0x58]).read_f32::<BigEndian>().unwrap(),
        meteor_cancel_delay:               (&data[0x5c..0x60]).read_f32::<BigEndian>().unwrap(),
        num_jumps:                         (&data[0x60..0x64]).read_u32::<BigEndian>().unwrap(),
        gravity:                           (&data[0x64..0x68]).read_f32::<BigEndian>().unwrap(),
        term_vel:                          (&data[0x68..0x6c]).read_f32::<BigEndian>().unwrap(),
        air_friction_y:                    (&data[0x6c..0x70]).read_f32::<BigEndian>().unwrap(),
        air_y_term_vel:                    (&data[0x70..0x74]).read_f32::<BigEndian>().unwrap(),
        air_mobility_a:                    (&data[0x74..0x78]).read_f32::<BigEndian>().unwrap(),
        air_mobility_b:                    (&data[0x78..0x7c]).read_f32::<BigEndian>().unwrap(),
        air_x_term_mobility:               (&data[0x7c..0x80]).read_f32::<BigEndian>().unwrap(),
        air_friction_x:                    (&data[0x80..0x84]).read_f32::<BigEndian>().unwrap(),
        fastfall_velocity:                 (&data[0x84..0x88]).read_f32::<BigEndian>().unwrap(),
        air_x_term_vel:                    (&data[0x88..0x8c]).read_f32::<BigEndian>().unwrap(),
        glide_frame_window:                (&data[0x8c..0x90]).read_u32::<BigEndian>().unwrap(),
        jab2_window:                       (&data[0x94..0x98]).read_f32::<BigEndian>().unwrap(),
        jab3_window:                       (&data[0x98..0x9c]).read_f32::<BigEndian>().unwrap(),
        ftilt2_window:                     (&data[0x9c..0xa0]).read_f32::<BigEndian>().unwrap(),
        ftilt3_window:                     (&data[0xa0..0xa4]).read_f32::<BigEndian>().unwrap(),
        fsmash2_window:                    (&data[0xa4..0xa8]).read_f32::<BigEndian>().unwrap(),
        flip_dir_frame:                    (&data[0xa8..0xac]).read_f32::<BigEndian>().unwrap(),
        weight:                            (&data[0xb0..0xb4]).read_f32::<BigEndian>().unwrap(),
        size:                              (&data[0xb4..0xb8]).read_f32::<BigEndian>().unwrap(),
        results_screen_size:               (&data[0xb8..0xbc]).read_f32::<BigEndian>().unwrap(),
        shield_size:                       (&data[0xc4..0xc8]).read_f32::<BigEndian>().unwrap(),
        shield_break_vel:                  (&data[0xc8..0xcc]).read_f32::<BigEndian>().unwrap(),
        shield_strength:                   (&data[0xcc..0xd0]).read_f32::<BigEndian>().unwrap(),
        respawn_platform_size:             (&data[0xd4..0xd8]).read_f32::<BigEndian>().unwrap(),
        edge_jump_x_vel:                   (&data[0xf4..0xf8]).read_f32::<BigEndian>().unwrap(),
        edge_jump_y_vel:                   (&data[0xfc..0x100]).read_f32::<BigEndian>().unwrap(),
        item_throw_strength:               (&data[0x118..0x11c]).read_f32::<BigEndian>().unwrap(),
        projectile_item_move_speed:        (&data[0x128..0x12c]).read_f32::<BigEndian>().unwrap(),
        projectile_item_move_speed_dash_f: (&data[0x12c..0x130]).read_f32::<BigEndian>().unwrap(),
        projectile_item_move_speed_dash_b: (&data[0x130..0x134]).read_f32::<BigEndian>().unwrap(),
        light_landing_lag:                 (&data[0x138..0x13c]).read_f32::<BigEndian>().unwrap(),
        normal_landing_lag:                (&data[0x13c..0x140]).read_f32::<BigEndian>().unwrap(),
        nair_landing_lag:                  (&data[0x140..0x144]).read_f32::<BigEndian>().unwrap(),
        fair_landing_lag:                  (&data[0x144..0x148]).read_f32::<BigEndian>().unwrap(),
        bair_landing_lag:                  (&data[0x148..0x14c]).read_f32::<BigEndian>().unwrap(),
        uair_landing_lag:                  (&data[0x14c..0x150]).read_f32::<BigEndian>().unwrap(),
        dair_landing_lag:                  (&data[0x150..0x154]).read_f32::<BigEndian>().unwrap(),
        term_vel_hard_frames:              (&data[0x154..0x158]).read_u32::<BigEndian>().unwrap(),
        hip_n_bone:                        (&data[0x158..0x15c]).read_u32::<BigEndian>().unwrap(),
        tag_height_value:                  (&data[0x15c..0x160]).read_f32::<BigEndian>().unwrap(),
        walljump_x_vel:                    (&data[0x164..0x168]).read_f32::<BigEndian>().unwrap(),
        walljump_y_vel:                    (&data[0x168..0x16c]).read_f32::<BigEndian>().unwrap(),
        lhand_n_bone:                      (&data[0x180..0x184]).read_u32::<BigEndian>().unwrap(),
        rhand_n_bone:                      (&data[0x184..0x188]).read_u32::<BigEndian>().unwrap(),
        water_y_acc:                       (&data[0x18c..0x190]).read_f32::<BigEndian>().unwrap(),
        spit_star_size:                    (&data[0x1a4..0x1a8]).read_f32::<BigEndian>().unwrap(),
        spit_star_damage:                  (&data[0x1a8..0x1ac]).read_u32::<BigEndian>().unwrap(),
        egg_size:                          (&data[0x1ac..0x1b0]).read_f32::<BigEndian>().unwrap(),
        hip_n_bone2:                       (&data[0x1cc..0x1d0]).read_u32::<BigEndian>().unwrap(),
        x_rot_n_bone:                      (&data[0x1e0..0x1e4]).read_u32::<BigEndian>().unwrap(),
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
    FighterData { header: ArcFighterData, attributes: FighterAttributes, misc: MiscSection },
    None,
}

const _ARC_FIGHTER_DATA_HEADER_SIZE: usize = 0x7c;
#[derive(Debug)]
pub struct ArcFighterData {
    subaction_flags_start: i32,
    model_visibility_start: i32,
    attribute_start: i32,
    sse_attribute_start: i32,
    misc_section_offset: i32,
    common_action_flags_start: i32,
    action_flags_start: i32,
    action_interrupts: i32,
    entry_actions_start: i32,
    exit_actions_start: i32,
    action_pre_start: i32,
    subaction_main_start: i32,
    subaction_gfx_start: i32,
    subaction_sfx_start: i32,
    subaction_other_start: i32,
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

#[derive(Debug)]
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
