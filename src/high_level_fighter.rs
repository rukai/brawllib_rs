use cgmath::{Vector3, Matrix4, SquareMatrix};

use arc::ArcChildData;
use bres::BresChildData;
use chr0::Chr0;
use fighter::Fighter;
use mdl0::bones::Bone;
use misc_section::{LedgeGrab, HurtBox};
use sakurai::{SectionData, FighterAttributes, AnimationFlags};
use script_ast::{ScriptAst, HitBoxArguments, SpecialHitBoxArguments, EdgeSlide};
use script_ast;
use script_runner::ScriptRunner;

/// The HighLevelFighter stores processed Fighter data in a format that is easy to read from.
/// If brawllib_rs eventually implements the ability to modify character files via modifying Fighter and its children, then HighLevelFighter WILL NOT support that.
#[derive(Clone, Debug)]
pub struct HighLevelFighter {
    pub name: String,
    pub attributes: FighterAttributes,
    pub actions: Vec<HighLevelAction>,
    pub ledge_grabs: Vec<LedgeGrab> // TODO: Instead of a single global vec, put a copy of the relevant LedgeGrab in HighLevelFrame
}

impl HighLevelFighter {
    /// Processes data from an &Fighter and stores it in a HighLevelFighter
    pub fn new(fighter: &Fighter) -> HighLevelFighter {
        // locate fighter data
        let mut fighter_data = None;
        for sub_arc in &fighter.moveset.children {
            match &sub_arc.data {
                &ArcChildData::Sakurai (ref data) => {
                    for section in &data.sections {
                        if let &SectionData::FighterData (ref fighter_data_ref) = &section.data {
                            fighter_data = Some(fighter_data_ref);
                        }
                    }
                }
                _ => { }
            }
        }

        // locate bones
        let mut first_bone: Option<&Bone> = None;
        if let Some(model) = fighter.models.get(0) {
            for sub_arc in model.children.iter() {
                match &sub_arc.data {
                    &ArcChildData::Arc (_) => {
                        panic!("Not expecting arc at this level")
                    }
                    &ArcChildData::Bres (ref bres) => {
                        for bres_child in bres.children.iter() {
                            match &bres_child.data {
                                &BresChildData::Bres (ref model) => {
                                    for model_child in model.children.iter() {
                                        if model_child.name == format!("Fit{}00", fighter.cased_name) {
                                            match &model_child.data {
                                                &BresChildData::Mdl0 (ref model) => {
                                                    first_bone = model.bones.as_ref();
                                                }
                                                _ => { }
                                            }
                                        }
                                    }
                                }
                                &BresChildData::Mdl0 (_) => {
                                    panic!("Not expecting Mdl at this level");
                                }
                                _ => { }
                            }
                        }
                    }
                    _ => { }
                }
            }
        }

        // locate animations
        let mut chr0s: Vec<&Chr0> = vec!();
        for sub_arc in &fighter.motion.children {
            match &sub_arc.data {
                &ArcChildData::Arc (ref arc) => {
                    for sub_arc in &arc.children {
                        match &sub_arc.data {
                            &ArcChildData::Bres (ref bres) => {
                                for bres_child in bres.children.iter() {
                                    match &bres_child.data {
                                        &BresChildData::Bres (ref bres) => {
                                            for bres_child in bres.children.iter() {
                                                match &bres_child.data {
                                                    &BresChildData::Bres (_) => {
                                                        panic!("Not expecting bres at this level");
                                                    }
                                                    &BresChildData::Chr0 (ref chr0) => {
                                                        chr0s.push(chr0);
                                                    }
                                                    _ => { }
                                                }
                                            }
                                        }
                                        &BresChildData::Chr0 (_) => {
                                            panic!("Not expecting Chr0 at this level");
                                        }
                                        _ => { }
                                    }
                                }
                            }
                            &ArcChildData::Arc (_) => {
                                //panic!("Not expecting arc at this level"); // TODO: Whats here
                            }
                            _ => { }
                        }
                    }
                }
                &ArcChildData::Bres (_) => {
                    panic!("Not expecting bres at this level");
                }
                _ => { }
            }
        }

        // create fighter actions
        let mut actions = vec!();
        if let Some(first_bone) = first_bone {
            for chr0 in chr0s {
                // TODO: DELETE THIS
                //if chr0.name == "AttackAirHi" {
                if chr0.name == "AttackS4S" && false {
                //if chr0.name == "AttackAirB" {
                //if chr0.name == "Wait1" {
                    //println!("{:#?}", chr0);
                    println!("animation name: {}", chr0.name);
                    for child in &chr0.children {
                        if child.name == "YRotN" {
                            println!("{}", child.debug_string(chr0.loop_value, chr0.num_frames as i32));
                        }
                    }
                }

                let mut animation_flags = None;
                let mut scripts = None;
                if let Some(fighter_data) = fighter_data {
                    for i in 0..fighter_data.sub_action_main.len() {
                        let sub_action_flags = &fighter_data.sub_action_flags[i];
                        if sub_action_flags.name == chr0.name {
                            animation_flags = Some(sub_action_flags.animation_flags.clone());
                            scripts = Some(HighLevelScripts {
                                script_main:  script_ast::script_ast(&fighter_data.sub_action_main[i].events),
                                script_gfx:   script_ast::script_ast(&fighter_data.sub_action_gfx[i].events),
                                script_sfx:   script_ast::script_ast(&fighter_data.sub_action_sfx[i].events),
                                script_other: script_ast::script_ast(&fighter_data.sub_action_other[i].events),
                            });
                        }
                    }
                }
                let animation_flags = animation_flags.unwrap_or(AnimationFlags::NONE);

                let mut frames: Vec<HighLevelFrame> = vec!();
                let mut prev_offset = None;
                let mut script_runner = ScriptRunner::new();
                let mut iasa = None;
                while (script_runner.frame_index as u16) < chr0.num_frames {
                    let mut first_bone = first_bone.clone();
                    let next_offset = HighLevelFighter::apply_chr0_to_bones(&mut first_bone, Matrix4::<f32>::identity(), chr0, script_runner.frame_index as i32, animation_flags);
                    let hurt_boxes = gen_hurt_boxes(&first_bone, &fighter_data.unwrap().misc.hurt_boxes);
                    let hit_boxes  = gen_hit_boxes(&first_bone, &script_runner.hitboxes);
                    let special_hit_boxes  = gen_special_hit_boxes(&first_bone, &script_runner.special_hitboxes);
                    let animation_velocity = match (prev_offset, next_offset) {
                        (Some(prev_offset), Some(next_offset)) => Some(next_offset - prev_offset),
                        (Some(_),           None)              => unreachable!(),
                        (None,              Some(next_offset)) => Some(next_offset),
                        (None,              None)              => None
                    };
                    prev_offset = next_offset;

                    frames.push(HighLevelFrame {
                        hurt_boxes,
                        hit_boxes,
                        special_hit_boxes,
                        animation_velocity,
                        interruptible: script_runner.interruptible,
                        edge_slide:    script_runner.edge_slide.clone(),
                        airbourne:     script_runner.airbourne,
                    });

                    // TODO: Hitboxes
                    // Hitboxes are circle at the bone point (appear long because PM debug mode uses interpolation with the previous frames hitbox)
                    // Need to take hitbox from previous frame and interpolate into this frame
                    if iasa.is_none() && script_runner.interruptible {
                        iasa = Some(script_runner.frame_index)
                    }

                    script_runner.step(&scripts);
                }

                let action = HighLevelAction {
                    name: chr0.name.clone(),
                    iasa: iasa.unwrap_or_default() as usize,
                    frames,
                    animation_flags,
                    scripts,
                };
                actions.push(action);
            }
        }

        HighLevelFighter {
            name: fighter.cased_name.clone(),
            attributes: fighter_data.unwrap().attributes.clone(),
            actions,
            ledge_grabs: fighter_data.unwrap().misc.ledge_grabs.clone(),
        }
    }

    /// Modifies, in place, the matrices of the passed tree of bones, to follow that of the specified animation frame
    /// Returns the MOVES_CHARACTER offset if enabled.
    fn apply_chr0_to_bones(bone: &mut Bone, parent_transform: Matrix4<f32>, chr0: &Chr0, frame: i32, animation_flags: AnimationFlags) -> Option<Vector3<f32>> {
        let moves_character = animation_flags.contains(AnimationFlags::MOVES_CHARACTER);

        bone.transform = parent_transform;
        let mut offset = None;
        for chr0_child in &chr0.children {
            let transform = bone.transform * chr0_child.get_transform(chr0.loop_value, frame);
            if moves_character && bone.name == "TransN" {
                offset = Some(Vector3::new(transform.w.x, transform.w.y, transform.w.z));
                // TODO: Should this case modify bone.transform rot and scale?
            }
            else if chr0_child.name == bone.name {
                bone.transform = transform;
            }
        }

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
    pub hurt_boxes:         Vec<HighLevelHurtBox>,
    pub hit_boxes:          Vec<HighLevelHitBox>,
    pub special_hit_boxes:  Vec<HighLevelSpecialHitBox>,
    pub animation_velocity: Option<Vector3<f32>>,
    pub interruptible:      bool,
    pub edge_slide:         EdgeSlide,
    pub airbourne:          bool,
}

#[derive(Clone, Debug)]
pub struct HighLevelHurtBox {
    pub bone_matrix: Matrix4<f32>,
    pub hurt_box: HurtBox,
}

#[derive(Clone, Debug)]
pub struct HighLevelHitBox {
    pub position: Vector3<f32>,
    pub hit_box: HitBoxArguments,
}

#[derive(Clone, Debug)]
pub struct HighLevelSpecialHitBox {
    pub position: Vector3<f32>,
    pub hit_box: SpecialHitBoxArguments,
}

fn gen_hurt_boxes(bone: &Bone, hurt_boxes: &[HurtBox]) -> Vec<HighLevelHurtBox> {
    let mut hl_hurt_boxes = vec!();
    for hurt_box in hurt_boxes {
        if bone.index == hurt_box.bone_index as i32 {
            hl_hurt_boxes.push(HighLevelHurtBox {
                bone_matrix: bone.transform,
                hurt_box: hurt_box.clone(),
            });
            break;
        }
    }

    for child in bone.children.iter() {
        hl_hurt_boxes.extend(gen_hurt_boxes(child, hurt_boxes));
    }

    hl_hurt_boxes
}

fn gen_hit_boxes(bone: &Bone, hit_boxes: &[HitBoxArguments]) -> Vec<HighLevelHitBox> {
    let mut hl_hit_boxes = vec!();
    for hit_box in hit_boxes {
        if bone.index == hit_box.bone_index as i32 {
            hl_hit_boxes.push(HighLevelHitBox {
                position: Vector3::new(bone.transform.w.x, bone.transform.w.y, bone.transform.w.z),
                hit_box: hit_box.clone(),
            });
            break;
        }
    }

    for child in bone.children.iter() {
        hl_hit_boxes.extend(gen_hit_boxes(child, hit_boxes));
    }

    hl_hit_boxes
}

fn gen_special_hit_boxes(bone: &Bone, hit_boxes: &[SpecialHitBoxArguments]) -> Vec<HighLevelSpecialHitBox> {
    let mut hl_hit_boxes = vec!();
    for hit_box in hit_boxes {
        if bone.index == hit_box.hitbox_args.bone_index as i32 {
            hl_hit_boxes.push(HighLevelSpecialHitBox {
                position: Vector3::new(bone.transform.w.x, bone.transform.w.y, bone.transform.w.z),
                hit_box: hit_box.clone(),
            });
            break;
        }
    }

    for child in bone.children.iter() {
        hl_hit_boxes.extend(gen_special_hit_boxes(child, hit_boxes));
    }

    hl_hit_boxes
}
