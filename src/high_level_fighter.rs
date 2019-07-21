use cgmath::{Point3, Vector3, Matrix4, SquareMatrix, InnerSpace, Transform};
use rayon::prelude::*;

use crate::chr0::Chr0;
use crate::fighter::Fighter;
use crate::mdl0::bones::Bone;
use crate::sakurai::{SectionScript, ExternalSubroutine};
use crate::sakurai::fighter_data::misc_section::{HurtBox, BoneRefs};
use crate::sakurai::fighter_data::{FighterAttributes, AnimationFlags};
use crate::script_ast::{
    ScriptAst,
    HitBoxArguments,
    SpecialHitBoxArguments,
    GrabBoxArguments,
    HurtBoxState,
    EdgeSlide,
    AngleFlip,
    HitBoxEffect,
    HitBoxSound,
    HitBoxSseType,
    GrabTarget,
    LedgeGrabEnable,
};
use crate::script_runner::{ScriptRunner, ChangeSubaction, ScriptCollisionBox, VelModify};

/// The HighLevelFighter stores processed Fighter data in a format that is easy to read from.
/// If brawllib_rs eventually implements the ability to modify character files via modifying Fighter and its children, then HighLevelFighter WILL NOT support that.
#[derive(Serialize, Clone, Debug)]
pub struct HighLevelFighter {
    pub name:                     String,
    pub internal_name:            String,
    pub attributes:               FighterAttributes,
    pub actions:                  Vec<HighLevelAction>,
    pub subactions:               Vec<HighLevelSubaction>,
    pub scripts_fragment_fighter: Vec<ScriptAst>,
    pub scripts_fragment_common:  Vec<ScriptAst>,
    pub scripts_section:          Vec<SectionScriptAst>,
}

impl HighLevelFighter {
    /// Processes data from an &Fighter and stores it in a HighLevelFighter
    // TODO: Maybe expose a `multithreaded` argument so caller can disable multithread and run its own multithreading on the entire `HighLevelFighter::new`.
    // Because rayon uses a threadpool we arent at risk of it hammering the system by spawning too many threads.
    // However it may be ineffecient due to overhead of spawning threads for every action.
    // Will need to benchmark any such changes.
    pub fn new(fighter: &Fighter) -> HighLevelFighter {
        info!("Generating HighLevelFighter for {}", fighter.cased_name);
        let fighter_sakurai = fighter.get_fighter_sakurai().unwrap();
        let fighter_sakurai_common = fighter.get_fighter_sakurai_common().unwrap();
        let fighter_data = fighter.get_fighter_data().unwrap();
        let fighter_data_common = fighter.get_fighter_data_common().unwrap();
        let fighter_data_common_scripts = fighter.get_fighter_data_common_scripts();
        let attributes = fighter_data.attributes.clone();
        let fighter_animations = fighter.get_animations();

        let fragment_scripts_fighter: Vec<_> = fighter_sakurai.fragment_scripts.iter().map(|x| ScriptAst::new(x)).collect();
        let subaction_main:           Vec<_> = fighter_data.subaction_main  .iter().map(|x| ScriptAst::new(x)).collect();
        let subaction_gfx:            Vec<_> = fighter_data.subaction_gfx   .iter().map(|x| ScriptAst::new(x)).collect();
        let subaction_sfx:            Vec<_> = fighter_data.subaction_sfx   .iter().map(|x| ScriptAst::new(x)).collect();
        let subaction_other:          Vec<_> = fighter_data.subaction_other .iter().map(|x| ScriptAst::new(x)).collect();

        let fragment_scripts_common: Vec<_> = fighter_sakurai_common.fragment_scripts.iter().map(|x| ScriptAst::new(x)).collect();

        let scripts_section: Vec<_> = fighter_data_common_scripts.iter().map(|x| SectionScriptAst::new(x, &fighter_sakurai.external_subroutines)).collect();

        let entry_actions_common: Vec<_> = fighter_data_common.entry_actions.iter().map(|x| ScriptAst::new(x)).collect();
        let entry_actions:        Vec<_> = fighter_data       .entry_actions.iter().map(|x| ScriptAst::new(x)).collect();
        let exit_actions_common:  Vec<_> = fighter_data_common.exit_actions .iter().map(|x| ScriptAst::new(x)).collect();
        let exit_actions:         Vec<_> = fighter_data       .exit_actions .iter().map(|x| ScriptAst::new(x)).collect();

        let mut fighter_scripts = vec!();
        for script in fragment_scripts_fighter.iter()
            .chain(subaction_main.iter())
            .chain(subaction_gfx.iter())
            .chain(subaction_sfx.iter())
            .chain(subaction_other.iter())
            .chain(entry_actions.iter())
            .chain(exit_actions.iter())
        {
            fighter_scripts.push(script);
        }

        let mut common_scripts = vec!();
        for script in fragment_scripts_common.iter()
            .chain(scripts_section.iter().map(|x| &x.script))
            .chain(entry_actions_common.iter())
            .chain(exit_actions_common.iter())
        {
            common_scripts.push(script);
        }

        let mut subaction_scripts = vec!();
        for i in 0..subaction_main.len() {
            subaction_scripts.push(HighLevelScripts {
                script_main:  subaction_main[i].clone(),
                script_gfx:   subaction_gfx[i].clone(),
                script_sfx:   subaction_sfx[i].clone(),
                script_other: subaction_other[i].clone(),
            });
        }

        let mut actions = vec!();
        for i in 0..entry_actions_common.len() {
            let name = crate::action_names::action_name(i);

            let entry_action_override = fighter_data.entry_action_overrides.iter().find(|x| x.action_id == i as u32);
            let (script_entry, script_entry_common) = if let Some(action_override) = entry_action_override {
                (ScriptAst::new(&action_override.script), false)
            } else {
                (entry_actions_common[i].clone(), true)
            };
            let exit_action_override = fighter_data.exit_action_overrides.iter().find(|x| x.action_id == i as u32);
            let (script_exit, script_exit_common) = if let Some(action_override) = exit_action_override {
                (ScriptAst::new(&action_override.script), false)
            } else {
                (exit_actions_common[i].clone(), true)
            };
            actions.push(HighLevelAction { name, script_entry, script_exit, script_entry_common, script_exit_common });
        }

        for i in 0..entry_actions.len() {
            actions.push(HighLevelAction {
                name:                crate::action_names::action_name(0x112 + i),
                script_entry:        entry_actions[i].clone(),
                script_entry_common: false,
                script_exit:         exit_actions[i].clone(),
                script_exit_common:  false,
            });
        }

        let subactions = if let Some(first_bone) = fighter.get_bones() {
            // TODO: After fixing a bug, where a huge amount of needless work was being done, parallelizing this doesnt get us as much.
            // It might be better for the caller of HighLevelFighter::new() to do the parallelization.
            subaction_scripts.into_par_iter().enumerate().map(|(i, scripts)| {
                let subaction_flags = &fighter_data.subaction_flags[i];
                let actual_name = subaction_flags.name.clone();

                // create a unique name for this subaction
                let mut count = 0;
                for j in 0..i {
                    if fighter_data.subaction_flags[j].name == actual_name {
                        count += 1;
                    }
                }
                let name = if count == 0 {
                    actual_name.clone()
                } else {
                    format!("{}_{}", actual_name, count)
                };

                let animation_flags = subaction_flags.animation_flags.clone();

                let chr0 = fighter_animations.iter().find(|x| x.name == actual_name);
                let action_scripts = vec!(&scripts.script_main, &scripts.script_gfx, &scripts.script_sfx, &scripts.script_other);

                let mut frames: Vec<HighLevelFrame> = vec!();
                let mut prev_animation_xyz_offset = Vector3::new(0.0, 0.0, 0.0);
                let mut script_runner = ScriptRunner::new(i, &fighter.wiird_frame_speed_modifiers, &action_scripts, &fighter_scripts, &common_scripts, &scripts_section, &fighter_data, actual_name.clone());
                let mut iasa = None;
                let mut prev_hit_boxes: Option<Vec<PositionHitBox>> = None;

                if let Some(chr0) = chr0 {
                    let num_frames = match actual_name.as_ref() {
                        "JumpSquat"    => attributes.jump_squat_frames as f32,
                        "LandingAirN"  => attributes.nair_landing_lag,
                        "LandingAirF"  => attributes.fair_landing_lag,
                        "LandingAirB"  => attributes.bair_landing_lag,
                        "LandingAirHi" => attributes.uair_landing_lag,
                        "LandingAirLw" => attributes.dair_landing_lag,
                        "LandingLight" => attributes.light_landing_lag, // TODO: This needs +1 do the others?!?!?
                        "LandingHeavy" => attributes.normal_landing_lag,
                        _              => chr0.num_frames as f32
                    };

                    let mut x_vel = 0.0;
                    let mut y_vel = 0.0;

                    let mut x_pos = 0.0;
                    let mut y_pos = 0.0;

                    while script_runner.animation_index < num_frames {
                        let chr0_frame_index = script_runner.animation_index * chr0.num_frames as f32 / num_frames; // map frame count between [0, chr0.num_frames]
                        let (animation_xyz_offset, frame_bones) = HighLevelFighter::transform_bones(
                            &first_bone,
                            &fighter_data.misc.bone_refs,
                            Matrix4::<f32>::identity(),
                            Matrix4::<f32>::identity(),
                            chr0,
                            chr0_frame_index as i32,
                            animation_flags,
                            fighter_data.attributes.size
                        );
                        let animation_xyz_offset = animation_xyz_offset.unwrap_or(Vector3::new(0.0, 0.0, 0.0));
                        // TODO: should DisableMovement affect xyz_offset from transform_bones?????
                        // script runner x-axis is equivalent to model z-axis

                        let animation_xyz_velocity = animation_xyz_offset - prev_animation_xyz_offset;
                        prev_animation_xyz_offset = animation_xyz_offset;

                        let x_vel_modify = script_runner.x_vel_modify.clone();
                        let y_vel_modify = script_runner.y_vel_modify.clone();

                        let x_vel_temp = animation_xyz_velocity.z;
                        let y_vel_temp = animation_xyz_velocity.y;

                        match x_vel_modify {
                            VelModify::Set (vel) => x_vel = vel,
                            VelModify::Add (vel) => x_vel += vel,
                            VelModify::None      => { }
                        }

                        match y_vel_modify {
                            VelModify::Set (vel) => y_vel = vel,
                            VelModify::Add (vel) => y_vel += vel,
                            VelModify::None      => { }
                        }

                        x_pos += x_vel + x_vel_temp;
                        y_pos += y_vel + y_vel_temp;

                        let hurt_boxes = gen_hurt_boxes(&frame_bones, &fighter_data.misc.hurt_boxes, &script_runner, fighter_data.attributes.size);
                        let hit_boxes: Vec<_> = script_runner.hitboxes.iter().filter(|x| x.is_some()).map(|x| x.clone().unwrap()).collect();
                        let hit_boxes = gen_hit_boxes(&frame_bones, &hit_boxes);
                        let mut hl_hit_boxes = vec!();
                        for next in &hit_boxes {
                            let mut prev_pos = None;
                            let mut prev_size = None;
                            let mut prev_values = None;
                            if next.interpolate {
                                if let &Some(ref prev_hit_boxes) = &prev_hit_boxes {
                                    for prev_hit_box in prev_hit_boxes {
                                        if prev_hit_box.hitbox_id == next.hitbox_id {
                                            // A bit hacky, but we need to undo the movement that occured this frame to get the correct hitbox interpolation
                                            prev_pos = Some(prev_hit_box.position - Vector3::new(0.0, y_vel, x_vel));
                                            prev_size = Some(prev_hit_box.size);
                                            prev_values = Some(prev_hit_box.values.clone());
                                        }
                                    }
                                }
                            }
                            hl_hit_boxes.push(HighLevelHitBox {
                                hitbox_id: next.hitbox_id,

                                prev_pos,
                                prev_size,
                                prev_values,

                                next_pos:    next.position,
                                next_size:   next.size,
                                next_values: next.values.clone(),
                            });
                        }
                        hl_hit_boxes.sort_by_key(|x| x.hitbox_id);

                        let mut option_ecb = None;
                        for misc_ecb in &fighter_data.misc.ecbs {
                            let min_ecb = ECB {
                                // This implementation is just a guess from my observations that:
                                // *    The higher the min_width the higher the right ecb point.
                                // *    The higher the min_width the lower the left ecb point.
                                // *    When further than all bones, both points move equally far apart.
                                // *    When further than all bones, actions that affect the ecb horizontally no longer affect the ecb e.g. marth jab
                                left:     -misc_ecb.min_width / 2.0, // TODO: Should I divide by 2.0 here?
                                right:    misc_ecb.min_width / 2.0, // TODO: Should I divide by 2.0 here?
                                top:      -10000.0,
                                bottom:   10000.0,
                                transn_x: 0.0,
                                transn_y: 0.0,
                            };
                            let mut ecb = gen_ecb(&frame_bones, &misc_ecb.bones, &fighter_data.misc.bone_refs, min_ecb);

                            // This implementation is just a guess from my observations that:
                            // *    The higher the min_height the higher the top ecb point.
                            // *    The higher the min_height the lower the bottom ecb point, capping out at transN.
                            // *    Actions such as crouching, lower the height of the top ecb point.
                            let middle_y = (ecb.top + ecb.bottom) / 2.0;
                            let new_top    = middle_y + misc_ecb.min_height / 2.0;
                            let new_bottom = middle_y - misc_ecb.min_height / 2.0;
                            if new_top > ecb.top {
                                ecb.top = new_top;
                            }
                            if new_bottom < ecb.bottom {
                                ecb.bottom = new_bottom;
                            }
                            if ecb.bottom < ecb.transn_y {
                                ecb.bottom = ecb.transn_y
                            }

                            option_ecb = Some(ecb);
                        }
                        let ecb = option_ecb.unwrap();

                        let weight_dependent_speed = match actual_name.as_ref() {
                            "ThrowLw" => attributes.weight_dependent_throw_down,
                            "ThrowHi" => attributes.weight_dependent_throw_up,
                            "ThrowF" => attributes.weight_dependent_throw_forward,
                            "ThrowB" => attributes.weight_dependent_throw_backward,
                            _        => false,
                        };

                        let mut throw = None;
                        if let Some(ref specify_throw) = script_runner.throw {
                            if script_runner.throw_activate {
                                throw = Some(HighLevelThrow {
                                    damage:      specify_throw.damage,
                                    trajectory:  specify_throw.trajectory,
                                    kbg:         specify_throw.kbg,
                                    wdsk:        specify_throw.wdsk,
                                    bkb:         specify_throw.bkb,
                                    effect:      specify_throw.effect.clone(),
                                    sfx:         specify_throw.sfx.clone(),
                                    grab_target: specify_throw.grab_target.clone(),
                                    i_frames:    specify_throw.i_frames,
                                    weight_dependent_speed,
                                });
                            }
                        }

                        let ledge_grab_box = if script_runner.ledge_grab_enable.enabled() {
                            // The first misc.ledge_grabs entry seems to be used for everything, not sure what the other entries are for.
                            if let Some(ledge_grab_box) = fighter_data.misc.ledge_grab_boxes.get(0) {
                                let left = if let LedgeGrabEnable::EnableInFrontAndBehind = script_runner.ledge_grab_enable {
                                    ecb.left - ledge_grab_box.x_padding
                                } else {
                                    ledge_grab_box.x_left
                                };

                                Some(Extent {
                                    left,
                                    right:  ecb.right + ledge_grab_box.x_padding,
                                    up:     ledge_grab_box.y + ledge_grab_box.height,
                                    down:   ledge_grab_box.y,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        frames.push(HighLevelFrame {
                            throw,
                            ecb,
                            x_pos,
                            y_pos,
                            x_vel_modify,
                            y_vel_modify,
                            x_vel_temp,
                            y_vel_temp,
                            ledge_grab_box,
                            hurt_boxes,
                            hit_boxes:             hl_hit_boxes,
                            interruptible:         script_runner.interruptible,
                            landing_lag:           script_runner.landing_lag,
                            edge_slide:            script_runner.edge_slide.clone(),
                            reverse_direction:     script_runner.reverse_direction.clone(),
                            airbourne:             script_runner.airbourne,
                            hitbox_sets_rehit:     script_runner.hitbox_sets_rehit,
                            slope_contour_stand:   script_runner.slope_contour_stand,
                            slope_contour_full:    script_runner.slope_contour_full,
                            rumble:                script_runner.rumble,
                            rumble_loop:           script_runner.rumble_loop,
                            grab_interrupt_damage: script_runner.grab_interrupt_damage,
                        });

                        if iasa.is_none() && script_runner.interruptible {
                            iasa = Some(script_runner.frame_count)
                        }

                        script_runner.step();
                        prev_hit_boxes = Some(hit_boxes);

                        if let ChangeSubaction::Continue = script_runner.change_subaction { } else { break }
                    }
                }

                let iasa = if let Some(iasa) = iasa {
                    iasa
                } else {
                    match actual_name.as_ref() {
                        "LandingAirN"  | "LandingAirF" |
                        "LandingAirB"  | "LandingAirHi" |
                        "LandingAirLw" | "LandingLight" |
                        "LandingHeavy" | "LandingFallSpecial"
                          => script_runner.frame_count,
                        _ => 0
                    }
                };

                let landing_lag = match actual_name.as_ref() {
                    "AttackAirN"  => Some(attributes.nair_landing_lag),
                    "AttackAirF"  => Some(attributes.fair_landing_lag),
                    "AttackAirB"  => Some(attributes.bair_landing_lag),
                    "AttackAirHi" => Some(attributes.uair_landing_lag),
                    "AttackAirLw" => Some(attributes.dair_landing_lag),
                    _             => None,
                };

                let bad_interrupts = script_runner.bad_interrupts.len() > 0;

                HighLevelSubaction { name, iasa, landing_lag, frames, animation_flags, scripts, bad_interrupts }
            }).collect()
        } else {
            vec!()
        };

        HighLevelFighter {
            internal_name:            fighter.cased_name.clone(),
            name:                     crate::fighter_maps::fighter_name(&fighter.cased_name),
            scripts_fragment_fighter: fragment_scripts_fighter,
            scripts_fragment_common:  fragment_scripts_common,
            scripts_section,
            attributes,
            actions,
            subactions,
        }
    }

    /// Generates a tree of BoneTransforms from the specified animation frame applied on the passed tree of bones
    /// The resulting matrices are independent of its parent bones matrix.
    /// Returns a tuple containing:
    ///     0.  The MOVES_CHARACTER offset if enabled. this is used by e.g. Ness's double jump
    ///     1.  The BoneTransforms tree.
    fn transform_bones(bone: &Bone, bone_refs: &BoneRefs, parent_transform: Matrix4<f32>, parent_transform_hitbox: Matrix4<f32>, chr0: &Chr0, frame: i32, animation_flags: AnimationFlags, size: f32) -> (Option<Vector3<f32>>, BoneTransforms) {
        let moves_character = animation_flags.contains(AnimationFlags::MOVES_CHARACTER);

        // by default the bones tpose transformation is used.
        let mut transform_normal = parent_transform * bone.gen_transform();
        let mut transform_hitbox = parent_transform_hitbox * bone.gen_transform_rot_only();

        // if the animation specifies a transform for the bone, override the models default tpose transform.
        let mut offset = None;
        for chr0_child in &chr0.children {
            if chr0_child.name == bone.name {
                let transform = parent_transform * chr0_child.get_transform(chr0.loop_value, frame);
                if moves_character && bone.index == bone_refs.trans_n {
                    // in this case TransN is not part of the animation but instead used to move the character in game.
                    assert!(offset.is_none());
                    offset = Some(Vector3::new(transform.w.x, transform.w.y, transform.w.z));
                    // TODO: Should this case modify transform_normal rot and scale?
                }
                else {
                    // The animation specifies a transform for this bone, and its not used for character movement. USE IT!
                    transform_normal = transform;
                    transform_hitbox = parent_transform_hitbox * chr0_child.get_transform_rot_only(chr0.loop_value, frame);
                }
            }
        }

        // Ignore any transformations from the models tpose TopN bone or the animations TopN bone.
        // Furthermore we make use of this bone to apply a scale to the entire model.
        if bone.name == "TopN" {
            transform_normal = Matrix4::from_scale(size);
            transform_hitbox = Matrix4::identity();
        }

        // do the same for all children bones
        let mut children = vec!();
        for child in bone.children.iter() {
            let (moves, processed_child) = HighLevelFighter::transform_bones(child, bone_refs, transform_normal, transform_hitbox, chr0, frame, animation_flags, size);
            children.push(processed_child);
            if let Some(moves) = moves {
                assert!(offset.is_none());
                offset = Some(moves);
            }
        }
        let bone = BoneTransforms {
            index: bone.index,
            transform_normal,
            transform_hitbox,
            children,
        };
        (offset, bone)
    }
}

pub struct BoneTransforms {
    pub index:            i32,
    pub transform_normal: Matrix4<f32>,
    pub transform_hitbox: Matrix4<f32>,
    pub children:         Vec<BoneTransforms>,
}

#[derive(Serialize, Clone, Debug)]
pub struct HighLevelAction {
    pub name:         String,
    pub script_entry: ScriptAst,
    pub script_exit:  ScriptAst,
    /// This is needed to determine where Goto/Subroutine events are pointing
    pub script_entry_common: bool,
    /// This is needed to determine where Goto/Subroutine events are pointing
    pub script_exit_common: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct HighLevelSubaction {
    pub name:            String,
    pub iasa:            usize,
    pub frames:          Vec<HighLevelFrame>,
    pub landing_lag:     Option<f32>,
    pub animation_flags: AnimationFlags,
    pub scripts:         HighLevelScripts,
    /// A hack where bad interrupts are ignored was used to process this subaction
    pub bad_interrupts:  bool,
}

impl HighLevelSubaction {
    /// Furthest point of a hitbox, starting from the bps
    /// Furthest values across all frames
    pub fn hit_box_extent(&self) -> Extent {
        let mut extent = Extent::new();
        for frame in &self.frames {
            let mut new_extent = frame.hit_box_extent();
            new_extent.up    += frame.y_pos;
            new_extent.down  += frame.y_pos;
            new_extent.left  += frame.x_pos;
            new_extent.right += frame.x_pos;
            extent.extend(&new_extent);
        }
        extent
    }

    /// Furthest point of a hurtbox, starting from the bps
    /// Furthest values across all frames
    pub fn hurt_box_extent(&self) -> Extent {
        let mut extent = Extent::new();
        for frame in &self.frames {
            let mut new_extent = frame.hurt_box_extent();
            new_extent.up    += frame.y_pos;
            new_extent.down  += frame.y_pos;
            new_extent.left  += frame.x_pos;
            new_extent.right += frame.x_pos;
            extent.extend(&new_extent);
        }
        extent
    }

    /// Furthest point of a ledge grab box, starting from the bps
    /// Furthest values across all frames
    pub fn ledge_grab_box_extent(&self) -> Extent {
        let mut extent = Extent::new();
        for frame in &self.frames {
            if let Some(ref ledge_grab_box) = frame.ledge_grab_box {
                extent.extend(ledge_grab_box);
            }
        }
        extent
    }

    /// Furthest point of a hurtbox, starting from the bps
    /// Furthest values across all frames
    pub fn hurt_box_vulnerable_extent(&self) -> Option<Extent> {
        let mut extent: Option<Extent> = None;
        for frame in &self.frames {
            if let Some(mut new_extent) = frame.hurt_box_vulnerable_extent() {
                new_extent.up    += frame.y_pos;
                new_extent.down  += frame.y_pos;
                new_extent.left  += frame.x_pos;
                new_extent.right += frame.x_pos;

                if let Some(ref mut extent) = extent {
                    extent.extend(&new_extent);
                } else {
                    extent = Some(new_extent);
                }
            }
        }
        extent
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct HighLevelScripts {
    pub script_main:  ScriptAst,
    pub script_gfx:   ScriptAst,
    pub script_sfx:   ScriptAst,
    pub script_other: ScriptAst,
}

#[derive(Serialize, Clone, Debug)]
pub struct HighLevelThrow {
    // TODO: I imagine the bone is used to determine the location the character is thrown from.
    // Transform the bone into an xy offset.
    pub damage:                 i32,
    pub trajectory:             i32,
    pub kbg:                    i32,
    pub wdsk:                   i32,
    pub bkb:                    i32,
    pub effect:                 HitBoxEffect,
    pub sfx:                    HitBoxSound,
    pub grab_target:            GrabTarget,
    pub i_frames:               i32,
    pub weight_dependent_speed: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct HighLevelFrame {
    pub hurt_boxes:            Vec<HighLevelHurtBox>,
    pub hit_boxes:             Vec<HighLevelHitBox>,
    pub ledge_grab_box:        Option<Extent>,
    pub x_pos:                 f32,
    pub y_pos:                 f32,
    pub interruptible:         bool,
    pub edge_slide:            EdgeSlide,
    pub reverse_direction:     bool,
    pub airbourne:             bool,
    pub landing_lag:           bool,
    pub ecb:                   ECB,
    pub hitbox_sets_rehit:     [bool; 10],
    pub slope_contour_stand:   Option<i32>,
    pub slope_contour_full:    Option<(i32, i32)>,
    pub rumble:                Option<(i32, i32)>,
    pub rumble_loop:           Option<(i32, i32)>,
    pub grab_interrupt_damage: Option<i32>,
    pub throw:                 Option<HighLevelThrow>,
    /// Affects the next frames velocity
    pub x_vel_modify: VelModify,
    /// Affects the next frames velocity
    pub y_vel_modify: VelModify,
    /// Does not affect the next frames velocity
    pub x_vel_temp: f32,
    /// Does not affect the next frames velocity
    pub y_vel_temp: f32,
}

impl HighLevelFrame {
    /// Furthest point of a hitbox, starting from the bps
    pub fn hit_box_extent(&self) -> Extent {
        let mut extent = Extent::new();
        for hit_box in &self.hit_boxes {
            if let (Some(pos), Some(size)) = (hit_box.prev_pos, hit_box.prev_size) {
                let new_extent = Extent {
                    up:    pos.y + size,
                    down:  pos.y - size,
                    left:  pos.z - size,
                    right: pos.z + size,
                };
                extent.extend(&new_extent);
            }

            let pos = hit_box.next_pos;
            let size = hit_box.next_size;
            let new_extent = Extent {
                up:    pos.y + size,
                down:  pos.y - size,
                left:  pos.z - size,
                right: pos.z + size,
            };
            extent.extend(&new_extent);
        }
        extent
    }

    /// Furthest point of a hurtbox, starting from the bps
    pub fn hurt_box_extent(&self) -> Extent {
        let mut extent = Extent::new();
        for hurt_box in &self.hurt_boxes {
            let bone_matrix = hurt_box.bone_matrix.clone();
            let bone_scale = Vector3::new(bone_matrix.x.magnitude(), bone_matrix.y.magnitude(), bone_matrix.z.magnitude());
          //let bone_trousle = https://www.youtube.com/watch?v=64cvrwzrmhU
            let radius = hurt_box.hurt_box.radius;
            let stretch = hurt_box.hurt_box.stretch;

            let stretch_face_temp = stretch / radius;
            let stretch_face = Vector3::new(
                stretch_face_temp.x / bone_scale.x,
                stretch_face_temp.y / bone_scale.y,
                stretch_face_temp.z / bone_scale.z
            );

            let transform_scale = Matrix4::from_scale(radius);
            let transform_translation = Matrix4::from_translation(Vector3::new(
                hurt_box.hurt_box.offset.x / (bone_scale.x * radius),
                hurt_box.hurt_box.offset.y / (bone_scale.y * radius),
                hurt_box.hurt_box.offset.z / (bone_scale.z * radius),
            ));
            let transform = bone_matrix * transform_scale * transform_translation;

            let sphere_8th_centers = [
                Point3::new(0.0,            0.0,            0.0),
                Point3::new(stretch_face.x, 0.0,            0.0),
                Point3::new(0.0,            stretch_face.y, 0.0),
                Point3::new(0.0,            0.0,            stretch_face.z),
                Point3::new(stretch_face.x, stretch_face.y, 0.0),
                Point3::new(0.0,            stretch_face.y, stretch_face.z),
                Point3::new(stretch_face.x, 0.0,            stretch_face.z),
            ];

            for center in &sphere_8th_centers {
                let transformed_center = transform.transform_point(*center);

                // from the center of each sphere 8th we can apply the radius to get the maximum extent in all dimensions
                let new_extent = Extent {
                    up:    transformed_center.y + radius,
                    down:  transformed_center.y - radius,
                    left:  transformed_center.z - radius,
                    right: transformed_center.z + radius,
                };
                extent.extend(&new_extent);
            }
        }
        extent
    }

    /// Furthest point of a hurtbox, starting from the bps, excludes intangible and invincible hurtboxes
    /// Returns None when there are no vulnerable hurtboxes
    pub fn hurt_box_vulnerable_extent(&self) -> Option<Extent> {
        let mut extent = Extent::new();
        let mut some = false;
        for hurt_box in &self.hurt_boxes {
            if let HurtBoxState::Normal = hurt_box.state {
                some = true;
                let bone_matrix = hurt_box.bone_matrix.clone();
                let bone_scale = Vector3::new(bone_matrix.x.magnitude(), bone_matrix.y.magnitude(), bone_matrix.z.magnitude());
              //let bone_trousle = https://www.youtube.com/watch?v=64cvrwzrmhU
                let radius = hurt_box.hurt_box.radius;
                let stretch = hurt_box.hurt_box.stretch;

                let stretch_face_temp = stretch / radius;
                let stretch_face = Vector3::new(
                    stretch_face_temp.x / bone_scale.x,
                    stretch_face_temp.y / bone_scale.y,
                    stretch_face_temp.z / bone_scale.z
                );

                let transform_scale = Matrix4::from_scale(radius);
                let transform_translation = Matrix4::from_translation(Vector3::new(
                    hurt_box.hurt_box.offset.x / (bone_scale.x * radius),
                    hurt_box.hurt_box.offset.y / (bone_scale.y * radius),
                    hurt_box.hurt_box.offset.z / (bone_scale.z * radius),
                ));
                let transform = bone_matrix * transform_scale * transform_translation;

                let sphere_8th_centers = [
                    Point3::new(0.0,            0.0,            0.0),
                    Point3::new(stretch_face.x, 0.0,            0.0),
                    Point3::new(0.0,            stretch_face.y, 0.0),
                    Point3::new(0.0,            0.0,            stretch_face.z),
                    Point3::new(stretch_face.x, stretch_face.y, 0.0),
                    Point3::new(0.0,            stretch_face.y, stretch_face.z),
                    Point3::new(stretch_face.x, 0.0,            stretch_face.z),
                ];

                for center in &sphere_8th_centers {
                    let transformed_center = transform.transform_point(*center);

                    // from the center of each sphere 8th we can apply the radius to get the maximum extent in all dimensions
                    let new_extent = Extent {
                        up:    transformed_center.y + radius,
                        down:  transformed_center.y - radius,
                        left:  transformed_center.z - radius,
                        right: transformed_center.z + radius,
                    };
                    extent.extend(&new_extent);
                }
            }
        }
        if some { Some(extent) } else { None }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Extent {
    pub left:  f32,
    pub right: f32,
    pub up:    f32,
    pub down:  f32,
}

impl Extent {
    pub fn new() -> Extent {
        Extent {
            left:  0.0,
            right: 0.0,
            up:    0.0,
            down:  0.0,
        }
    }

    pub fn extend(&mut self, other: &Extent) {
        if other.left < self.left {
            self.left = other.left;
        }
        if other.right > self.right {
            self.right = other.right;
        }
        if other.up > self.up {
            self.up = other.up;
        }
        if other.down < self.down {
            self.down = other.down;
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct HighLevelHurtBox {
    pub bone_matrix: Matrix4<f32>,
    pub hurt_box: HurtBox,
    pub state: HurtBoxState,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum CollisionBoxValues {
    Hit (HitBoxValues),
    Grab (GrabBoxValues),
}

impl CollisionBoxValues {
    pub(crate) fn from_hitbox(args: &HitBoxArguments, damage: f32) -> CollisionBoxValues {
        CollisionBoxValues::Hit(HitBoxValues {
            hitbox_id:            args.hitbox_id,
            set_id:               args.set_id,
            damage:               damage,
            trajectory:           args.trajectory,
            wdsk:                 args.wdsk,
            kbg:                  args.kbg,
            shield_damage:        args.shield_damage,
            bkb:                  args.bkb,
            size:                 args.size,
            tripping_rate:        args.tripping_rate,
            hitlag_mult:          args.hitlag_mult,
            sdi_mult:             args.sdi_mult,
            effect:               args.effect.clone(),
            sound_level:          args.sound_level,
            sound:                args.sound.clone(),
            ground:               args.ground,
            aerial:               args.aerial,
            sse_type:             args.sse_type.clone(),
            clang:                args.clang,
            direct:               args.direct,
            rehit_rate:           0, // TODO: ?
            angle_flipping:       AngleFlip::AwayFromAttacker, // TODO: ?
            stretches_to_bone:    false,
            can_hit1:             true,
            can_hit2:             true,
            can_hit3:             true,
            can_hit4:             true,
            can_hit5:             true,
            can_hit6:             true,
            can_hit7:             true,
            can_hit8:             true,
            can_hit9:             true,
            can_hit10:            true,
            can_hit11:            true,
            can_hit12:            true,
            can_hit13:            true,
            enabled:              true,
            can_be_shielded:      true,
            can_be_reflected:     false,
            can_be_absorbed:      false,
            remain_grabbed:       false,
            ignore_invincibility: false,
            freeze_frame_disable: false,
            flinchless:           false,
        })
    }

    pub(crate) fn from_special_hitbox(special_args: &SpecialHitBoxArguments, damage: f32) -> CollisionBoxValues {
        let args = &special_args.hitbox_args;
        CollisionBoxValues::Hit(HitBoxValues {
            hitbox_id:            args.hitbox_id,
            set_id:               args.set_id,
            damage:               damage,
            trajectory:           args.trajectory,
            wdsk:                 args.wdsk,
            kbg:                  args.kbg,
            shield_damage:        args.shield_damage,
            bkb:                  args.bkb,
            size:                 args.size,
            tripping_rate:        args.tripping_rate,
            hitlag_mult:          args.hitlag_mult,
            sdi_mult:             args.sdi_mult,
            effect:               args.effect.clone(),
            sound_level:          args.sound_level,
            sound:                args.sound.clone(),
            ground:               args.ground,
            aerial:               args.aerial,
            sse_type:             args.sse_type.clone(),
            clang:                args.clang,
            direct:               args.direct,
            rehit_rate:           special_args.rehit_rate,
            angle_flipping:       special_args.angle_flipping.clone(),
            stretches_to_bone:    special_args.stretches_to_bone,
            can_hit1:             special_args.can_hit1,
            can_hit2:             special_args.can_hit2,
            can_hit3:             special_args.can_hit3,
            can_hit4:             special_args.can_hit4,
            can_hit5:             special_args.can_hit5,
            can_hit6:             special_args.can_hit6,
            can_hit7:             special_args.can_hit7,
            can_hit8:             special_args.can_hit8,
            can_hit9:             special_args.can_hit9,
            can_hit10:            special_args.can_hit10,
            can_hit11:            special_args.can_hit11,
            can_hit12:            special_args.can_hit12,
            can_hit13:            special_args.can_hit13,
            enabled:              special_args.enabled,
            can_be_shielded:      special_args.can_be_shielded,
            can_be_reflected:     special_args.can_be_reflected,
            can_be_absorbed:      special_args.can_be_absorbed,
            remain_grabbed:       special_args.remain_grabbed,
            ignore_invincibility: special_args.ignore_invincibility,
            freeze_frame_disable: special_args.freeze_frame_disable,
            flinchless:           special_args.flinchless,
        })
    }

    pub(crate) fn from_grabbox(args: &GrabBoxArguments) -> CollisionBoxValues {
        CollisionBoxValues::Grab(GrabBoxValues {
            hitbox_id:  args.hitbox_id,
            size:       args.size,
            set_action: args.set_action,
            target:     args.target.clone(),
            unk:        args.unk.clone(),
        })
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GrabBoxValues {
    pub hitbox_id:  i32,
    pub size:       f32,
    pub set_action: i32,
    pub target:     GrabTarget,
    pub unk:        Option<i32>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct HitBoxValues {
    pub hitbox_id:            u8,
    pub set_id:               u8,
    pub damage:               f32,
    pub trajectory:           i32,
    pub wdsk:                 i16,
    pub kbg:                  i16,
    pub shield_damage:        i16,
    pub bkb:                  i16,
    pub size:                 f32,
    pub tripping_rate:        f32,
    pub hitlag_mult:          f32,
    pub sdi_mult:             f32,
    pub effect:               HitBoxEffect,
    pub sound_level:          u8,
    pub sound:                HitBoxSound,
    pub ground:               bool,
    pub aerial:               bool,
    pub sse_type:             HitBoxSseType,
    pub clang:                bool,
    pub direct:               bool,
    pub rehit_rate:           i32,
    pub angle_flipping:       AngleFlip,
    pub stretches_to_bone:    bool,
    pub can_hit1:             bool,
    pub can_hit2:             bool,
    pub can_hit3:             bool,
    pub can_hit4:             bool,
    pub can_hit5:             bool,
    pub can_hit6:             bool,
    pub can_hit7:             bool,
    pub can_hit8:             bool,
    pub can_hit9:             bool,
    pub can_hit10:            bool,
    pub can_hit11:            bool,
    pub can_hit12:            bool,
    pub can_hit13:            bool,
    pub enabled:              bool,
    pub can_be_shielded:      bool,
    pub can_be_reflected:     bool,
    pub can_be_absorbed:      bool,
    pub remain_grabbed:       bool,
    pub ignore_invincibility: bool,
    pub freeze_frame_disable: bool,
    pub flinchless:           bool,
}

impl HitBoxValues {
    pub fn can_hit_fighter(&self) -> bool {
        self.can_hit1
    }

    pub fn can_hit_waddle_dee_doo(&self) -> bool {
        self.can_hit1 || self.can_hit12
    }

    pub fn can_hit_pikmin(&self) -> bool {
        self.can_hit1 || self.can_hit12
    }

    pub fn can_hit_sse(&self) -> bool {
        self.can_hit2
    }

    pub fn can_hit_gyro(&self) -> bool {
        self.can_hit4 || self.can_hit11
    }

    pub fn can_hit_snake_grenade(&self) -> bool {
        self.can_hit4 || self.can_hit11
    }

    pub fn can_hit_mr_saturn(&self) -> bool {
        self.can_hit4 || self.can_hit11
    }

    pub fn can_hit_stage_non_wall_ceiling_floor(&self) -> bool {
        self.can_hit7 || self.can_hit11
    }

    pub fn can_hit_wall_ceiling_floor(&self) -> bool {
        self.can_hit8 || self.can_hit11
    }

    pub fn can_hit_link_bomb(&self) -> bool {
        self.can_hit9 || self.can_hit10
    }

    pub fn can_hit_bobomb(&self) -> bool {
        self.can_hit9 || self.can_hit10
    }
}

#[derive(Clone, Debug)]
struct PositionHitBox {
    pub hitbox_id:   u8,
    pub position:    Point3<f32>,
    pub size:        f32,
    pub interpolate: bool,
    pub values:      CollisionBoxValues,
}

#[derive(Serialize, Clone, Debug)]
pub struct HighLevelHitBox {
    pub hitbox_id: u8,

    /// This value doesnt take into account the distance travelled by the character that HighLevelFighter doesnt know about e.g. due to velocity from previous subaction
    pub prev_pos:    Option<Point3<f32>>,
    pub prev_size:   Option<f32>,
    pub prev_values: Option<CollisionBoxValues>,

    pub next_pos:    Point3<f32>,
    pub next_size:   f32,
    pub next_values: CollisionBoxValues,
}

#[derive(Serialize, Clone, Debug)]
pub struct ECB {
    pub left:     f32,
    pub right:    f32,
    pub top:      f32,
    pub bottom:   f32,
    pub transn_x: f32,
    pub transn_y: f32,
}

fn gen_ecb(bone: &BoneTransforms, ecb_bones: &[i32], bone_refs: &BoneRefs, mut ecb: ECB) -> ECB {
    for ecb_bone in ecb_bones {
        if bone.index == *ecb_bone {
            let x = bone.transform_normal.w.z;
            let y = bone.transform_normal.w.y;

            if x < ecb.left {
                ecb.left = x;
            }
            if x > ecb.right {
                ecb.right = x;
            }
            if y < ecb.bottom {
                ecb.bottom = y;
            }
            if y > ecb.top {
                ecb.top = y;
            }
        }
    }
    if bone.index == bone_refs.trans_n {
        ecb.transn_x = bone.transform_normal.w.z;
        ecb.transn_y = bone.transform_normal.w.y;
    }

    for child in bone.children.iter() {
        ecb = gen_ecb(child, ecb_bones, bone_refs, ecb);
    }
    ecb
}

fn gen_hurt_boxes(bone: &BoneTransforms, hurt_boxes: &[HurtBox], script_runner: &ScriptRunner, size: f32) -> Vec<HighLevelHurtBox> {
    let hurtbox_state_all = &script_runner.hurtbox_state_all;
    let hurtbox_states    = &script_runner.hurtbox_states;
    let invisible_bones   = &script_runner.invisible_bones;

    let mut hl_hurt_boxes = vec!();
    for hurt_box in hurt_boxes {
        if bone.index == get_bone_index(hurt_box.bone_index as i32) {
            let state = if let Some(state) = hurtbox_states.get(&bone.index) {
                state
            } else {
                hurtbox_state_all
            }.clone();

            if invisible_bones.iter().all(|x| get_bone_index(*x) != bone.index) {
                let mut hurt_box = hurt_box.clone();
                hurt_box.offset *= size;
                hurt_box.stretch *= size;
                // dont multiply radius as that will be multiplied by the bone_matrix

                hl_hurt_boxes.push(HighLevelHurtBox {
                    bone_matrix: bone.transform_normal,
                    hurt_box,
                    state,
                });
            }
        }
    }

    for child in bone.children.iter() {
        hl_hurt_boxes.extend(gen_hurt_boxes(child, hurt_boxes, script_runner, size));
    }

    hl_hurt_boxes
}

fn gen_hit_boxes(bone: &BoneTransforms, hit_boxes: &[ScriptCollisionBox]) -> Vec<PositionHitBox> {
    let mut pos_hit_boxes = vec!();
    for hit_box in hit_boxes.iter() {
        if bone.index == get_bone_index(hit_box.bone_index as i32) {
            let offset = Point3::new(hit_box.x_offset, hit_box.y_offset, hit_box.z_offset);
            let offset = bone.transform_hitbox.transform_point(offset);
            let position = Point3::new(
                offset.x + bone.transform_normal.w.x,
                offset.y + bone.transform_normal.w.y,
                offset.z + bone.transform_normal.w.z,
            );

            pos_hit_boxes.push(PositionHitBox {
                hitbox_id:   hit_box.hitbox_id,
                size:        hit_box.size,
                values:      hit_box.values.clone(),
                interpolate: hit_box.interpolate,
                position,
            });
        }
    }

    for child in bone.children.iter() {
        pos_hit_boxes.extend(gen_hit_boxes(child, hit_boxes));
    }

    pos_hit_boxes
}

// This is a basic (incorrect) implementation to handle wario and kirby's weird bone indices.
// Refer to https://github.com/libertyernie/brawltools/blob/83b79a571d84efc1884950204852a14eab58060e/Ikarus/Moveset%20Entries/MovesetNode.cs#L261
pub fn get_bone_index(index: i32) -> i32 {
    if index >= 400 {
        index - 400
    } else {
        index
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct SectionScriptAst {
    pub name:    String,
    pub script:  ScriptAst,
    pub callers: Vec<i32>,
}

impl SectionScriptAst {
    fn new(section_script: &SectionScript, external_subroutines: &[ExternalSubroutine]) -> SectionScriptAst {
        SectionScriptAst {
            name:    section_script.name.clone(),
            script:  ScriptAst::new(&section_script.script),
            callers: external_subroutines.iter().find(|x| x.name == section_script.name).map(|x| x.offsets.clone()).unwrap_or_default(),
        }
    }
}
