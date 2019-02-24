use cgmath::{Point3, Vector3, Matrix4, SquareMatrix, InnerSpace, Transform};
use rayon::prelude::*;

use crate::chr0::Chr0;
use crate::fighter::Fighter;
use crate::mdl0::bones::Bone;
use crate::sakurai::fighter_data::misc_section::{LedgeGrab, HurtBox};
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
};
use crate::script_runner::{ScriptRunner, ChangeSubaction, ScriptCollisionBox, VelModify};

use std::collections::HashMap;

/// The HighLevelFighter stores processed Fighter data in a format that is easy to read from.
/// If brawllib_rs eventually implements the ability to modify character files via modifying Fighter and its children, then HighLevelFighter WILL NOT support that.
#[derive(Serialize, Clone, Debug)]
pub struct HighLevelFighter {
    pub name:                     String,
    pub internal_name:            String,
    pub attributes:               FighterAttributes,
    pub actions:                  Vec<HighLevelAction>,
    pub subactions:               Vec<HighLevelSubaction>,
    pub ledge_grabs:              Vec<LedgeGrab>, // TODO: Instead of a single global vec, put a copy of the relevant LedgeGrab in HighLevelFrame
    pub scripts_fragment_fighter: Vec<ScriptAst>,
    pub scripts_fragment_common:  Vec<ScriptAst>,
}

impl HighLevelFighter {
    /// Processes data from an &Fighter and stores it in a HighLevelFighter
    // TODO: Maybe expose a `multithreaded` argument so caller can disable multithread and run its own multithreading on the entire `HighLevelFighter::new`.
    // Because rayon uses a threadpool we arent at risk of it hammering the system by spawning too many threads.
    // However it may be ineffecient due to overhead of spawning threads for every action.
    // Will need to benchmark any such changes.
    pub fn new(fighter: &Fighter) -> HighLevelFighter {
        info!("Generating HighLevelFighter for {}", fighter.cased_name);
        let fighter_data = fighter.get_fighter_data().unwrap();
        let fighter_data_common = fighter.get_fighter_data_common().unwrap();
        let attributes = fighter_data.attributes.clone();
        let fighter_animations = fighter.get_animations();

        let fragment_scripts_fighter: Vec<ScriptAst> = fighter_data.fragment_scripts.iter().map(|x| ScriptAst::new(x)).collect();
        let subaction_main:          Vec<ScriptAst> = fighter_data.subaction_main .iter().map(|x| ScriptAst::new(x)).collect();
        let subaction_gfx:           Vec<ScriptAst> = fighter_data.subaction_gfx  .iter().map(|x| ScriptAst::new(x)).collect();
        let subaction_sfx:           Vec<ScriptAst> = fighter_data.subaction_sfx  .iter().map(|x| ScriptAst::new(x)).collect();
        let subaction_other:         Vec<ScriptAst> = fighter_data.subaction_other.iter().map(|x| ScriptAst::new(x)).collect();

        let fragment_scripts_common: Vec<ScriptAst> = fighter_data_common.fragment_scripts.iter().map(|x| ScriptAst::new(x)).collect();
        let entry_actions: Vec<ScriptAst> = fighter_data_common.entry_actions.iter().map(|x| ScriptAst::new(x))
            .chain(fighter_data.entry_actions.iter().map(|x| ScriptAst::new(x)))
            .collect();
        let exit_actions: Vec<ScriptAst> = fighter_data_common.exit_actions.iter().map(|x| ScriptAst::new(x))
            .chain(fighter_data.exit_actions.iter().map(|x| ScriptAst::new(x)))
            .collect();

        let mut all_scripts = vec!();
        for script in fragment_scripts_fighter.iter()
            .chain(subaction_main.iter())
            .chain(subaction_gfx.iter())
            .chain(subaction_sfx.iter())
            .chain(subaction_other.iter())
            .chain(entry_actions.iter())
            .chain(exit_actions.iter())
        {
            all_scripts.push(script);
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
        for i in 0..entry_actions.len() {
            actions.push(HighLevelAction {
                name:         crate::action_names::action_name(i),
                script_entry: entry_actions[i].clone(),
                script_exit:  exit_actions[i].clone(),
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
                let mut script_runner = ScriptRunner::new(&action_scripts, &all_scripts);
                let mut iasa = None;
                let mut prev_hit_boxes: Option<Vec<PositionHitBox>> = None;

                if let Some(chr0) = chr0 {
                    let num_frames = match actual_name.as_ref() {
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

                    while script_runner.frame_index < num_frames {
                        let chr0_frame_index = script_runner.frame_index * chr0.num_frames as f32 / num_frames; // map frame count between [0, chr0.num_frames]
                        let (animation_xyz_offset, frame_bones) = HighLevelFighter::transform_bones(
                            &first_bone,
                            Matrix4::<f32>::identity(),
                            Matrix4::<f32>::identity(),
                            chr0,
                            chr0_frame_index as i32,
                            animation_flags
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

                        let hurt_boxes = gen_hurt_boxes(&frame_bones, &fighter_data.misc.hurt_boxes, &script_runner.hurtbox_state_all, &script_runner.hurtbox_states);
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

                        // TODO: get these from the fighter data
                        let min_width = 2.0;
                        let min_height = 2.0;

                        // TODO: figure out how exactly these min values are supposed to work.
                        let min_ecb = ECB {
                            left:   -min_width / 2.0,
                            right:  min_width / 2.0,
                            top:    min_height,
                            bottom: if script_runner.airbourne { min_height } else { 0.0 }
                        };
                        let ecb = gen_ecb(&frame_bones, &fighter_data.misc.ecb_bones, min_ecb);

                        frames.push(HighLevelFrame {
                            ecb,
                            x_pos,
                            y_pos,
                            x_vel_modify,
                            y_vel_modify,
                            x_vel_temp,
                            y_vel_temp,
                            hurt_boxes,
                            hit_boxes:           hl_hit_boxes,
                            interruptible:       script_runner.interruptible,
                            landing_lag:         script_runner.landing_lag,
                            edge_slide:          script_runner.edge_slide.clone(),
                            airbourne:           script_runner.airbourne,
                            hitbox_sets_rehit:   script_runner.hitbox_sets_rehit,
                            slope_contour_stand: script_runner.slope_contour_stand,
                            slope_contour_full:  script_runner.slope_contour_full,
                            rumble:              script_runner.rumble,
                            rumble_loop:         script_runner.rumble_loop,
                        });

                        if iasa.is_none() && script_runner.interruptible {
                            iasa = Some(script_runner.frame_index)
                        }

                        script_runner.step(actual_name.as_ref());
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
                          => script_runner.frame_index,
                        _ => 0.0
                    }
                } as usize;

                let landing_lag = match actual_name.as_ref() {
                    "AttackAirN"  => Some(attributes.nair_landing_lag),
                    "AttackAirF"  => Some(attributes.fair_landing_lag),
                    "AttackAirB"  => Some(attributes.bair_landing_lag),
                    "AttackAirHi" => Some(attributes.uair_landing_lag),
                    "AttackAirLw" => Some(attributes.dair_landing_lag),
                    _             => None,
                };

                HighLevelSubaction { name, iasa, landing_lag, frames, animation_flags, scripts }
            }).collect()
        } else {
            vec!()
        };

        HighLevelFighter {
            internal_name:            fighter.cased_name.clone(),
            name:                     crate::fighter_names::fighter_name(&fighter.cased_name),
            ledge_grabs:              fighter_data.misc.ledge_grabs.clone(),
            scripts_fragment_fighter: fragment_scripts_fighter,
            scripts_fragment_common:  fragment_scripts_common,
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
    fn transform_bones(bone: &Bone, parent_transform: Matrix4<f32>, parent_transform_hitbox: Matrix4<f32>, chr0: &Chr0, frame: i32, animation_flags: AnimationFlags) -> (Option<Vector3<f32>>, BoneTransforms) {
        let moves_character = animation_flags.contains(AnimationFlags::MOVES_CHARACTER);

        // by default the bones tpose transformation is used.
        let mut transform_normal = parent_transform * bone.gen_transform();
        let mut transform_hitbox = parent_transform_hitbox * bone.gen_transform_rot_only();

        // if the animation specifies a transform for the bone, override the models default tpose transform.
        let mut offset = None;
        for chr0_child in &chr0.children {
            if chr0_child.name == bone.name {
                let transform = parent_transform * chr0_child.get_transform(chr0.loop_value, frame);
                if moves_character && bone.name == "TransN" {
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

        // Ignore any transformations from the models tpose TopN bone or the animations TopN bone
        if bone.name == "TopN" {
            transform_normal = Matrix4::identity();
            transform_hitbox = Matrix4::identity();
        }

        // do the same for all children bones
        let mut children = vec!();
        for child in bone.children.iter() {
            let (moves, processed_child) = HighLevelFighter::transform_bones(child, transform_normal, transform_hitbox, chr0, frame, animation_flags);
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
}

#[derive(Serialize, Clone, Debug)]
pub struct HighLevelSubaction {
    pub name:            String,
    pub iasa:            usize,
    pub frames:          Vec<HighLevelFrame>,
    pub landing_lag:     Option<f32>,
    pub animation_flags: AnimationFlags,
    pub scripts:         HighLevelScripts,
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
pub struct HighLevelFrame {
    pub hurt_boxes:          Vec<HighLevelHurtBox>,
    pub hit_boxes:           Vec<HighLevelHitBox>,
    pub x_pos:               f32,
    pub y_pos:               f32,
    pub interruptible:       bool,
    pub edge_slide:          EdgeSlide,
    pub airbourne:           bool,
    pub landing_lag:         bool,
    pub ecb:                 ECB,
    pub hitbox_sets_rehit:   [bool; 10],
    pub slope_contour_stand: Option<i32>,
    pub slope_contour_full:  Option<(i32, i32)>,
    pub rumble:              Option<(i32, i32)>,
    pub rumble_loop:         Option<(i32, i32)>,
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
    pub(crate) fn from_hitbox(args: &HitBoxArguments) -> CollisionBoxValues {
        CollisionBoxValues::Hit(HitBoxValues {
            hitbox_id:            args.hitbox_id,
            set_id:               args.set_id,
            damage:               args.damage,
            trajectory:           args.trajectory,
            weight_knockback:     args.weight_knockback,
            kbg:                  args.kbg,
            shield_damage:        args.shield_damage,
            bkb:                  args.bkb,
            size:                 args.size,
            tripping_rate:        args.tripping_rate,
            hitlag_mult:          args.hitlag_mult,
            di_mult:              args.di_mult,
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

    pub(crate) fn from_special_hitbox(special_args: &SpecialHitBoxArguments) -> CollisionBoxValues {
        let args = &special_args.hitbox_args;
        CollisionBoxValues::Hit(HitBoxValues {
            hitbox_id:            args.hitbox_id,
            set_id:               args.set_id,
            damage:               args.damage,
            trajectory:           args.trajectory,
            weight_knockback:     args.weight_knockback,
            kbg:                  args.kbg,
            shield_damage:        args.shield_damage,
            bkb:                  args.bkb,
            size:                 args.size,
            tripping_rate:        args.tripping_rate,
            hitlag_mult:          args.hitlag_mult,
            di_mult:              args.di_mult,
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
    pub damage:               i32,
    pub trajectory:           i32,
    pub weight_knockback:     i16,
    pub kbg:                  i16,
    pub shield_damage:        i16,
    pub bkb:                  i16,
    pub size:                 f32,
    pub tripping_rate:        f32,
    pub hitlag_mult:          f32,
    pub di_mult:              f32,
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
    pub left:   f32,
    pub right:  f32,
    pub top:    f32,
    pub bottom: f32,
}

fn gen_ecb(bone: &BoneTransforms, ecb_bones: &[i32], mut ecb: ECB) -> ECB {
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

    for child in bone.children.iter() {
        ecb = gen_ecb(child, ecb_bones, ecb);
    }
    ecb
}

fn gen_hurt_boxes(bone: &BoneTransforms, hurt_boxes: &[HurtBox], hurtbox_state_all: &HurtBoxState, hurtbox_states: &HashMap<i32, HurtBoxState>) -> Vec<HighLevelHurtBox> {
    let mut hl_hurt_boxes = vec!();
    for hurt_box in hurt_boxes {
        if bone.index == get_bone_index(hurt_box.bone_index as i32) {
            let state = if let Some(state) = hurtbox_states.get(&bone.index) {
                state
            } else {
                hurtbox_state_all
            }.clone();

            hl_hurt_boxes.push(HighLevelHurtBox {
                bone_matrix: bone.transform_normal,
                hurt_box:    hurt_box.clone(),
                state,
            });
        }
    }

    for child in bone.children.iter() {
        hl_hurt_boxes.extend(gen_hurt_boxes(child, hurt_boxes, hurtbox_state_all, hurtbox_states));
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
    if index > 400 {
        index - 400
    } else {
        index
    }
}
