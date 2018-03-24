use cgmath::{Vector3, Matrix4, SquareMatrix, ElementWise};

use arc::ArcChildData;
use bres::BresChildData;
use chr0::Chr0;
use fighter::Fighter;
use math;
use mdl0::bones::Bone;
use misc_section::HurtBox;
use sakurai::{SectionData, FighterAttributes};

/// The HighLevelFighter stores processed Fighter data in a format that is easy to read from.
/// If brawllib_rs eventually implements the ability to modify character files via modifying Fighter and its children, then HighLevelFighter WILL NOT support that.
#[derive(Clone, Debug)]
pub struct HighLevelFighter {
    pub name: String,
    pub attributes: FighterAttributes,
    pub actions: Vec<HighLevelAction>,
}

impl HighLevelFighter {
    /// The processes data from an &Fighter and stores it in a HighLevelFighter
    pub fn new(fighter: &Fighter) -> HighLevelFighter {
        let mut hurt_boxes = None;
        let mut attributes = None;
        for sub_arc in &fighter.moveset.children {
            match &sub_arc.data {
                &ArcChildData::Sakurai (ref data) => {
                    for section in &data.sections {
                        if let &SectionData::FighterData { attributes: ref attributes_value, ref misc, .. } = &section.data {
                            hurt_boxes = Some(&misc.hurt_boxes);
                            attributes = Some(attributes_value.clone());
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
                if chr0.name == "AttackS4S" {
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

                let mut frames: Vec<HighLevelFrame> = vec!();
                for frame in 0..chr0.num_frames {
                    let mut first_bone = first_bone.clone();
                    HighLevelFighter::apply_chr0_to_bones(&mut first_bone, Matrix4::<f32>::identity(), chr0, frame as i32);
                    if let Some(hurt_boxes) = hurt_boxes {
                        let hurt_boxes = HighLevelHurtBox::gen_hurt_boxes(
                            &first_bone,
                            hurt_boxes,
                        );

                        frames.push(HighLevelFrame { hurt_boxes });
                    }
                    // TODO: Hitboxes
                    // Hitboxes are circle at the bone point (appear long because PM debug mode uses interpolation with the previous frames hitbox)
                    // Need to take hitbox from previous frame and interpolate into this frame
                }

                let action = HighLevelAction {
                    name: chr0.name.clone(),
                    iasa: 0,
                    frames
                };
                actions.push(action);
            }
        }

        HighLevelFighter {
            name: fighter.cased_name.clone(),
            attributes: attributes.unwrap(),
            actions,
        }
    }

    fn apply_chr0_to_bones(bone: &mut Bone, parent_transform: Matrix4<f32>, chr0: &Chr0, frame: i32) {
        bone.transform = parent_transform;
        for chr0_child in &chr0.children {
            if chr0_child.name == bone.name {
                bone.transform = bone.transform * chr0_child.get_transform(chr0.loop_value, frame);
            }
        }

        for child in bone.children.iter_mut() {
            HighLevelFighter::apply_chr0_to_bones(child, bone.transform, chr0, frame);
        }
    }
}

#[derive(Clone, Debug)]
pub struct HighLevelAction {
    pub name:   String,
    pub iasa:   usize,
    pub frames: Vec<HighLevelFrame>
}

#[derive(Clone, Debug)]
pub struct HighLevelFrame {
    pub hurt_boxes: Vec<HighLevelHurtBox>
}

#[derive(Clone, Debug)]
pub struct HighLevelHurtBox {
    pub start: Vector3<f32>,
    pub end:   Option<Vector3<f32>>,
    pub radius: f32,
}

impl HighLevelHurtBox {
    fn gen_hurt_boxes(
        bone: &Bone,
        hurt_boxes: &[HurtBox],
    ) -> Vec<HighLevelHurtBox> {
        let mut hl_hurt_boxes = vec!();

        for hurt_box in hurt_boxes {
            // create hurt_box
            if bone.index == hurt_box.bone_index as i32 {
                let transform = bone.transform * Matrix4::<f32>::from_translation(hurt_box.offset);
                let bones_cl = bone.scale * hurt_box.radius;
                let _matrix = math::gen_transform(bones_cl, bone.rot, bone.translate) * Matrix4::<f32>::from_translation(hurt_box.offset.div_element_wise(bones_cl));

                let end = if let Some(last_child_bone) = bone.children.get(bone.children.len()-1) { // TODO: This is weird but seems to work, maybe I need to do it for all children instead? Maybe there is a value that says which child to use.
                    let child_transform = last_child_bone.transform * Matrix4::<f32>::from_translation(hurt_box.offset);
                    Some(Vector3::new(child_transform.w.z, child_transform.w.y, child_transform.w.x))
                } else {
                    None
                };

                // TODO: properly handle stretch
                hl_hurt_boxes.push(HighLevelHurtBox {
                    radius: hurt_box.radius,
                    start: Vector3::new(transform.w.z, transform.w.y, transform.w.x),
                    end,
                });

                break;
            }
        }

        for child in bone.children.iter() {
            hl_hurt_boxes.extend(HighLevelHurtBox::gen_hurt_boxes(child, hurt_boxes));
        }

        hl_hurt_boxes
    }
}
