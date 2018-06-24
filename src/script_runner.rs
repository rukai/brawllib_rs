use std::collections::HashMap;
use script_ast::{Block, EventAst, HitBoxArguments, SpecialHitBoxArguments, EdgeSlide, Expression};
use high_level_fighter::{HighLevelScripts, HitBoxValues};

pub struct ScriptRunner {
    pub variables: HashMap<i32, i32>,
    pub callstack: Vec<(Block, usize)>,
    pub frame_index: f32,
    pub wait_until: Option<f32>,
    pub interruptible: bool,
    pub hitboxes: [Option<ScriptHitBox>; 7],
    pub frame_speed_modifier: f32,
    pub airbourne: bool,
    pub edge_slide: EdgeSlide, // TODO: This value seems inaccurate as its rarely set, is ledge cancel normally just hardcoded for say movement vs attack
    pub change_sub_action: ChangeSubAction,
    pub hitlist_reset: bool,
}

pub enum ChangeSubAction {
    Continue,
    InfiniteLoop,
    ChangeSubAction (i32),
    ChangeSubActionRestartFrame (i32),
}

#[derive(Clone, Debug)]
pub struct ScriptHitBox {
    pub bone_index: i16,
    pub x_offset:   f32,
    pub y_offset:   f32,
    pub z_offset:   f32,
    pub values:     HitBoxValues
}

impl ScriptHitBox {
    fn from_hitbox(args: &HitBoxArguments) -> ScriptHitBox {
        ScriptHitBox {
            bone_index: args.bone_index,
            x_offset:   args.x_offset,
            y_offset:   args.y_offset,
            z_offset:   args.z_offset,
            values:     HitBoxValues::from_hitbox(args)
        }
    }

    fn from_special_hitbox(args: &SpecialHitBoxArguments) -> ScriptHitBox {
        ScriptHitBox {
            bone_index: args.hitbox_args.bone_index,
            x_offset:   args.hitbox_args.x_offset,
            y_offset:   args.hitbox_args.y_offset,
            z_offset:   args.hitbox_args.z_offset,
            values:     HitBoxValues::from_special_hitbox(args)
        }
    }
}

impl ScriptRunner {
    pub fn new() -> ScriptRunner {
        ScriptRunner {
            variables: HashMap::new(),
            event_indices: vec!(0),
            frame_index: 0.0,
            wait_until: None,
            interruptible: false,
            hitboxes: [None, None, None, None, None, None, None],
            frame_speed_modifier: 1.0,
            airbourne: false,
            edge_slide: EdgeSlide::SlideOff,
            change_sub_action: ChangeSubAction::Continue,
            hitlist_reset: false,
        }
    }

    pub fn step(&mut self, scripts: &Option<HighLevelScripts>, action_name: &str) {
        self.frame_index += self.frame_speed_modifier;

        if let Some(wait_until) = self.wait_until {
            if self.frame_index >= wait_until
                self.wait_until = None;
            }
        }

        if self.wait_until.is_none() {
            if let &Some(ref scripts) = scripts {
                self.step_block(&scripts.script_main, 0, action_name);
            }
        }

        if self.frame_speed_modifier == 0.0 {
            self.change_sub_action = ChangeSubAction::InfiniteLoop
        }
    }

    fn evaluate_expression(&mut self, expression: &Expression) -> bool {
        info!("{:?}", expression);
        false
    }

    fn step_block(&mut self, block: &Block, continue_index: usize, action_name: &str) {
        self.hitlist_reset = false;
        //let event_index = self.event_indices.last_mut().unwrap(); // TODO: DELETE
        for event in block.events[continue_index..] {
            match event {
                &EventAst::SyncWait (ref value) => {
                    self.wait_until = Some(self.frame_index + *value);
                    return;
                }
                &EventAst::AsyncWait (ref value) => {
                    self.wait_until = Some(*value);
                    return;
                }
                &EventAst::SetLoop (_) => { }
                &EventAst::ExecuteLoop => { }
                &EventAst::Subroutine (_) => { }
                &EventAst::Return => { }
                &EventAst::Goto (_) => { }
                &EventAst::IfStatement (ref if_statement) => {
                    if self.evaluate_expression(&if_statement.test) {
                        self.step_block(&if_statement.then_branch, 0, action_name);
                    }
                    else {
                        if let Some(else_branch) = &if_statement.else_branch {
                            self.step_block(else_branch, 0, action_name);
                        }
                    }
                }
                &EventAst::IfValue (_, _) => { }
                &EventAst::IfComparison (_, _, _, _) => { }
                &EventAst::Else => { }
                &EventAst::AndComparison (_, _, _, _) => { }
                &EventAst::ElseIfComparison (_, _, _, _) => { }
                &EventAst::Switch (_, _) => { }
                &EventAst::EndSwitch => { }
                &EventAst::ChangeSubAction (v0) => {
                    self.change_sub_action = ChangeSubAction::ChangeSubAction (v0);
                }
                &EventAst::ChangeSubActionRestartFrame (v0) => {
                    self.change_sub_action = ChangeSubAction::ChangeSubActionRestartFrame (v0);
                }
                &EventAst::SetFrame (v0) => {
                    self.frame_index = v0;
                }
                &EventAst::FrameSpeedModifier (v0) => {
                    if action_name != "LandingFallSpecial" { // TODO: Hack because scripts are setting this to all sorts of weird values in this action
                        self.frame_speed_modifier = v0;
                    }
                }
                &EventAst::TimeManipulation (_, _) => { }
                &EventAst::SetAirGround (v0) => {
                    self.airbourne = v0 == 0; // TODO: Seems like brawlbox is incomplete here e.g 36
                }
                &EventAst::SetEdgeSlide (ref v0) => {
                    self.edge_slide = v0.clone();
                }
                &EventAst::ReverseDirection => { }
                &EventAst::CreateHitBox (ref args) => {
                    if args.hitbox_index < self.hitboxes.len() as u8 {
                        if let Some(ref prev_hitbox) = self.hitboxes[args.hitbox_index as usize] {
                            if args.rehit_hitbox_index > prev_hitbox.values.rehit_hitbox_index {
                                self.hitlist_reset = true;
                            }
                        }
                        self.hitboxes[args.hitbox_index as usize] = Some(ScriptHitBox::from_hitbox(args));
                    } else {
                        error!("invalid hitbox index {} {}", args.hitbox_index, action_name);
                    }
                }
                &EventAst::CreateSpecialHitBox (ref args) => {
                    let index = args.hitbox_args.hitbox_index as usize;
                    if args.hitbox_args.hitbox_index < self.hitboxes.len() as u8 {
                        if let Some(ref prev_hitbox) = self.hitboxes[index] {
                            if args.hitbox_args.rehit_hitbox_index > prev_hitbox.values.rehit_hitbox_index {
                                self.hitlist_reset = true;
                            }
                        }
                        self.hitboxes[index] = Some(ScriptHitBox::from_special_hitbox(args));
                    } else {
                        error!("invalid hitbox index {} {}", args.hitbox_args.hitbox_index, action_name);
                    }
                }
                &EventAst::RemoveAllHitBoxes => {
                    for hitbox in self.hitboxes.iter_mut() {
                        *hitbox = None;
                    }
                    self.hitlist_reset = true;
                }
                &EventAst::MoveHitBox (ref move_hitbox) => {
                    if let Some(ref mut hitbox) = self.hitboxes[move_hitbox.hitbox_id as usize] {
                        hitbox.bone_index = move_hitbox.new_bone as i16;
                        hitbox.x_offset = move_hitbox.new_x_offset;
                        hitbox.y_offset = move_hitbox.new_y_offset;
                        hitbox.z_offset = move_hitbox.new_z_offset;
                    }
                }
                &EventAst::ChangeHitBoxDamage { hitbox_id, new_damage } => {
                    if let Some(ref mut hitbox) = self.hitboxes[hitbox_id as usize] {
                        hitbox.values.damage = new_damage;
                    }
                }
                &EventAst::ChangeHitBoxSize { hitbox_id, new_size } => {
                    if let Some(ref mut hitbox) = self.hitboxes[hitbox_id as usize] {
                        hitbox.values.size = new_size as f32;
                    }
                }
                &EventAst::DeleteHitBox (id) => {
                    self.hitboxes[id as usize] = None;
                }
                &EventAst::AllowInterrupt => {
                    self.interruptible = true;
                }
                &EventAst::SoundEffect1 (_) => { }
                &EventAst::SoundEffect2 (_) => { }
                &EventAst::SoundEffectTransient (_) => { }
                &EventAst::SoundEffectStop (_) => { }
                &EventAst::SoundEffectVictory (_) => { }
                &EventAst::SoundEffectUnk (_) => { }
                &EventAst::SoundEffectOther1 (_) => { }
                &EventAst::SoundEffectOther2 (_) => { }
                &EventAst::SoundVoiceLow => { }
                &EventAst::SoundVoiceDamage => { }
                &EventAst::SoundVoiceOttotto => { }
                &EventAst::SoundVoiceEating => { }
                &EventAst::GraphicEffect (_) => { }
                &EventAst::Unknown (ref event) => {
                    debug!("unknown event: {:#?}", event);
                }
                &EventAst::Nop => { }
            }
        }
    }
}
