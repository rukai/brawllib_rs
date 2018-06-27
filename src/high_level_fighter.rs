use cgmath::{Vector3, Matrix4, SquareMatrix};

use chr0::Chr0;
use fighter::Fighter;
use mdl0::bones::Bone;
use misc_section::{LedgeGrab, HurtBox};
use sakurai::{FighterAttributes, AnimationFlags};
use script_ast::{ScriptAst, HitBoxArguments, SpecialHitBoxArguments, EdgeSlide, AngleFlip, Effect};
use script_runner::{ScriptRunner, ChangeSubAction, ScriptHitBox};

/// The HighLevelFighter stores processed Fighter data in a format that is easy to read from.
/// If brawllib_rs eventually implements the ability to modify character files via modifying Fighter and its children, then HighLevelFighter WILL NOT support that.
#[derive(Clone, Debug)]
pub struct HighLevelFighter {
    pub name: String,
    pub attributes: FighterAttributes,
    pub actions: Vec<HighLevelAction>,
    pub ledge_grabs: Vec<LedgeGrab>, // TODO: Instead of a single global vec, put a copy of the relevant LedgeGrab in HighLevelFrame
    pub fragment_scripts: Vec<ScriptAst>,
}

impl HighLevelFighter {
    /// Processes data from an &Fighter and stores it in a HighLevelFighter
    pub fn new(fighter: &Fighter) -> HighLevelFighter {
        info!("Generating HighLevelFighter for {}", fighter.cased_name);
        let fighter_data = fighter.get_fighter_data();
        let attributes = fighter_data.unwrap().attributes.clone();
        let mut actions = vec!();
        if let Some(first_bone) = fighter.get_bones() {
            for chr0 in fighter.get_animations() {
                let name = chr0.name.clone();
                let mut animation_flags = None;
                let mut scripts = None;
                if let Some(fighter_data) = fighter_data {
                    for i in 0..fighter_data.sub_action_main.len() {
                        let sub_action_flags = &fighter_data.sub_action_flags[i];
                        if sub_action_flags.name == chr0.name {
                            animation_flags = Some(sub_action_flags.animation_flags.clone());
                            //info!("{}", name);
                            scripts = Some(HighLevelScripts {
                                script_main:  ScriptAst::new(&fighter_data.sub_action_main[i]),
                                script_gfx:   ScriptAst::new(&fighter_data.sub_action_gfx[i]),
                                script_sfx:   ScriptAst::new(&fighter_data.sub_action_sfx[i]),
                                script_other: ScriptAst::new(&fighter_data.sub_action_other[i]),
                            });
                        }
                    }
                }
                let script_refs = if let &Some(ref scripts) = &scripts {
                    vec!(&scripts.script_main, &scripts.script_gfx, &scripts.script_sfx, &scripts.script_other)
                } else {
                    vec!()
                };
                let animation_flags = animation_flags.unwrap_or(AnimationFlags::NONE);

                let mut frames: Vec<HighLevelFrame> = vec!();
                let mut prev_offset = None;
                let mut script_runner = ScriptRunner::new(&script_refs);
                let mut iasa = None;
                let mut prev_hit_boxes: Option<Vec<PositionHitBox>> = None;

                let num_frames = match name.as_ref() {
                    "LandingAirN"  => attributes.nair_landing_lag,
                    "LandingAirF"  => attributes.fair_landing_lag,
                    "LandingAirB"  => attributes.bair_landing_lag,
                    "LandingAirHi" => attributes.uair_landing_lag,
                    "LandingAirLw" => attributes.dair_landing_lag,
                    "LandingLight" => attributes.light_landing_lag,
                    "LandingHeavy" => attributes.normal_landing_lag,
                    _              => chr0.num_frames as f32
                };
                while script_runner.frame_index < num_frames {
                    let mut first_bone = first_bone.clone();
                    let chr0_frame_index = script_runner.frame_index * chr0.num_frames as f32 / num_frames; // map frame count between [0, chr0.num_frames]
                    let next_offset = HighLevelFighter::apply_chr0_to_bones(&mut first_bone, Matrix4::<f32>::identity(), chr0, chr0_frame_index as i32, animation_flags);
                    let hurt_boxes = gen_hurt_boxes(&first_bone, &fighter_data.unwrap().misc.hurt_boxes);
                    let hit_boxes: Vec<_> = script_runner.hitboxes.iter().filter(|x| x.is_some()).map(|x| x.clone().unwrap()).collect();
                    let hit_boxes = gen_hit_boxes(&first_bone, &hit_boxes);
                    let mut hl_hit_boxes = vec!();
                    for next in &hit_boxes {
                        let mut prev = None;
                        let mut prev_values = None;
                        if let &Some(ref prev_hit_boxes) = &prev_hit_boxes {
                            for prev_hit_box in prev_hit_boxes {
                                if prev_hit_box.values.hitbox_index == next.values.hitbox_index {
                                    prev = Some(prev_hit_box.position);
                                    prev_values = Some(prev_hit_box.values.clone());
                                }
                            }
                        }
                        hl_hit_boxes.push(HighLevelHitBox {
                            prev,
                            prev_values,
                            next:        next.position,
                            next_values: next.values.clone(),
                        });
                    }

                    let animation_velocity = match (prev_offset, next_offset) {
                        (Some(prev_offset), Some(next_offset)) => Some(next_offset - prev_offset),
                        (Some(_),           None)              => unreachable!(),
                        (None,              Some(next_offset)) => Some(next_offset),
                        (None,              None)              => None
                    };
                    prev_offset = next_offset;

                    // TODO: get these from the fighter data
                    let min_width = 2.0;
                    let min_height = 2.0;

                    // TODO: figure out how exactly these min values are supposed to work.
                    let ecb = ECB {
                        left:   -min_width / 2.0,
                        right:  min_width / 2.0,
                        top:    min_height,
                        bottom: if script_runner.airbourne { min_height } else { 0.0 }
                    };
                    let ecb = gen_ecb(&first_bone, &fighter_data.unwrap().misc.ecb_bones, ecb);

                    frames.push(HighLevelFrame {
                        ecb,
                        animation_velocity,
                        hurt_boxes,
                        hit_boxes:           hl_hit_boxes,
                        interruptible:       script_runner.interruptible,
                        edge_slide:          script_runner.edge_slide.clone(),
                        airbourne:           script_runner.airbourne,
                        hitlist_reset:       script_runner.hitlist_reset,
                        slope_contour_stand: script_runner.slope_contour_stand,
                        slope_contour_full:  script_runner.slope_contour_full,
                        rumble:              script_runner.rumble,
                        rumble_loop:         script_runner.rumble_loop,
                    });

                    if iasa.is_none() && script_runner.interruptible {
                        iasa = Some(script_runner.frame_index)
                    }

                    script_runner.step(name.as_ref());
                    prev_hit_boxes = Some(hit_boxes);

                    if let ChangeSubAction::Continue = script_runner.change_sub_action { } else { break }
                }

                let iasa = if let Some(iasa) = iasa {
                    iasa
                } else {
                    match name.as_ref() {
                        "LandingAirN"  | "LandingAirF" |
                        "LandingAirB"  | "LandingAirHi" |
                        "LandingAirLw" | "LandingLight" |
                        "LandingHeavy" | "LandingFallSpecial"
                            => script_runner.frame_index,
                        _                    => 0.0
                    }
                } as usize;

                actions.push(HighLevelAction { name, iasa, frames, animation_flags, scripts });
            }
        }

        // TODO: Delete this
        let fragment_scripts: Vec<_> = fighter_data.unwrap().fragment_scripts.iter().map(|x| ScriptAst::new(x)).collect();
        info!("fragment_scripts: {:#?}", fragment_scripts);

        HighLevelFighter {
            name:              fighter.cased_name.clone(),
            ledge_grabs:       fighter_data.unwrap().misc.ledge_grabs.clone(),
            fragment_scripts:  fighter_data.unwrap().fragment_scripts.iter().map(|x| ScriptAst::new(x)).collect(),
            attributes,
            actions,
        }
    }

    /// Modifies, in place, the matrices of the passed tree of bones, to follow that of the specified animation frame
    /// The resulting matrices are independent of its parent bones matrix.
    /// Returns the MOVES_CHARACTER offset if enabled. this is used by e.g. Ness's double jump
    fn apply_chr0_to_bones(bone: &mut Bone, parent_transform: Matrix4<f32>, chr0: &Chr0, frame: i32, animation_flags: AnimationFlags) -> Option<Vector3<f32>> {
        let moves_character = animation_flags.contains(AnimationFlags::MOVES_CHARACTER);

        // by default the bones tpose transformation is used.
        bone.transform = parent_transform * bone.gen_transform();
        let mut offset = None;
        for chr0_child in &chr0.children {
            let transform = parent_transform * chr0_child.get_transform(chr0.loop_value, frame);
            // in this case TransN is not part of the animation but instead used to move the character in game.
            if moves_character && bone.name == "TransN" {
                offset = Some(Vector3::new(transform.w.x, transform.w.y, transform.w.z));
                // TODO: Should this case modify bone.transform rot and scale?
            }
            // the animation specifies a transform for this bone, USE IT!
            else if chr0_child.name == bone.name {
                bone.transform = transform;
            }
        }

        // do the same for all children bones
        for child in bone.children.iter_mut() {
            if let Some(result) = HighLevelFighter::apply_chr0_to_bones(child, bone.transform, chr0, frame, animation_flags) {
                offset = Some(result);
            }
        }
        offset
    }
}

#[derive(Clone, Debug)]
pub struct HighLevelAction {
    pub name:            String,
    pub iasa:            usize,
    pub frames:          Vec<HighLevelFrame>,
    pub animation_flags: AnimationFlags,
    pub scripts:         Option<HighLevelScripts>,
}

#[derive(Clone, Debug)]
pub struct HighLevelScripts {
    pub script_main:  ScriptAst,
    pub script_gfx:   ScriptAst,
    pub script_sfx:   ScriptAst,
    pub script_other: ScriptAst,
}

#[derive(Clone, Debug)]
pub struct HighLevelFrame {
    pub hurt_boxes:          Vec<HighLevelHurtBox>,
    pub hit_boxes:           Vec<HighLevelHitBox>,
    pub animation_velocity:  Option<Vector3<f32>>,
    pub interruptible:       bool,
    pub edge_slide:          EdgeSlide,
    pub airbourne:           bool,
    pub ecb:                 ECB,
    pub hitlist_reset:       bool,
    pub slope_contour_stand: Option<i32>,
    pub slope_contour_full:  Option<(i32, i32)>,
    pub rumble:              Option<(i32, i32)>,
    pub rumble_loop:         Option<(i32, i32)>,
}

#[derive(Clone, Debug)]
pub struct HighLevelHurtBox {
    pub bone_matrix: Matrix4<f32>,
    pub hurt_box: HurtBox,
}

#[derive(Clone, Debug)]
pub struct HitBoxValues {
    pub hitbox_index:                   u8,
    pub rehit_hitbox_index:             u8,
    pub damage:                         i32,
    pub trajectory:                     i32,
    pub weight_knockback:               i16,
    pub kbg:                            i16,
    pub shield_damage:                  i16,
    pub bkb:                            i16,
    pub size:                           f32,
    pub tripping_rate:                  f32,
    pub hitlag_mult:                    f32,
    pub di_mult:                        f32,
    pub effect:                         Effect,
    pub sound_level:                    u8,
    pub ground:                         bool,
    pub aerial:                         bool,
    pub ty:                             u8,
    pub clang:                          bool,
    pub direct:                         bool,
    pub rehit_rate:                     i32,
    pub angle_flipping:                 AngleFlip,
    pub stretches:                      bool,
    pub can_hit_multiplayer_characters: bool,
    pub can_hit_sse_enemies:            bool,
    pub can_hit_damageable_ceilings:    bool,
    pub can_hit_damageable_walls:       bool,
    pub can_hit_damageable_floors:      bool,
    pub enabled:                        bool,
    pub can_be_shielded:                bool,
    pub can_be_reflected:               bool,
    pub can_be_absorbed:                bool,
    pub can_hit_gripped_character:      bool,
    pub ignore_invincibility:           bool,
    pub freeze_frame_disable:           bool,
    pub flinchless:                     bool,
}

impl HitBoxValues {
    pub fn from_hitbox(args: &HitBoxArguments) -> HitBoxValues {
        HitBoxValues {
            hitbox_index:                   args.hitbox_index,
            rehit_hitbox_index:             args.rehit_hitbox_index,
            damage:                         args.damage,
            trajectory:                     args.trajectory,
            weight_knockback:               args.weight_knockback,
            kbg:                            args.kbg,
            shield_damage:                  args.shield_damage,
            bkb:                            args.bkb,
            size:                           args.size,
            tripping_rate:                  args.tripping_rate,
            hitlag_mult:                    args.hitlag_mult,
            di_mult:                        args.di_mult,
            effect:                         args.effect.clone(),
            sound_level:                    args.sound_level,
            ground:                         args.ground,
            aerial:                         args.aerial,
            ty:                             args.ty,
            clang:                          args.clang,
            direct:                         args.direct,
            rehit_rate:                     0, // TODO: ?
            angle_flipping:                 AngleFlip::AwayFromAttacker, // TODO: ?
            stretches:                      false,
            can_hit_multiplayer_characters: true,
            can_hit_sse_enemies:            true,
            can_hit_damageable_ceilings:    true,
            can_hit_damageable_walls:       true,
            can_hit_damageable_floors:      true,
            enabled:                        true,
            can_be_shielded:                true,
            can_be_reflected:               false,
            can_be_absorbed:                false,
            can_hit_gripped_character:      true,
            ignore_invincibility:           false,
            freeze_frame_disable:           false,
            flinchless:                     false,
        }
    }

    pub fn from_special_hitbox(special_args: &SpecialHitBoxArguments) -> HitBoxValues {
        let args = &special_args.hitbox_args;
        HitBoxValues {
            hitbox_index:                   args.hitbox_index,
            rehit_hitbox_index:             args.rehit_hitbox_index,
            damage:                         args.damage,
            trajectory:                     args.trajectory,
            weight_knockback:               args.weight_knockback,
            kbg:                            args.kbg,
            shield_damage:                  args.shield_damage,
            bkb:                            args.bkb,
            size:                           args.size,
            tripping_rate:                  args.tripping_rate,
            hitlag_mult:                    args.hitlag_mult,
            di_mult:                        args.di_mult,
            effect:                         args.effect.clone(),
            sound_level:                    args.sound_level,
            ground:                         args.ground,
            aerial:                         args.aerial,
            ty:                             args.ty,
            clang:                          args.clang,
            direct:                         args.direct,
            rehit_rate:                     special_args.rehit_rate,
            angle_flipping:                 special_args.angle_flipping.clone(),
            stretches:                      special_args.stretches,
            can_hit_multiplayer_characters: special_args.can_hit_multiplayer_characters,
            can_hit_sse_enemies:            special_args.can_hit_sse_enemies,
            can_hit_damageable_ceilings:    special_args.can_hit_damageable_ceilings,
            can_hit_damageable_walls:       special_args.can_hit_damageable_walls,
            can_hit_damageable_floors:      special_args.can_hit_damageable_floors,
            enabled:                        special_args.enabled,
            can_be_shielded:                special_args.can_be_shielded,
            can_be_reflected:               special_args.can_be_reflected,
            can_be_absorbed:                special_args.can_be_absorbed,
            can_hit_gripped_character:      special_args.can_hit_gripped_character,
            ignore_invincibility:           special_args.ignore_invincibility,
            freeze_frame_disable:           special_args.freeze_frame_disable,
            flinchless:                     special_args.flinchless,
        }
    }
}

#[derive(Clone, Debug)]
struct PositionHitBox {
    pub position: Vector3<f32>,
    pub values:   HitBoxValues,
}

#[derive(Clone, Debug)]
pub struct HighLevelHitBox {
    pub prev:        Option<Vector3<f32>>,
    pub prev_values: Option<HitBoxValues>,
    pub next:        Vector3<f32>,
    pub next_values: HitBoxValues,
}

#[derive(Clone, Debug)]
pub struct ECB {
    pub left:   f32,
    pub right:  f32,
    pub top:    f32,
    pub bottom: f32,
}

fn gen_ecb(bone: &Bone, ecb_bones: &[i32], mut ecb: ECB) -> ECB {
    for ecb_bone in ecb_bones {
        if bone.index == *ecb_bone {
            let x = bone.transform.w.z;
            let y = bone.transform.w.y;

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

fn gen_hurt_boxes(bone: &Bone, hurt_boxes: &[HurtBox]) -> Vec<HighLevelHurtBox> {
    let mut hl_hurt_boxes = vec!();
    for hurt_box in hurt_boxes {
        if bone.index == hurt_box.bone_index as i32 {
            hl_hurt_boxes.push(HighLevelHurtBox {
                bone_matrix: bone.transform,
                hurt_box: hurt_box.clone(),
            });
        }
    }

    for child in bone.children.iter() {
        hl_hurt_boxes.extend(gen_hurt_boxes(child, hurt_boxes));
    }

    hl_hurt_boxes
}

fn gen_hit_boxes(bone: &Bone, hit_boxes: &[ScriptHitBox]) -> Vec<PositionHitBox> {
    let mut hl_hit_boxes = vec!();
    for hit_box in hit_boxes.iter() {
        if bone.index == hit_box.bone_index as i32 {
            let transform = bone.transform * Matrix4::from_translation(Vector3::new(hit_box.x_offset, hit_box.y_offset, hit_box.z_offset));
            hl_hit_boxes.push(PositionHitBox {
                position: Vector3::new(transform.w.x, transform.w.y, transform.w.z),
                values:   hit_box.values.clone()
            });
        }
    }

    for child in bone.children.iter() {
        hl_hit_boxes.extend(gen_hit_boxes(child, hit_boxes));
    }

    hl_hit_boxes
}
