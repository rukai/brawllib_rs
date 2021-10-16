pub mod misc_section;

use crate::script;
use crate::script::Script;
use crate::util;
use crate::wii_memory::WiiMemory;
use misc_section::MiscSection;

use fancy_slice::FancySlice;

#[rustfmt::skip]
pub(crate) fn arc_fighter_data(parent_data: FancySlice, data: FancySlice, wii_memory: &WiiMemory) -> ArcFighterData {
    let subaction_flags_start        = data.i32_be(0);
    let model_visibility_start       = data.i32_be(4);
    let attribute_start              = data.i32_be(8);
    let sse_attribute_start          = data.i32_be(12);
    let misc_section_offset          = data.i32_be(16);
    let common_action_flags_start    = data.i32_be(20);
    let action_flags_start           = data.i32_be(24);
    let _unknown0                    = data.i32_be(28);
    let action_interrupts            = data.i32_be(32);
    let entry_actions_start          = data.i32_be(36);
    let exit_actions_start           = data.i32_be(40);
    let action_pre_start             = data.i32_be(44);
    let subaction_main_start         = data.i32_be(48);
    let subaction_gfx_start          = data.i32_be(52);
    let subaction_sfx_start          = data.i32_be(56);
    let subaction_other_start        = data.i32_be(60);
    let anchored_item_positions      = data.i32_be(64);
    let gooey_bomb_positions         = data.i32_be(68);
    let bone_ref1                    = data.i32_be(72);
    let bone_ref2                    = data.i32_be(76);
    let entry_action_overrides_start = data.i32_be(80);
    let exit_action_overrides_start  = data.i32_be(84);
    let _unknown1                    = data.i32_be(88);
    let samus_arm_cannon_positions   = data.i32_be(92);
    let _unknown2                    = data.i32_be(96);
    let static_articles_start        = data.i32_be(100);
    let entry_articles_start         = data.i32_be(104);
    let flags1                       = data.u32_be(116);
    let flags2                       = data.i32_be(120);

    let sizes = get_sizes(data);

    let subaction_flags_num = sizes.iter().find(|x| x.offset == subaction_flags_start as usize).unwrap().size / SUB_ACTION_FLAGS_SIZE;
    let subaction_flags = subaction_flags(parent_data, parent_data.relative_fancy_slice(subaction_flags_start as usize ..), subaction_flags_num);

    let model_visibility = model_visibility(parent_data, model_visibility_start);

    let action_flags_num = sizes.iter().find(|x| x.offset == action_flags_start as usize).unwrap().size / ACTION_FLAGS_SIZE;
    let action_flags = action_flags(parent_data.relative_fancy_slice(action_flags_start as usize ..), action_flags_num);

    let entry_actions_num = sizes.iter().find(|x| x.offset == entry_actions_start as usize).unwrap().size / 4; // divide by integer size
    let entry_actions = script::scripts(parent_data.relative_fancy_slice(..), parent_data.relative_fancy_slice(entry_actions_start as usize ..), entry_actions_num, wii_memory);
    let exit_actions = script::scripts(parent_data.relative_fancy_slice(..), parent_data.relative_fancy_slice(exit_actions_start as usize ..), entry_actions_num, wii_memory);

    let subaction_main_num = sizes.iter().find(|x| x.offset == subaction_main_start as usize).unwrap().size / 4; // divide by integer size
    let subaction_main = script::scripts(parent_data.relative_fancy_slice(..), parent_data.relative_fancy_slice(subaction_main_start as usize ..), subaction_main_num, wii_memory);
    let subaction_gfx = script::scripts(parent_data.relative_fancy_slice(..), parent_data.relative_fancy_slice(subaction_gfx_start as usize ..), subaction_main_num, wii_memory);
    let subaction_sfx = script::scripts(parent_data.relative_fancy_slice(..), parent_data.relative_fancy_slice(subaction_sfx_start as usize ..), subaction_main_num, wii_memory);
    let subaction_other = script::scripts(parent_data.relative_fancy_slice(..), parent_data.relative_fancy_slice(subaction_other_start as usize ..), subaction_main_num, wii_memory);

    let attributes = fighter_attributes(parent_data.relative_fancy_slice(attribute_start as usize ..));
    let misc = misc_section::misc_section(parent_data.relative_fancy_slice(misc_section_offset as usize ..), parent_data);

    let entry_action_overrides = if entry_action_overrides_start != 0 {
        action_overrides(parent_data.relative_fancy_slice(..), parent_data.relative_fancy_slice(entry_action_overrides_start as usize ..), wii_memory)
    } else {
        vec!()
    };

    let exit_action_overrides = if exit_action_overrides_start != 0 {
        action_overrides(parent_data.relative_fancy_slice(..), parent_data.relative_fancy_slice(exit_action_overrides_start as usize ..), wii_memory)
    } else {
        vec!()
    };

    ArcFighterData {
        subaction_flags,
        attributes,
        misc,
        action_flags,
        entry_actions,
        exit_actions,
        subaction_main,
        subaction_gfx,
        subaction_sfx,
        subaction_other,
        model_visibility,
        sse_attribute_start,
        common_action_flags_start,
        action_interrupts,
        action_pre_start,
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

#[rustfmt::skip]
fn fighter_attributes(data: FancySlice) -> FighterAttributes {
    FighterAttributes {
        walk_init_vel:                     data.f32_be(0x00),
        walk_acc:                          data.f32_be(0x04),
        walk_max_vel:                      data.f32_be(0x08),
        ground_friction:                   data.f32_be(0x0c),
        dash_init_vel:                     data.f32_be(0x10),
        dash_run_acc_a:                    data.f32_be(0x14),
        dash_run_acc_b:                    data.f32_be(0x18),
        dash_run_term_vel:                 data.f32_be(0x1c),
        grounded_max_x_vel:                data.f32_be(0x24),
        dash_cancel_frame_window:          data.i32_be(0x28),
        guard_on_max_momentum:             data.f32_be(0x2c),
        jump_squat_frames:                 data.i32_be(0x30),
        jump_x_init_vel:                   data.f32_be(0x34),
        jump_y_init_vel:                   data.f32_be(0x38),
        jump_x_vel_ground_mult:            data.f32_be(0x3c),
        jump_x_init_term_vel:              data.f32_be(0x40),
        jump_y_init_vel_short:             data.f32_be(0x44),
        air_jump_x_mult:                   data.f32_be(0x48),
        air_jump_y_mult:                   data.f32_be(0x4c),
        footstool_init_vel:                data.f32_be(0x50),
        footstool_init_vel_short:          data.f32_be(0x54),
        meteor_cancel_delay:               data.f32_be(0x5c),
        num_jumps:                         data.u32_be(0x60),
        gravity:                           data.f32_be(0x64),
        term_vel:                          data.f32_be(0x68),
        air_friction_y:                    data.f32_be(0x6c),
        air_y_term_vel:                    data.f32_be(0x70),
        air_mobility_a:                    data.f32_be(0x74),
        air_mobility_b:                    data.f32_be(0x78),
        air_x_term_vel:                    data.f32_be(0x7c),
        air_friction_x:                    data.f32_be(0x80),
        fastfall_velocity:                 data.f32_be(0x84),
        air_x_term_vel_hard:               data.f32_be(0x88),
        glide_frame_window:                data.u32_be(0x8c),
        jab2_window:                       data.f32_be(0x94),
        jab3_window:                       data.f32_be(0x98),
        ftilt2_window:                     data.f32_be(0x9c),
        ftilt3_window:                     data.f32_be(0xa0),
        fsmash2_window:                    data.f32_be(0xa4),
        flip_dir_frame:                    data.f32_be(0xa8),
        weight:                            data.f32_be(0xb0),
        size:                              data.f32_be(0xb4),
        results_screen_size:               data.f32_be(0xb8),
        shield_size:                       data.f32_be(0xc4),
        shield_break_vel:                  data.f32_be(0xc8),
        shield_strength:                   data.f32_be(0xcc),
        respawn_platform_size:             data.f32_be(0xd4),
        edge_jump_x_vel:                   data.f32_be(0xf4),
        edge_jump_y_vel:                   data.f32_be(0xfc),
        item_throw_strength:               data.f32_be(0x118),
        projectile_item_move_speed:        data.f32_be(0x128),
        projectile_item_move_speed_dash_f: data.f32_be(0x12c),
        projectile_item_move_speed_dash_b: data.f32_be(0x130),
        light_landing_lag:                 data.f32_be(0x138),
        normal_landing_lag:                data.f32_be(0x13c),
        nair_landing_lag:                  data.f32_be(0x140),
        fair_landing_lag:                  data.f32_be(0x144),
        bair_landing_lag:                  data.f32_be(0x148),
        uair_landing_lag:                  data.f32_be(0x14c),
        dair_landing_lag:                  data.f32_be(0x150),
        term_vel_hard_frames:              data.u32_be(0x154),
        hip_n_bone:                        data.u32_be(0x158),
        tag_height_value:                  data.f32_be(0x15c),
        walljump_x_vel:                    data.f32_be(0x164),
        walljump_y_vel:                    data.f32_be(0x168),
        lhand_n_bone:                      data.u32_be(0x180),
        rhand_n_bone:                      data.u32_be(0x184),
        water_y_acc:                       data.f32_be(0x18c),
        spit_star_size:                    data.f32_be(0x1a4),
        spit_star_damage:                  data.u32_be(0x1a8),
        egg_size:                          data.f32_be(0x1ac),
        hip_n_bone2:                       data.u32_be(0x1cc),
        x_rot_n_bone:                      data.u32_be(0x1e0),
        camera_initial_y_offset:           data.f32_be(0x1f8),
        camera_size_front:                 data.f32_be(0x1fc),
        camera_size_back:                  data.f32_be(0x200),
        camera_size_top:                   data.f32_be(0x204),
        camera_size_bottom:                data.f32_be(0x208),
        zoom_camera_size_front:            data.f32_be(0x210),
        zoom_camera_size_back:             data.f32_be(0x214),
        zoom_camera_size_top:              data.f32_be(0x218),
        zoom_camera_size_bottom:           data.f32_be(0x21c),
        head_n_bone:                       data.u32_be(0x220),
        pause_camera_zoom_distance:        data.f32_be(0x244),
        magnifying_glass_size:             data.f32_be(0x244),
        weight_dependent_throw_backward:   data.u32_be(0x2dc) & 0b0001 == 0,
        weight_dependent_throw_forward:    data.u32_be(0x2dc) & 0b0010 == 0,
        weight_dependent_throw_up:         data.u32_be(0x2dc) & 0b0100 == 0,
        weight_dependent_throw_down:       data.u32_be(0x2dc) & 0b1000 == 0,
    }
}

const _ARC_FIGHTER_DATA_HEADER_SIZE: usize = 0x7c;
#[derive(Clone, Debug)]
pub struct ArcFighterData {
    pub subaction_flags: Vec<SubactionFlags>,
    pub attributes: FighterAttributes,
    pub misc: MiscSection,
    pub action_flags: Vec<ActionFlags>,
    pub entry_actions: Vec<Script>,
    pub exit_actions: Vec<Script>,
    pub subaction_main: Vec<Script>,
    pub subaction_gfx: Vec<Script>,
    pub subaction_sfx: Vec<Script>,
    pub subaction_other: Vec<Script>,
    pub model_visibility: ModelVisibility,
    pub entry_action_overrides: Vec<ActionOverride>,
    pub exit_action_overrides: Vec<ActionOverride>,
    sse_attribute_start: i32,
    common_action_flags_start: i32,
    action_interrupts: i32,
    action_pre_start: i32,
    anchored_item_positions: i32,
    gooey_bomb_positions: i32,
    bone_ref1: i32,
    bone_ref2: i32,
    samus_arm_cannon_positions: i32,
    static_articles_start: i32,
    entry_articles_start: i32,
    flags1: u32,
    flags2: i32,
}

#[derive(Serialize, Clone, Debug)]
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
    pub air_x_term_vel: f32,
    pub air_friction_x: f32,
    pub fastfall_velocity: f32,
    pub air_x_term_vel_hard: f32,
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
    pub camera_initial_y_offset: f32,
    pub camera_size_front: f32,
    pub camera_size_back: f32,
    pub camera_size_top: f32,
    pub camera_size_bottom: f32,
    pub zoom_camera_size_front: f32,
    pub zoom_camera_size_back: f32,
    pub zoom_camera_size_top: f32,
    pub zoom_camera_size_bottom: f32,
    pub head_n_bone: u32,
    pub pause_camera_zoom_distance: f32,
    pub magnifying_glass_size: f32,
    pub weight_dependent_throw_down: bool,
    pub weight_dependent_throw_up: bool,
    pub weight_dependent_throw_forward: bool,
    pub weight_dependent_throw_backward: bool,
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    #[rustfmt::skip]
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

fn subaction_flags(parent_data: FancySlice, data: FancySlice, num: usize) -> Vec<SubactionFlags> {
    let mut result = vec![];
    let num = num + 1;
    for i in 0..num {
        let in_translation_time = data.u8(i * SUB_ACTION_FLAGS_SIZE + 0);
        let animation_flags_int = data.u8(i * SUB_ACTION_FLAGS_SIZE + 1);
        //  padding               data.u16_be(i * SUB_ACTION_FLAGS_SIZE + 2..);
        let string_offset = data.i32_be(i * SUB_ACTION_FLAGS_SIZE + 4);

        let animation_flags = AnimationFlags::from_bits(animation_flags_int).unwrap();
        let name = if string_offset == 0 {
            String::new()
        } else {
            parent_data.str(string_offset as usize).unwrap().to_string()
        };

        result.push(SubactionFlags {
            in_translation_time,
            animation_flags,
            name,
        });
    }
    result
}

const SUB_ACTION_FLAGS_SIZE: usize = 0x8;
#[derive(Clone, Debug)]
pub struct SubactionFlags {
    pub in_translation_time: u8,
    pub animation_flags: AnimationFlags,
    pub name: String,
}

fn model_visibility(parent_data: FancySlice, model_visibility_start: i32) -> ModelVisibility {
    let reference_offset = parent_data.i32_be(model_visibility_start as usize + 0x00);
    let bone_switch_count = parent_data.i32_be(model_visibility_start as usize + 0x04);
    let defaults_offset = parent_data.i32_be(model_visibility_start as usize + 0x08);
    let defaults_count = parent_data.i32_be(model_visibility_start as usize + 0x0c);

    let mut references = vec![];
    if reference_offset != 0 {
        // this works because the data at reference_offset, defaults_offset and model_visibility_start are stored sequentially
        let reference_count = if defaults_offset == 0 {
            (model_visibility_start - reference_offset) / VISIBILITY_REFERENCE_SIZE as i32
        } else {
            (defaults_offset - reference_offset) / VISIBILITY_REFERENCE_SIZE as i32
        };
        if reference_count < 0 {
            error!("Oh no the reference_count calculation is messed up, please handle this case properly");
            return ModelVisibility {
                references: vec![],
                defaults: vec![],
            };
        }

        for reference_i in 0..reference_count as usize {
            let bone_switch_offset = parent_data
                .i32_be(reference_offset as usize + VISIBILITY_REFERENCE_SIZE * reference_i)
                as usize;
            let mut bone_switches = vec![];
            if bone_switch_offset != 0 {
                for bone_switch_i in 0..bone_switch_count as usize {
                    let visibility_group_list =
                        util::list_offset(parent_data.relative_fancy_slice(
                            bone_switch_offset + util::LIST_OFFSET_SIZE * bone_switch_i..,
                        ));
                    let mut groups = vec![];

                    for visibility_group_i in 0..visibility_group_list.count as usize {
                        let bone_list = util::list_offset(parent_data.relative_fancy_slice(
                            visibility_group_list.start_offset as usize
                                + util::LIST_OFFSET_SIZE * visibility_group_i..,
                        ));
                        let mut bones = vec![];

                        for bone_i in 0..bone_list.count as usize {
                            let bone =
                                parent_data.i32_be(bone_list.start_offset as usize + 4 * bone_i);
                            bones.push(bone);
                        }

                        groups.push(VisibilityGroup { bones });
                    }

                    bone_switches.push(VisibilityBoneSwitch { groups });
                }
            }
            references.push(VisibilityReference { bone_switches });
        }
    }

    let mut defaults = vec![];
    for i in 0..defaults_count as usize {
        let switch_index =
            parent_data.i32_be(defaults_offset as usize + VISIBILITY_DEFAULT_SIZE * i);
        let group_index =
            parent_data.i32_be(defaults_offset as usize + VISIBILITY_DEFAULT_SIZE * i + 4);

        defaults.push(VisibilityDefault {
            switch_index,
            group_index,
        });
    }

    ModelVisibility {
        references,
        defaults,
    }
}

#[derive(Clone, Debug)]
pub struct ModelVisibility {
    pub references: Vec<VisibilityReference>,
    pub defaults: Vec<VisibilityDefault>,
}

const VISIBILITY_REFERENCE_SIZE: usize = 0x4;
#[derive(Clone, Debug)]
pub struct VisibilityReference {
    pub bone_switches: Vec<VisibilityBoneSwitch>,
}

#[derive(Clone, Debug)]
pub struct VisibilityBoneSwitch {
    pub groups: Vec<VisibilityGroup>,
}

/// Enabling a `VisibilityGroup` will disable all other groups in the same `VisibilityBoneSwitch`
#[derive(Clone, Debug)]
pub struct VisibilityGroup {
    pub bones: Vec<i32>,
}

const VISIBILITY_DEFAULT_SIZE: usize = 0x8;
/// Enables the `VisibilityGroup` with the matching `switch_index` and `group_index` for all `VisibilityReferences`s.
/// When a new subaction is started, everything is set invisible and then all `VisibilityDefault`s are run.
#[derive(Clone, Debug)]
pub struct VisibilityDefault {
    pub switch_index: i32,
    pub group_index: i32,
}

fn action_flags(data: FancySlice, num: usize) -> Vec<ActionFlags> {
    let mut result = vec![];
    for i in 0..num {
        result.push(ActionFlags {
            flag1: data.u32_be(i * ACTION_FLAGS_SIZE + 0x0),
            flag2: data.u32_be(i * ACTION_FLAGS_SIZE + 0x4),
            flag3: data.u32_be(i * ACTION_FLAGS_SIZE + 0x8),
            flag4: data.u32_be(i * ACTION_FLAGS_SIZE + 0xc),
        });
    }
    result
}

const ACTION_FLAGS_SIZE: usize = 0x10;
#[derive(Clone, Debug)]
pub struct ActionFlags {
    pub flag1: u32,
    pub flag2: u32,
    pub flag3: u32,
    pub flag4: u32,
}

struct OffsetSizePair {
    offset: usize,
    size: usize,
}

fn get_sizes(data: FancySlice) -> Vec<OffsetSizePair> {
    let mut pairs = vec![];
    for i in 0..27 {
        let offset = data.i32_be(i * 4) as usize;
        if offset != 0 {
            pairs.push(OffsetSizePair { offset, size: 0 });
        }
    }

    pairs.sort_by_key(|x| x.offset);

    // fill in size for most elements
    for i in 0..pairs.len() - 1 {
        pairs[i].size = pairs[i + 1].offset - pairs[i].offset
    }

    // Just pop the last element, so if we try to access it we get a panic
    pairs.pop();

    pairs
}

fn action_overrides(
    parent_data: FancySlice,
    data: FancySlice,
    wii_memory: &WiiMemory,
) -> Vec<ActionOverride> {
    let mut overrides = vec![];
    for i in 0..10 {
        let action_id = data.u32_be(i * OVERRIDE_SIZE);
        let offset = data.u32_be(i * OVERRIDE_SIZE + 4);
        let script = script::new_script(parent_data.relative_fancy_slice(..), offset, wii_memory);

        if action_id == !0u32 {
            break;
        }
        overrides.push(ActionOverride { action_id, script });
    }
    overrides
}

const OVERRIDE_SIZE: usize = 0x8;
#[derive(Clone, Debug)]
pub struct ActionOverride {
    pub action_id: u32,
    pub script: Script,
}
