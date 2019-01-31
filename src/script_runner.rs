use crate::high_level_fighter::CollisionBoxValues;
use crate::script::Requirement;
use crate::script_ast::{
    ScriptAst,
    Block,
    EventAst,
    Iterations,
    HitBoxArguments,
    SpecialHitBoxArguments,
    GrabBoxArguments,
    HurtBoxState,
    LedgeGrabEnable,
    ArmorType,
    DisableMovement,
    EdgeSlide,
    Expression,
    ComparisonOperator,
};

use std::collections::HashMap;

pub struct ScriptRunner<'a> {
    pub call_stacks:          Vec<CallStack<'a>>,
    pub all_scripts:          &'a [&'a ScriptAst],
    pub variables:            HashMap<i32, i32>,
    pub visited_gotos:        Vec<u32>,
    pub frame_index:          f32,
    pub interruptible:        bool,
    pub hitboxes:             [Option<ScriptCollisionBox>; 7],
    pub hurtbox_state_all:    HurtBoxState,
    pub hurtbox_states:       HashMap<i32, HurtBoxState>,
    pub ledge_grab_enable:    LedgeGrabEnable,
    pub frame_speed_modifier: f32,
    pub tag_display:          bool,
    pub x:                    f32,
    pub y:                    f32,
    pub x_vel:                f32,
    pub y_vel:                f32,
    pub disable_movement:     DisableMovement,
    pub armor_type:           ArmorType,
    pub armor_tolerance:      f32,
    pub damage:               f32,
    pub airbourne:            bool,
    pub edge_slide:           EdgeSlide, // TODO: This value seems inaccurate as its rarely set, is ledge cancel normally just hardcoded for say movement vs attack
    pub change_subaction:     ChangeSubaction,
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

pub enum ChangeSubaction {
    Continue,
    InfiniteLoop,
    ChangeSubaction (i32),
    ChangeSubactionRestartFrame (i32),
}

#[derive(Clone, Debug)]
pub struct ScriptCollisionBox {
    pub bone_index:   i16,
    pub hitbox_index: u8,
    pub x_offset:     f32,
    pub y_offset:     f32,
    pub z_offset:     f32,
    pub size:         f32,
    pub values:       CollisionBoxValues
}

impl ScriptCollisionBox {
    fn from_hitbox(args: &HitBoxArguments) -> ScriptCollisionBox {
        ScriptCollisionBox {
            bone_index:   args.bone_index,
            hitbox_index: args.hitbox_index,
            x_offset:     args.x_offset,
            y_offset:     args.y_offset,
            z_offset:     args.z_offset,
            size:         args.size,
            values:       CollisionBoxValues::from_hitbox(args)
        }
    }

    fn from_special_hitbox(args: &SpecialHitBoxArguments) -> ScriptCollisionBox {
        ScriptCollisionBox {
            bone_index:   args.hitbox_args.bone_index,
            hitbox_index: args.hitbox_args.hitbox_index,
            x_offset:     args.hitbox_args.x_offset,
            y_offset:     args.hitbox_args.y_offset,
            z_offset:     args.hitbox_args.z_offset,
            size:         args.hitbox_args.size,
            values:       CollisionBoxValues::from_special_hitbox(args)
        }
    }

    fn from_grabbox(args: &GrabBoxArguments) -> ScriptCollisionBox {
        ScriptCollisionBox {
            bone_index:   args.bone_index as i16,
            hitbox_index: args.hitbox_index as u8,
            x_offset:     args.x_offset,
            y_offset:     args.y_offset,
            z_offset:     args.z_offset,
            size:         args.size,
            values:       CollisionBoxValues::from_grabbox(args)
        }
    }

    fn is_hit(&self) -> bool {
        match self.values {
            CollisionBoxValues::Hit(_) => true,
            CollisionBoxValues::Grab(_) => false,
        }
    }

    fn is_grab(&self) -> bool {
        match self.values {
            CollisionBoxValues::Hit(_) => false,
            CollisionBoxValues::Grab(_) => true,
        }
    }
}

impl<'a> ScriptRunner<'a> {
    /// Runs the action main, gfx, sfx and other scripts in subaction_scripts.
    /// all_scripts contains any functions that the action scripts need to call into.
    /// The returned runner has completed the first frame.
    /// Calling `runner.step` will advance to frame 2 and then frame 3 and so on.
    pub fn new(subaction_scripts: &[&'a ScriptAst], all_scripts: &'a [&'a ScriptAst]) -> ScriptRunner<'a> {
        let mut call_stacks = vec!();
        for script in subaction_scripts {
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

        let mut runner = ScriptRunner {
            call_stacks,
            all_scripts,
            variables:            HashMap::new(),
            visited_gotos:        vec!(),
            frame_index:          0.0,
            interruptible:        false,
            hitboxes:             [None, None, None, None, None, None, None],
            hurtbox_state_all:    HurtBoxState::Normal,
            hurtbox_states:       HashMap::new(),
            ledge_grab_enable:    LedgeGrabEnable::Disable,
            frame_speed_modifier: 1.0,
            tag_display:          true,
            x:                    0.0,
            y:                    0.0,
            x_vel:                0.0,
            y_vel:                0.0,
            disable_movement:     DisableMovement::Enable,
            armor_type:           ArmorType::None,
            armor_tolerance:      0.0,
            damage:               0.0,
            airbourne:            false,
            edge_slide:           EdgeSlide::SlideOff,
            change_subaction:     ChangeSubaction::Continue,
            hitlist_reset:        false,
            slope_contour_stand:  None,
            slope_contour_full:   None,
            rumble:               None,
            rumble_loop:          None,
        };

        // Need to run the script until the first wait, so that the script is in the valid state
        // for the first frame.
        runner.step_script("ScriptRunner init");

        runner
    }

    /// Steps the main, gfx, sfx and other scripts by 1 game frame.
    /// `action_name` can be anything, it is just used for debugging.
    pub fn step(&mut self, action_name: &str) {
        self.frame_index += self.frame_speed_modifier;
        self.step_script(action_name);
    }

    fn step_script(&mut self, action_name: &str) {
        self.hitlist_reset = false;
        self.rumble = None; // TODO: I guess rumble_loop shouldnt be reset?
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
                        StepEventResult::NewForLoop { block, iterations } => {
                            for _ in 0..iterations {
                                self.call_stacks[i].calls.push(Call { block, index: 0, subroutine: false });
                            }
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
            self.change_subaction = ChangeSubaction::InfiniteLoop
        }

        match self.disable_movement {
            DisableMovement::Enable => {
                self.x += self.x_vel;
                self.y += self.y_vel;
            }
            DisableMovement::DisableVertical => {
                self.x += self.x_vel;
            }
            DisableMovement::DisableHorizontal => {
                self.y += self.y_vel;
            }
            _ => error!("Unknown DisableMovement value"),
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
            &EventAst::ForLoop (ref for_loop) => {
                match for_loop.iterations {
                    Iterations::Finite (iterations) => {
                        return StepEventResult::NewForLoop { block: &for_loop.block, iterations };
                    }
                    Iterations::Infinite => {
                        // obviously an infinite loop should not be attempted :P
                        return StepEventResult::NewCall (&for_loop.block);
                    }
                }
            }
            &EventAst::Subroutine (offset) => {
                // TODO: Maybe I should implement a protection similar to visited_gotos for subroutines.
                // If that turns out to be a bad idea document why.
                for script in all_scripts.iter() {
                    if script.offset == offset as u32 {
                        if script.block.events.len() > 0 && &script.block.events[0] as *const _ == event as *const _ {
                            error!("Avoided hard Subroutine infinite loop (attempted to jump to the same location)");
                        }
                        else {
                            return StepEventResult::Subroutine (&script.block);
                        }
                    }
                }
                error!("Couldnt find Subroutine offset");
            }
            &EventAst::Return => {
                return StepEventResult::Return;
            }
            &EventAst::Goto (offset) => {
                if !self.visited_gotos.iter().any(|x| *x == offset as u32) {
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
            &EventAst::Switch (_, _) => { } // TODO
            &EventAst::EndSwitch => { }
            &EventAst::Case (_) => { }
            &EventAst::DefaultCase => { }
            &EventAst::LoopRest => { error!("LoopRest: This means the code is expected to infinite loop") } // TODO: Handle infinite loops better
            &EventAst::EnableActionStatusID (_) => { } // TODO
            &EventAst::ChangeActionStatus { .. } => { } // TODO
            &EventAst::ChangeAction { .. } => { } // TODO
            &EventAst::AllowInterrupt => {
                self.interruptible = true;
            }
            &EventAst::ChangeSubaction (v0) => {
                self.change_subaction = ChangeSubaction::ChangeSubaction (v0);
            }
            &EventAst::ChangeSubactionRestartFrame (v0) => {
                self.change_subaction = ChangeSubaction::ChangeSubactionRestartFrame (v0);
            }

            // timing
            &EventAst::SetFrame (v0) => {
                self.frame_index = v0;
            }
            &EventAst::FrameSpeedModifier (v0) => {
                self.frame_speed_modifier = v0;
            }
            &EventAst::TimeManipulation (_, _) => { }

            // misc state
            &EventAst::SetAirGround (v0) => {
                self.airbourne = v0 == 0; // TODO: Seems like brawlbox is incomplete here e.g 36
            }
            &EventAst::SetEdgeSlide (ref v0) => {
                self.edge_slide = v0.clone();
            }
            &EventAst::ReverseDirection => { }

            // hitboxes
            &EventAst::CreateHitBox (ref args) => {
                if args.hitbox_index < self.hitboxes.len() as u8 {
                    if let Some(ref prev_hitbox) = self.hitboxes[args.hitbox_index as usize] {
                        if let CollisionBoxValues::Hit (ref prev_hitbox) = prev_hitbox.values {
                            if args.rehit_hitbox_index > prev_hitbox.rehit_hitbox_index {
                                self.hitlist_reset = true;
                            }
                        }
                    }
                    self.hitboxes[args.hitbox_index as usize] = Some(ScriptCollisionBox::from_hitbox(args));
                } else {
                    error!("invalid hitbox index {} {}", args.hitbox_index, action_name);
                }
            }
            &EventAst::CreateSpecialHitBox (ref args) => {
                let index = args.hitbox_args.hitbox_index as usize;
                if args.hitbox_args.hitbox_index < self.hitboxes.len() as u8 {
                    if let Some(ref prev_hitbox) = self.hitboxes[index] {
                        if let CollisionBoxValues::Hit (ref prev_hitbox) = prev_hitbox.values {
                            if args.hitbox_args.rehit_hitbox_index > prev_hitbox.rehit_hitbox_index {
                                self.hitlist_reset = true;
                            }
                        }
                    }
                    self.hitboxes[index] = Some(ScriptCollisionBox::from_special_hitbox(args));
                } else {
                    error!("invalid hitbox index {} {}", args.hitbox_args.hitbox_index, action_name);
                }
            }
            &EventAst::DeleteAllHitBoxes => {
                for hitbox in self.hitboxes.iter_mut() {
                    if hitbox.as_ref().map(|x| x.is_hit()).unwrap_or(false) {
                        *hitbox = None;
                    }
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
                if let Some(ref mut hitbox) = &mut self.hitboxes[hitbox_id as usize] {
                    if let CollisionBoxValues::Hit (ref mut hitbox) = hitbox.values {
                        hitbox.damage = new_damage;
                    }
                }
            }
            &EventAst::ChangeHitBoxSize { hitbox_id, new_size } => {
                if let Some(ref mut hitbox) = &mut self.hitboxes[hitbox_id as usize] {
                    if let CollisionBoxValues::Hit (ref mut hitbox) = hitbox.values {
                        hitbox.size = new_size as f32;
                    }
                }
            }
            &EventAst::DeleteHitBox (hitbox_index) => {
                if self.hitboxes[hitbox_index as usize].as_ref().map(|x| x.is_hit()).unwrap_or(false) {
                    self.hitboxes[hitbox_index as usize] = None;
                }
            }
            &EventAst::CreateGrabBox (ref args) => {
                if args.hitbox_index < self.hitboxes.len() as i32 {
                    self.hitboxes[args.hitbox_index as usize] = Some(ScriptCollisionBox::from_grabbox(args));
                } else {
                    error!("invalid hitbox index {} {}", args.hitbox_index, action_name);
                }
            }
            &EventAst::DeleteGrabBox (hitbox_index) => {
                if self.hitboxes[hitbox_index as usize].as_ref().map(|x| x.is_grab()).unwrap_or(false) {
                    self.hitboxes[hitbox_index as usize] = None;
                }
            }
            &EventAst::DeleteAllGrabBoxes => {
                for hitbox in self.hitboxes.iter_mut() {
                    if hitbox.as_ref().map(|x| x.is_grab()).unwrap_or(false) {
                        *hitbox = None;
                    }
                }
                self.hitlist_reset = true;
            }

            // hurtboxes
            &EventAst::ChangeHurtBoxStateAll { ref state } => {
                self.hurtbox_state_all = state.clone();
            }
            &EventAst::ChangeHurtBoxStateSpecific { bone, ref state } => {
                self.hurtbox_states.insert(bone, state.clone());
            }
            &EventAst::UnchangeHurtBoxStateSpecific => {
                self.hurtbox_states.clear();
            }

            // misc
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
            &EventAst::GenerateArticle { .. } => { }
            &EventAst::ArticleEvent (_) => { }
            &EventAst::ArticleAnimation (_) => { }
            &EventAst::ArticleRemove (_) => { }
            &EventAst::ArticleVisibility { .. } => { }
            &EventAst::FinalSmashEnter => { }
            &EventAst::FinalSmashExit => { }
            &EventAst::TerminateSelf => { }
            &EventAst::Posture (_) => { }
            &EventAst::LedgeGrabEnable (ref enable) => {
                self.ledge_grab_enable = enable.clone();
            }
            &EventAst::TagDisplay (display) => {
                self.tag_display = display;
            }
            &EventAst::Armor { ref armor_type, tolerance } => {
                self.armor_type = armor_type.clone();
                self.armor_tolerance = tolerance;
            }
            &EventAst::AddDamage (damage) => {
                self.damage += damage;
            }
            &EventAst::SetOrAddVelocity (ref values) => {
                if values.x_set {
                    self.x_vel = values.x_vel
                }
                else {
                    self.x_vel += values.x_vel
                }

                if values.y_set {
                    self.y_vel = values.y_vel
                }
                else {
                    self.y_vel += values.y_vel
                }
            }
            &EventAst::SetVelocity { x_vel, y_vel } => {
                self.x_vel = x_vel;
                self.y_vel = y_vel;
            }
            &EventAst::AddVelocity { x_vel, y_vel } => {
                self.x_vel += x_vel;
                self.y_vel += y_vel;
            }
            &EventAst::DisableMovement (ref disable_movement) => {
                self.disable_movement = disable_movement.clone();
            }
            &EventAst::DisableMovement2 (_) => { } // TODO: What!?!?
            &EventAst::ResetVerticalVelocityAndAcceleration (reset) => {
                if reset {
                    self.y_vel = 0.0;
                }
            }

            // sound
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

            // variables
            &EventAst::IntVariableSet { value, variable } => {
                self.variables.insert(variable, value);
            }
            &EventAst::IntVariableAdd { value, variable } => {
                let old_value = self.variables.get(&variable).cloned().unwrap_or(0);
                self.variables.insert(variable, old_value + value);
            }
            &EventAst::IntVariableSubtract { value, variable } => {
                let old_value = self.variables.get(&variable).cloned().unwrap_or(0);
                self.variables.insert(variable, old_value - value);
            }
            &EventAst::IntVariableIncrement { variable } => {
                let old_value = self.variables.get(&variable).cloned().unwrap_or(0);
                self.variables.insert(variable, old_value + 1);
            }
            &EventAst::IntVariableDecrement { variable } => {
                let old_value = self.variables.get(&variable).cloned().unwrap_or(0);
                self.variables.insert(variable, old_value - 1);
            }
            &EventAst::FloatVariableSet { value, variable } => {
                self.variables.insert(variable, value as i32); // TODO: Should these be cast bitwise? Or an enum VariableType { Int(i32), Float(f32) } ?
            }
            &EventAst::FloatVariableAdd { value, variable } => {
                let old_value = self.variables.get(&variable).cloned().map(|x| x as f32).unwrap_or(0.0);
                self.variables.insert(variable, (old_value + value) as i32);
            }
            &EventAst::FloatVariableSubtract { value, variable } => {
                let old_value = self.variables.get(&variable).cloned().map(|x| x as f32).unwrap_or(0.0);
                self.variables.insert(variable, (old_value - value) as i32);
            }
            &EventAst::FloatVariableMultiply { value, variable } => {
                let old_value = self.variables.get(&variable).cloned().map(|x| x as f32).unwrap_or(0.0);
                self.variables.insert(variable, (old_value * value) as i32);
            }
            &EventAst::FloatVariableDivide { value, variable } => {
                let old_value = self.variables.get(&variable).cloned().map(|x| x as f32).unwrap_or(0.0);
                self.variables.insert(variable, (old_value / value) as i32);
            }
            &EventAst::BoolVariableSetTrue { variable } => {
                self.variables.insert(variable, 1);
            }
            &EventAst::BoolVariableSetFalse { variable } => {
                self.variables.insert(variable, 0);
            }

            // graphics
            &EventAst::GraphicEffect (_) => { }
            &EventAst::ExternalGraphicEffect (_) => { }
            &EventAst::LimitedScreenTint (_) => { }
            &EventAst::UnlimitedScreenTint (_) => { }
            &EventAst::EndUnlimitedScreenTint { .. } => { }
            &EventAst::SwordGlow (_) => { }
            &EventAst::DeleteSwordGlow { .. } => { }
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
    NewForLoop { block: &'a Block, iterations: i32 },
    NewCall    (&'a Block),
    Goto       (&'a Block),
    Subroutine (&'a Block),
    Return,
    None
}
