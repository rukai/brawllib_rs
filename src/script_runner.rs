use crate::high_level_fighter::HitBoxValues;
use crate::script::Requirement;
use crate::script_ast::{
    ScriptAst,
    Block,
    EventAst,
    HitBoxArguments,
    SpecialHitBoxArguments,
    EdgeSlide,
    Expression,
    ComparisonOperator
};

use std::collections::HashMap;

pub struct ScriptRunner<'a> {
    pub call_stacks:          Vec<CallStack<'a>>,
    pub all_scripts:          &'a [&'a ScriptAst],
    pub variables:            HashMap<i32, i32>,
    pub visited_gotos:        Vec<u32>,
    pub frame_index:          f32,
    pub interruptible:        bool,
    pub hitboxes:             [Option<ScriptHitBox>; 7],
    pub frame_speed_modifier: f32,
    pub airbourne:            bool,
    pub edge_slide:           EdgeSlide, // TODO: This value seems inaccurate as its rarely set, is ledge cancel normally just hardcoded for say movement vs attack
    pub change_sub_action:    ChangeSubAction,
    pub hitlist_reset:        bool,
    pub slope_contour_stand:  Option<i32>,
    pub slope_contour_full:   Option<(i32, i32)>,
    pub rumble:               Option<(i32, i32)>,
    pub rumble_loop:          Option<(i32, i32)>,
}

pub struct CallStack<'a> {
    pub calls: Vec<Call<'a>>,
    pub wait_until: f32,
}

pub struct Call<'a> {
    pub block: &'a Block,
    pub index: usize,
    pub subroutine: bool,
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

impl<'a> ScriptRunner<'a> {
    pub fn new(action_scripts: &[&'a ScriptAst], all_scripts: &'a [&'a ScriptAst]) -> ScriptRunner<'a> {
        let mut call_stacks = vec!();
        for script in action_scripts {
            let calls = vec!(Call {
                block: &script.block,
                index: 0,
                subroutine: false
            });
            call_stacks.push(CallStack {
                calls,
                wait_until: -1.0
            });
        }

        ScriptRunner {
            call_stacks,
            all_scripts,
            variables:            HashMap::new(),
            visited_gotos:        vec!(),
            frame_index:          0.0,
            interruptible:        false,
            hitboxes:             [None, None, None, None, None, None, None],
            frame_speed_modifier: 1.0,
            airbourne:            false,
            edge_slide:           EdgeSlide::SlideOff,
            change_sub_action:    ChangeSubAction::Continue,
            hitlist_reset:        false,
            slope_contour_stand:  None,
            slope_contour_full:   None,
            rumble:               None,
            rumble_loop:          None,
        }
    }

    pub fn step(&mut self, action_name: &str) {
        self.hitlist_reset = false;
        self.rumble = None; // TODO: I guess rumble_loop shouldnt be reset?
        self.frame_index += self.frame_speed_modifier;
        self.visited_gotos.clear();

        // run the main, gfx, sfx and other scripts
        for i in 0..self.call_stacks.len() {
            while !self.call_stacks[i].calls.is_empty() { // reached the end of the script
                // Handle wait events
                if self.frame_index < self.call_stacks[i].wait_until {
                    break;
                }

                // Process the next event in the call_stack
                let call = self.call_stacks[i].calls.last().unwrap();
                if let Some(event) = call.block.events.get(call.index) {
                    self.call_stacks[i].calls.last_mut().unwrap().index += 1;
                    match self.step_event(event, self.all_scripts, action_name) {
                        StepEventResult::WaitUntil (value) => {
                            self.call_stacks[i].wait_until = value;
                        }
                        StepEventResult::NewCall (block) => {
                            self.call_stacks[i].calls.push(Call { block, index: 0, subroutine: false });
                        }
                        StepEventResult::Subroutine (block) => {
                            self.call_stacks[i].calls.push(Call { block, index: 0, subroutine: true });
                        }
                        StepEventResult::Return => {
                            let mut run = false;
                            while run {
                                run = self.call_stacks[i].calls.pop().map(|x| !x.subroutine).unwrap_or(false);
                            }
                        }
                        StepEventResult::Goto (block) => {
                            self.call_stacks[i].calls.pop();
                            self.call_stacks[i].calls.push(Call { block, index: 0, subroutine: false });
                        }
                        StepEventResult::None => { }
                    }
                } else {
                    self.call_stacks[i].calls.pop();
                }
            }
        }

        if self.frame_speed_modifier == 0.0 {
            self.change_sub_action = ChangeSubAction::InfiniteLoop
        }
    }

    fn evaluate_expression(&mut self, expression: &Expression) -> bool {
        match expression {
            &Expression::Nullary (ref requirement) => {
                match requirement {
                    Requirement::CharacterExists => true,
                    Requirement::OnGround => true,
                    Requirement::InAir => false,
                    Requirement::FacingRight => true,
                    Requirement::HasntTethered3Times => true,
                    Requirement::IsNotInDamagingLens => true,
                    _ => false
                }
            }
            &Expression::Unary (ref unary) => {
                match unary.requirement {
                    Requirement::CharacterExists => true,
                    Requirement::OnGround => true,
                    Requirement::InAir => false,
                    Requirement::FacingRight => true,
                    Requirement::HasntTethered3Times => true,
                    Requirement::IsNotInDamagingLens => true,
                    _ => false
                }
            }
            &Expression::Binary (ref binary) => {
                let left = match &*binary.left {
                    &Expression::Variable (ref address) => self.variables.get(address).cloned().unwrap_or(0) as f32, // TODO: Maybe this needs to be converted to be read as the same type as right, i.e. f32 or i32
                    &Expression::Value    (ref value)   => *value as f32,
                    &Expression::Scalar   (ref value)   => *value,
                    _                                   => 0.0
                };
                let right = match &*binary.right {
                    &Expression::Variable (ref address) => self.variables.get(address).cloned().unwrap_or(0) as f32,
                    &Expression::Value    (ref value)   => *value as f32,
                    &Expression::Scalar   (ref value)   => *value,
                    _                                   => 0.0
                };
                match &binary.operator {
                    &ComparisonOperator::LessThan           => left <  right,
                    &ComparisonOperator::LessThanOrEqual    => left <= right,
                    &ComparisonOperator::Equal              => left == right,
                    &ComparisonOperator::NotEqual           => left != right,
                    &ComparisonOperator::GreaterThanOrEqual => left >= right,
                    &ComparisonOperator::GreaterThan        => left >  right,
                    &ComparisonOperator::Or                 => self.evaluate_expression(&*binary.left) || self.evaluate_expression(&*binary.right),
                    &ComparisonOperator::And                => self.evaluate_expression(&*binary.left) && self.evaluate_expression(&*binary.right),
                    &ComparisonOperator::UnknownArg (_)     => false,
                }
            }
            &Expression::Not (ref expression) => {
                !self.evaluate_expression(expression)
            }
            &Expression::Variable (_) |
            &Expression::Value (_) |
            &Expression::Scalar (_) => {
                false
            }
        }
    }

    /// Returns the wait_until value
    fn step_event<'b>(&mut self, event: &'b EventAst, all_scripts: &[&'b ScriptAst], action_name: &str) -> StepEventResult<'b> {
        match event {
            &EventAst::SyncWait (ref value) => {
                return StepEventResult::WaitUntil (self.frame_index + *value);
            }
            &EventAst::AsyncWait (ref value) => {
                return StepEventResult::WaitUntil (*value);
            }
            &EventAst::SetLoop (_) => { }
            &EventAst::ExecuteLoop => { }
            &EventAst::Subroutine (offset) => {
                for script in all_scripts.iter() {
                    if script.offset == offset as u32 {
                        return StepEventResult::Subroutine (&script.block);
                    }
                }
                error!("Couldnt find Subroutine offset");
            }
            &EventAst::Return => {
                return StepEventResult::Return;
            }
            &EventAst::Goto (offset) => {
                if !self.visited_gotos.iter().any(|x| *x == offset as u32)  {
                    self.visited_gotos.push(offset as u32);
                    for script in all_scripts.iter() {
                        if script.offset == offset as u32 {
                            return StepEventResult::Goto (&script.block);
                        }
                    }
                    error!("Couldnt find Goto offset");
                }
                error!("Avoided Goto infinite loop");
            }
            &EventAst::IfStatement (ref if_statement) => {
                if self.evaluate_expression(&if_statement.test) {
                    return StepEventResult::NewCall (&if_statement.then_branch);
                }
                else {
                    if let Some(else_branch) = &if_statement.else_branch {
                        return StepEventResult::NewCall (else_branch);
                    }
                }
            }
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
                self.frame_speed_modifier = v0;
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
            &EventAst::Rumble { unk1, unk2 } => {
                self.rumble = Some((unk1, unk2))
            }
            &EventAst::RumbleLoop { unk1, unk2 } => {
                self.rumble_loop = Some((unk1, unk2))
            }
            &EventAst::SlopeContourStand { leg_bone_parent } => {
                if leg_bone_parent == 0 {
                    self.slope_contour_stand = None;
                }
                else {
                    self.slope_contour_stand = Some(leg_bone_parent);
                }
            }
            &EventAst::SlopeContourFull { hip_n_or_top_n, trans_bone } => {
                if hip_n_or_top_n == 0 && trans_bone == 0 {
                    self.slope_contour_full = None;
                }
                else {
                    self.slope_contour_full = Some((hip_n_or_top_n, trans_bone));
                }
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
            &EventAst::AestheticWindEffect (_) => { }
            &EventAst::ScreenShake { .. } => { }
            &EventAst::ModelChanger { .. } => { }
            &EventAst::Unknown (ref event) => {
                debug!("unknown event: {:#?}", event);
            }
            &EventAst::Nop => { }
        }
        StepEventResult::None
    }
}

enum StepEventResult<'a> {
    WaitUntil  (f32),
    NewCall    (&'a Block),
    Goto       (&'a Block),
    Subroutine (&'a Block),
    Return,
    None
}
