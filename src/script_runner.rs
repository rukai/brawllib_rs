use crate::high_level_fighter::CollisionBoxValues;
use crate::high_level_fighter;
use crate::script::{Requirement, VariableDataType};
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
    ChangeAction,
};
use crate::script_ast::variable_ast::{
    VariableAst,
    InternalConstantInt,
    LongtermAccessInt, LongtermAccessFloat, LongtermAccessBool,
    RandomAccessInt,   RandomAccessFloat,   RandomAccessBool,
};

use std::collections::HashMap;

pub struct ScriptRunner<'a> {
    pub call_stacks:          Vec<CallStack<'a>>,
    pub all_scripts:          &'a [&'a ScriptAst],
    pub call_every_frame:     HashMap<i32, &'a Block>,
    pub visited_gotos:        Vec<u32>,
    pub frame_index:          f32,
    pub interruptible:        bool,
    pub hitboxes:             [Option<ScriptCollisionBox>; 7],
    pub hurtbox_state_all:    HurtBoxState,
    pub hurtbox_states:       HashMap<i32, HurtBoxState>,
    pub ledge_grab_enable:    LedgeGrabEnable,
    pub frame_speed_modifier: f32,
    pub tag_display:          bool,
    /// State is maintained across frames
    pub x:                    f32,
    /// State is maintained across frames
    pub y:                    f32,
    /// State is maintained across frames
    pub x_vel:                f32,
    /// State is maintained across frames
    pub y_vel:                f32,
    /// Reset to None before processing each frame
    pub x_vel_modify:         VelModify,
    /// Reset to None before processing each frame
    pub y_vel_modify:         VelModify,
    pub disable_movement:     DisableMovement,
    pub armor_type:           ArmorType,
    pub armor_tolerance:      f32,
    pub damage:               f32,
    pub airbourne:            bool,
    pub edge_slide:           EdgeSlide, // TODO: This value seems inaccurate as its rarely set, is ledge cancel normally just hardcoded for say movement vs attack
    pub change_subaction:     ChangeSubaction,
    /// These are rechecked every frame after being created
    pub change_actions:       Vec<ChangeAction>,
    pub hitbox_sets_rehit:    [bool; 10],
    pub slope_contour_stand:  Option<i32>,
    pub slope_contour_full:   Option<(i32, i32)>,
    pub rumble:               Option<(i32, i32)>,
    pub rumble_loop:          Option<(i32, i32)>,

    // LongtermAccessInt
    pub jumps_used: i32,
    pub wall_jump_count: i32,
    pub wall_jump_interval: i32,
    pub footstool_count: i32,
    pub fall_time: i32,
    pub swim_time: i32,
    pub lip_stick_refresh: i32,
    pub curry_remaining_time: i32,
    pub curry_angle2: i32,
    pub star_remaining_time: i32,
    pub mushroom_remaining_time: i32,
    pub lightning_remaining_time: i32,
    pub size_flag: i32,
    pub metal_block_remaining_time: i32,
    pub combo_count: i32,
    pub bubble_time: i32,
    pub attacks_performed: i32,
    pub costume_id: i32,
    pub hitstun_frames_remaining: i32,
    pub meteor_cancel_window: i32,
    pub missed_techs: i32,
    pub tether_count: i32,
    pub temp1: i32,
    pub temp2: i32,

    // LongtermAccessFloat
    pub special_landing_lag: f32,
    pub special_fall_mobility_multiplier: f32,
    pub shield_charge: f32,
    pub curry_angle1: f32,
    pub curry_randomness: f32,

    // LongtermAccessBool
    pub is_dead: bool,
    pub cannot_die: bool,
    pub automatic_footstool: bool,
    pub has_final: bool,
    pub has_final_aura: bool,
    pub has_curry: bool,
    pub has_hammer: bool,
    pub hit_by_paralyze: bool,
    pub has_screw_attack: bool,
    pub stamina_dead: bool,
    pub has_tag: bool,
    pub can_not_ledge_grab: bool,
    pub can_not_teeter: bool,
    pub velocity_ignore_hitstun: bool,
    pub deflection: bool,

    // RandomAccessInt
    pub throw_data_param1: i32,
    pub throw_data_param2: i32,
    pub throw_data_param3: i32,

    // RandomAccessFloat
    pub enable_turn_when_below_zero: f32,

    // RandomAccessBool
    pub character_float: bool,
    pub enable_fast_fall: bool,
    pub shorthop: bool,
    pub enable_action_transition: bool,
    pub specials_movement: bool,
    pub enable_glide: bool,
    pub enable_jab_loop: bool,
    pub enable_auto_jab: bool,
    pub enable_jab_end: bool,
    pub landing_lag: bool,
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
    // TODO: Because we currently only operate at the subaction level, this is the best we can do.
    ChangeAction (i32),
}

#[derive(Clone, Debug)]
pub struct ScriptCollisionBox {
    pub bone_index:  i16,
    pub hitbox_id:   u8,
    pub x_offset:    f32,
    pub y_offset:    f32,
    pub z_offset:    f32,
    pub size:        f32,
    pub values:      CollisionBoxValues,
    pub interpolate: bool,
}

impl ScriptCollisionBox {
    fn from_hitbox(args: &HitBoxArguments, interpolate: bool) -> ScriptCollisionBox {
        ScriptCollisionBox {
            bone_index:  args.bone_index,
            hitbox_id:   args.hitbox_id,
            x_offset:    args.x_offset,
            y_offset:    args.y_offset,
            z_offset:    args.z_offset,
            size:        args.size,
            values:      CollisionBoxValues::from_hitbox(args),
            interpolate,
        }
    }

    fn from_special_hitbox(args: &SpecialHitBoxArguments, interpolate: bool) -> ScriptCollisionBox {
        ScriptCollisionBox {
            bone_index: args.hitbox_args.bone_index,
            hitbox_id:  args.hitbox_args.hitbox_id,
            x_offset:   args.hitbox_args.x_offset,
            y_offset:   args.hitbox_args.y_offset,
            z_offset:   args.hitbox_args.z_offset,
            size:       args.hitbox_args.size,
            values:     CollisionBoxValues::from_special_hitbox(args),
            interpolate,
        }
    }

    fn from_grabbox(args: &GrabBoxArguments, interpolate: bool) -> ScriptCollisionBox {
        ScriptCollisionBox {
            bone_index: args.bone_index as i16,
            hitbox_id:  args.hitbox_id as u8,
            x_offset:   args.x_offset,
            y_offset:   args.y_offset,
            z_offset:   args.z_offset,
            size:       args.size,
            values:     CollisionBoxValues::from_grabbox(args),
            interpolate,
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
            call_every_frame:     HashMap::new(),
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
            x_vel_modify:         VelModify::None,
            y_vel_modify:         VelModify::None,
            disable_movement:     DisableMovement::Enable,
            armor_type:           ArmorType::None,
            armor_tolerance:      0.0,
            damage:               0.0,
            airbourne:            false,
            edge_slide:           EdgeSlide::SlideOff,
            change_subaction:     ChangeSubaction::Continue,
            change_actions:       vec!(),
            hitbox_sets_rehit:    [false; 10],
            slope_contour_stand:  None,
            slope_contour_full:   None,
            rumble:               None,
            rumble_loop:          None,

            // LongtermAccessInt
            jumps_used: 0,
            wall_jump_count: 0,
            wall_jump_interval: 0,
            footstool_count: 0,
            fall_time: 0,
            swim_time: 0,
            lip_stick_refresh: 0,
            curry_remaining_time: 0,
            curry_angle2: 0,
            star_remaining_time: 0,
            mushroom_remaining_time: 0,
            lightning_remaining_time: 0,
            size_flag: 0,
            metal_block_remaining_time: 0,
            combo_count: 0,
            bubble_time: 0,
            attacks_performed: 0,
            costume_id: 0,
            hitstun_frames_remaining: 0,
            meteor_cancel_window: 0,
            missed_techs: 0,
            tether_count: 0,
            temp1: 0,
            temp2: 0,

            // LongtermAccessFloat
            special_landing_lag: 0.0,
            special_fall_mobility_multiplier: 0.0,
            shield_charge: 0.0,
            curry_angle1: 0.0,
            curry_randomness: 0.0,

            // LongtermAccessBool
            is_dead: false,
            cannot_die: false,
            automatic_footstool: false,
            has_final: false,
            has_final_aura: false,
            has_curry: false,
            has_hammer: false,
            hit_by_paralyze: false,
            has_screw_attack: false,
            stamina_dead: false,
            has_tag: false,
            can_not_ledge_grab: false,
            can_not_teeter: false,
            velocity_ignore_hitstun: false,
            deflection: false,

            // RandomAccessInt
            throw_data_param1: 0,
            throw_data_param2: 0,
            throw_data_param3: 0,

            // RandomAccessFloat
            enable_turn_when_below_zero: 0.0,

            // RandomAccessBool
            character_float: false,
            enable_fast_fall: false,
            shorthop: false,
            enable_action_transition: false,
            specials_movement: false,
            enable_glide: false,
            enable_jab_loop: false,
            enable_auto_jab: false,
            enable_jab_end: false,
            landing_lag: false,
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
        for rehit in self.hitbox_sets_rehit.iter_mut() {
            *rehit = false;
        }
        self.rumble = None; // TODO: I guess rumble_loop shouldnt be reset?
        self.visited_gotos.clear();
        self.x_vel_modify = VelModify::None;
        self.y_vel_modify = VelModify::None;

        // The hitbox existed last frame so should be interpolated.
        // (Unless it gets overwritten, but that will be handled when that happens)
        for hitbox in &mut self.hitboxes {
            if let Some(ref mut hitbox) = hitbox {
                hitbox.interpolate = true;
            }
        }

        for change_action in self.change_actions.clone() {
            if self.evaluate_expression(&change_action.test).unwrap_bool() {
                // TODO: Because we currently only operate at the subaction level, this is the best we can do.
                self.change_subaction = ChangeSubaction::ChangeAction (change_action.action);
            }
        }

        // create a callstack for CallEveryFrame block
        for block in self.call_every_frame.values() {
            let calls = vec!(Call {
                block: block,
                index: 0,
                subroutine: false
            });
            self.call_stacks.push(CallStack {
                calls,
                wait_until: -1.0
            });
        }

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
                        StepEventResult::CallEveryFrame { block, thread_id } => {
                            self.call_every_frame.insert(thread_id, block);
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

        // CallEveryFrame call stacks only get one frame to complete so remove them now.
        self.call_stacks.truncate(4); // keep main, gfx, sfx and other call stacks

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
                if self.evaluate_expression(&if_statement.test).unwrap_bool() {
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
            &EventAst::CallEveryFrame { thread_id, offset } => {
                for script in all_scripts.iter() {
                    if script.offset == offset as u32 {
                        return StepEventResult::CallEveryFrame { thread_id, block: &script.block };
                    }
                }
            }
            &EventAst::RemoveCallEveryFrame { thread_id } => {
                self.call_every_frame.remove(&thread_id);
            }
            &EventAst::EnableActionStatusID (_) => { } // TODO
            &EventAst::ChangeActionStatus { .. } => { } // TODO
            &EventAst::ChangeAction (ref change_action) => {
                self.change_actions.push(change_action.clone());
            }
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
                if args.hitbox_id < self.hitboxes.len() as u8 {
                    // Need to check this here, as hitboxes can be deleted in the current frame but still exist in the previous frame.
                    // In this case interpolation should not occur.
                    let interpolate = if let Some(ref prev_hitbox) = self.hitboxes[args.hitbox_id as usize] {
                        match prev_hitbox.values {
                            CollisionBoxValues::Hit (ref prev_hitbox) => args.set_id == prev_hitbox.set_id,
                            _ => false,
                        }
                    } else { false };

                    // Force rehit if no existing hitboxes.
                    // Both DeleteAllHitboxes and DeleteHitbox on the last hitbox will trigger rehit.
                    // http://localhost:8000/PM3.6/Squirtle/subactions/SpecialHi.html?frame=11
                    let mut empty_set = true;
                    for hitbox in self.hitboxes.iter().filter_map(|x| x.clone()) {
                        if let CollisionBoxValues::Hit (hitbox) = hitbox.values {
                            if hitbox.set_id == args.set_id {
                                empty_set = false;
                            }
                        }
                    }
                    if empty_set {
                        if let Some(rehit) = self.hitbox_sets_rehit.get_mut(args.set_id as usize) {
                            *rehit = true;
                        }
                    }

                    self.hitboxes[args.hitbox_id as usize] = Some(ScriptCollisionBox::from_hitbox(args, interpolate));
                } else {
                    error!("invalid hitbox index {} {}", args.hitbox_id, action_name);
                }
            }
            &EventAst::CreateSpecialHitBox (ref args) => {
                if args.hitbox_args.hitbox_id < self.hitboxes.len() as u8 {
                    // Need to check this here, as hitboxes can be deleted in the current frame but still exist in the previous frame.
                    // In this case interpolation should not occur.
                    let index = args.hitbox_args.hitbox_id as usize;
                    let interpolate = if let Some(ref prev_hitbox) = self.hitboxes[index] {
                        match prev_hitbox.values {
                            CollisionBoxValues::Hit (ref prev_hitbox) => args.hitbox_args.set_id == prev_hitbox.set_id,
                            _ => false,
                        }
                    } else { false };

                    // Force rehit if no existing hitboxes.
                    // Both DeleteAllHitboxes and DeleteHitbox on the last hitbox will trigger rehit.
                    // http://localhost:8000/PM3.6/Squirtle/subactions/SpecialHi.html
                    let mut empty_set = true;
                    for hitbox in self.hitboxes.iter().filter_map(|x| x.clone()) {
                        if let CollisionBoxValues::Hit (hitbox) = hitbox.values {
                            if hitbox.set_id == args.hitbox_args.set_id {
                                empty_set = false;
                            }
                        }
                    }
                    if empty_set {
                        if let Some(rehit) = self.hitbox_sets_rehit.get_mut(args.hitbox_args.set_id as usize) {
                            *rehit = true;
                        }
                    }

                    self.hitboxes[index] = Some(ScriptCollisionBox::from_special_hitbox(args, interpolate));
                } else {
                    error!("invalid hitbox index {} {}", args.hitbox_args.hitbox_id, action_name);
                }
            }
            &EventAst::DeleteAllHitBoxes => {
                for hitbox in self.hitboxes.iter_mut() {
                    if hitbox.as_ref().map(|x| x.is_hit()).unwrap_or(false) {
                        *hitbox = None;
                    }
                }
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
            &EventAst::DeleteHitBox (hitbox_id) => {
                // Shock claims this doesnt work on special hitboxes but it seems to work fine here:
                // http://localhost:8000/PM3.6/Squirtle/subactions/SpecialHi.html
                if self.hitboxes[hitbox_id as usize].as_ref().map(|x| x.is_hit()).unwrap_or(false) {
                    self.hitboxes[hitbox_id as usize] = None;
                }
            }
            &EventAst::CreateGrabBox (ref args) => {
                let mut interpolate = false;
                if let Some(ref prev_hitbox) = self.hitboxes[args.hitbox_id as usize] {
                    // TODO: Should a grabbox interpolate from an existing hitbox and vice versa
                    if let CollisionBoxValues::Grab (_) = prev_hitbox.values {
                        interpolate = true;
                    }
                }
                if args.hitbox_id < self.hitboxes.len() as i32 {
                    self.hitboxes[args.hitbox_id as usize] = Some(ScriptCollisionBox::from_grabbox(args, interpolate));
                } else {
                    error!("invalid hitbox index {} {}", args.hitbox_id, action_name);
                }
            }
            &EventAst::DeleteGrabBox (hitbox_id) => {
                if self.hitboxes[hitbox_id as usize].as_ref().map(|x| x.is_grab()).unwrap_or(false) {
                    self.hitboxes[hitbox_id as usize] = None;
                }
            }
            &EventAst::DeleteAllGrabBoxes => {
                for hitbox in self.hitboxes.iter_mut() {
                    if hitbox.as_ref().map(|x| x.is_grab()).unwrap_or(false) {
                        *hitbox = None;
                    }
                }
            }

            // hurtboxes
            &EventAst::ChangeHurtBoxStateAll { ref state } => {
                self.hurtbox_state_all = state.clone();
            }
            &EventAst::ChangeHurtBoxStateSpecific { bone, ref state } => {
                self.hurtbox_states.insert(high_level_fighter::get_bone_index(bone), state.clone());
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
                    self.x_vel = values.x_vel;
                    self.x_vel_modify = VelModify::Set(values.x_vel);
                }
                else {
                    self.x_vel += values.x_vel;
                    self.x_vel_modify = VelModify::Add(self.x_vel_modify.value() + values.x_vel);
                }

                if values.y_set {
                    self.y_vel = values.y_vel;
                    self.y_vel_modify = VelModify::Set(values.y_vel);
                }
                else {
                    self.y_vel += values.y_vel;
                    self.y_vel_modify = VelModify::Add(self.y_vel_modify.value() + values.y_vel);
                }
            }
            &EventAst::SetVelocity { x_vel, y_vel } => {
                self.x_vel = x_vel;
                self.y_vel = y_vel;

                self.x_vel_modify = VelModify::Set(x_vel);
                self.y_vel_modify = VelModify::Set(y_vel);
            }
            &EventAst::AddVelocity { x_vel, y_vel } => {
                self.x_vel += x_vel;
                self.y_vel += y_vel;

                self.x_vel_modify = VelModify::Add(self.x_vel_modify.value() + x_vel);
                self.y_vel_modify = VelModify::Add(self.y_vel_modify.value() + y_vel);
            }
            &EventAst::DisableMovement (ref disable_movement) => {
                self.disable_movement = disable_movement.clone();
            }
            &EventAst::DisableMovement2 (_) => { } // TODO: What!?!?
            &EventAst::ResetVerticalVelocityAndAcceleration (reset) => {
                if reset {
                    self.y_vel = 0.0;

                    self.y_vel_modify = VelModify::Set(0.0);
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
            &EventAst::IntVariableSet { value, ref variable } => {
                self.set_variable_int(variable, value);
            }
            &EventAst::IntVariableAdd { value, ref variable } => {
                let old_value = self.get_variable_int(variable);
                self.set_variable_int(variable, old_value + value);
            }
            &EventAst::IntVariableSubtract { value, ref variable } => {
                let old_value = self.get_variable_int(variable);
                self.set_variable_int(variable, old_value - value);
            }
            &EventAst::IntVariableIncrement { ref variable } => {
                let old_value = self.get_variable_int(variable);
                self.set_variable_int(variable, old_value + 1);
            }
            &EventAst::IntVariableDecrement { ref variable } => {
                let old_value = self.get_variable_int(variable);
                self.set_variable_int(variable, old_value - 1);
            }
            &EventAst::FloatVariableSet { value, ref variable } => {
                self.set_variable_float(variable, value);
            }
            &EventAst::FloatVariableAdd { value, ref variable } => {
                let old_value = self.get_variable_float(variable);
                self.set_variable_float(variable, old_value + value);
            }
            &EventAst::FloatVariableSubtract { value, ref variable } => {
                let old_value = self.get_variable_float(variable);
                self.set_variable_float(variable, old_value - value);
            }
            &EventAst::FloatVariableMultiply { value, ref variable } => {
                let old_value = self.get_variable_float(variable);
                self.set_variable_float(variable, old_value * value);
            }
            &EventAst::FloatVariableDivide { value, ref variable } => {
                let old_value = self.get_variable_float(variable);
                self.set_variable_float(variable, old_value / value);
            }
            &EventAst::BoolVariableSetTrue { ref variable } => {
                match variable.data_type() {
                    VariableDataType::Int   => self.set_variable_int_inner(variable, 1),
                    VariableDataType::Float => self.set_variable_float_inner(variable, 1.0),
                    VariableDataType::Bool  => self.set_variable_bool_inner(variable, true),
                    VariableDataType::Unknown { .. } => { }
                }
            }
            &EventAst::BoolVariableSetFalse { ref variable } => {
                match variable.data_type() {
                    VariableDataType::Int   => self.set_variable_int_inner(variable, 0),
                    VariableDataType::Float => self.set_variable_float_inner(variable, 0.0),
                    VariableDataType::Bool  => self.set_variable_bool_inner(variable, false),
                    VariableDataType::Unknown { .. } => { }
                }
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

    fn evaluate_expression(&mut self, expression: &Expression) -> ExprResult {
        match expression {
            &Expression::Nullary (ref requirement) => {
                ExprResult::Bool (match requirement {
                    Requirement::CharacterExists => true,
                    Requirement::OnGround => true,
                    Requirement::InAir => false,
                    Requirement::FacingRight => true,
                    Requirement::HasntTethered3Times => true,
                    Requirement::IsNotInDamagingLens => true,
                    _ => false
                })
            }
            &Expression::Unary (ref unary) => {
                ExprResult::Bool (match unary.requirement {
                    Requirement::CharacterExists => true,
                    Requirement::OnGround => true,
                    Requirement::InAir => false,
                    Requirement::FacingRight => true,
                    Requirement::HasntTethered3Times => true,
                    Requirement::IsNotInDamagingLens => true,
                    _ => false
                })
            }
            &Expression::Binary (ref binary) => {
                let result = match &binary.operator {
                    &ComparisonOperator::LessThan => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          < right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) < right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          < right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          < right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    &ComparisonOperator::LessThanOrEqual => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          <= right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) <= right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          <= right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          <= right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    &ComparisonOperator::Equal => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          == right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) == right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          == right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          == right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    &ComparisonOperator::NotEqual => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          != right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) != right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          != right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          != right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    &ComparisonOperator::GreaterThanOrEqual => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          >= right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) >= right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          >= right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          >= right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    &ComparisonOperator::GreaterThan => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          > right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) > right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          > right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          > right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    &ComparisonOperator::Or                 => self.evaluate_expression(&*binary.left).unwrap_bool() || self.evaluate_expression(&*binary.right).unwrap_bool(),
                    &ComparisonOperator::And                => self.evaluate_expression(&*binary.left).unwrap_bool() && self.evaluate_expression(&*binary.right).unwrap_bool(),
                    &ComparisonOperator::UnknownArg (_)     => false,
                };
                ExprResult::Bool (result)
            }
            &Expression::Not (ref expression) => ExprResult::Bool (!self.evaluate_expression(expression).unwrap_bool()),
            &Expression::Variable (ref variable) => {
                match variable.data_type() {
                    VariableDataType::Int            => ExprResult::Int   (self.get_variable_int_inner(variable)),
                    VariableDataType::Float          => ExprResult::Float (self.get_variable_float_inner(variable)),
                    VariableDataType::Bool           => ExprResult::Bool  (self.get_variable_bool_inner(variable)),
                    VariableDataType::Unknown { .. } => ExprResult::Bool (false), // can't be handled properly so just give a dummy value
                }
            }
            &Expression::Value (int) => ExprResult::Int (int),
            &Expression::Scalar (float) => ExprResult::Float (float),
        }
    }

    fn get_variable_int(&mut self, variable: &VariableAst) -> i32 {
        match variable.data_type() {
            VariableDataType::Int   => self.get_variable_int_inner(variable),
            VariableDataType::Float => self.get_variable_float_inner(variable) as i32,
            VariableDataType::Bool  => if self.get_variable_bool_inner(variable) { 1 } else { 0 },
            VariableDataType::Unknown { .. } => 0,
        }
    }

    fn get_variable_float(&mut self, variable: &VariableAst) -> f32 {
        match variable.data_type() {
            VariableDataType::Int   => self.get_variable_int_inner(variable) as f32,
            VariableDataType::Float => self.get_variable_float_inner(variable),
            VariableDataType::Bool  => if self.get_variable_bool_inner(variable) { 1.0 } else { 0.0 },
            VariableDataType::Unknown { .. } => 0.0,
        }
    }

    fn set_variable_int(&mut self, variable: &VariableAst, value: i32) {
        match variable.data_type() {
            VariableDataType::Int   => self.set_variable_int_inner(variable, value),
            VariableDataType::Float => self.set_variable_float_inner(variable, value as f32),
            VariableDataType::Bool  => self.set_variable_bool_inner(variable, value != 0),
            VariableDataType::Unknown { .. } => { }
        }
    }

    fn set_variable_float(&mut self, variable: &VariableAst, value: f32) {
        match variable.data_type() {
            VariableDataType::Int   => self.set_variable_int_inner(variable, value as i32),
            VariableDataType::Float => self.set_variable_float_inner(variable, value),
            VariableDataType::Bool  => self.set_variable_bool_inner(variable, value != 0.0),
            VariableDataType::Unknown { .. } => { }
        }
    }

    fn get_variable_int_inner(&self, variable: &VariableAst) -> i32 {
        match variable {
            VariableAst::InternalConstantInt (InternalConstantInt::CurrentFrame) => self.frame_index as i32,
            VariableAst::InternalConstantInt (InternalConstantInt::CharacterDirection) => 1,
            VariableAst::InternalConstantInt (InternalConstantInt::CharacterDirectionOpposite) => -1,
            VariableAst::InternalConstantInt (InternalConstantInt::CurrentFrameSpeed) => self.frame_speed_modifier as i32,
            VariableAst::InternalConstantInt (InternalConstantInt::CurrentSubaction) => 0, // TODO: Get this passed as an argument to ScriptRunner::new
            VariableAst::InternalConstantInt (InternalConstantInt::CurrentAction) => 0, // TODO: Get this passed as an argument to ScriptRunner::new
            VariableAst::InternalConstantInt (_) => 0, // Best we can do for everything else is 0
            VariableAst::LongtermAccessInt   (LongtermAccessInt::JumpsUsed) => self.jumps_used,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::WallJumpCount) => self.wall_jump_count,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::WallJumpInterval) => self.wall_jump_interval,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::FootstoolCount) => self.footstool_count,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::FallTime) => self.fall_time,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::SwimTime) => self.swim_time,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::LipStickRefresh) => self.lip_stick_refresh,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::CurryRemainingTime) => self.curry_remaining_time,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::CurryAngle2) => self.curry_angle2,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::StarRemainingTime) => self.star_remaining_time,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::MushroomRemainingTime) => self.mushroom_remaining_time,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::LightningRemainingTime) => self.lightning_remaining_time,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::SizeFlag) => self.size_flag,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::MetalBlockRemainingTime) => self.metal_block_remaining_time,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::ComboCount) => self.combo_count,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::BubbleTime) => self.bubble_time,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::AttacksPerformed) => self.attacks_performed,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::CostumeID) => self.costume_id,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::HitstunFramesRemaining) => self.hitstun_frames_remaining,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::MeteorCancelWindow) => self.meteor_cancel_window,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::MissedTechs) => self.missed_techs,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::TetherCount) => self.tether_count,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::Temp1) => self.temp1,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::Temp2) => self.temp2,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::Address (_)) => 0,
            VariableAst::RandomAccessInt     (RandomAccessInt::ThrowDataParam1) => self.throw_data_param1,
            VariableAst::RandomAccessInt     (RandomAccessInt::ThrowDataParam2) => self.throw_data_param2,
            VariableAst::RandomAccessInt     (RandomAccessInt::ThrowDataParam3) => self.throw_data_param3,
            VariableAst::RandomAccessInt     (RandomAccessInt::Address (_)) => 0, // TODO
            VariableAst::Unknown             { .. } => 0, // Likely from garbage data

            VariableAst::LongtermAccessFloat (_) | VariableAst::LongtermAccessBool (_) |
            VariableAst::RandomAccessFloat (_) | VariableAst::RandomAccessBool (_)
                => panic!("Called get_variable_int on a variable that is not an int. '{:?}' It is a brawllib_rs logic error if this is reached", variable),
        }
    }

    fn set_variable_int_inner(&mut self, variable: &VariableAst, value: i32) {
        match variable {
            VariableAst::InternalConstantInt (_) => {}, // Cant set a constant
            VariableAst::LongtermAccessInt   (LongtermAccessInt::JumpsUsed) => self.jumps_used = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::WallJumpCount) => self.wall_jump_count = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::WallJumpInterval) => self.wall_jump_interval = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::FootstoolCount) => self.footstool_count = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::FallTime) => self.fall_time = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::SwimTime) => self.swim_time = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::LipStickRefresh) => self.lip_stick_refresh = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::CurryRemainingTime) => self.curry_remaining_time = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::CurryAngle2) => self.curry_angle2 = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::StarRemainingTime) => self.star_remaining_time = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::MushroomRemainingTime) => self.mushroom_remaining_time = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::LightningRemainingTime) => self.lightning_remaining_time = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::SizeFlag) => self.size_flag = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::MetalBlockRemainingTime) => self.metal_block_remaining_time = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::ComboCount) => self.combo_count = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::BubbleTime) => self.bubble_time = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::AttacksPerformed) => self.attacks_performed = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::CostumeID) => self.costume_id = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::HitstunFramesRemaining) => self.hitstun_frames_remaining = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::MeteorCancelWindow) => self.meteor_cancel_window = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::MissedTechs) => self.missed_techs = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::TetherCount) => self.tether_count = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::Temp1) => self.temp1 = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::Temp2) => self.temp2 = value,
            VariableAst::LongtermAccessInt   (LongtermAccessInt::Address (_)) => { }, // TODO
            VariableAst::RandomAccessInt     (RandomAccessInt::ThrowDataParam1) => self.throw_data_param1 = value,
            VariableAst::RandomAccessInt     (RandomAccessInt::ThrowDataParam2) => self.throw_data_param2 = value,
            VariableAst::RandomAccessInt     (RandomAccessInt::ThrowDataParam3) => self.throw_data_param3 = value,
            VariableAst::RandomAccessInt     (RandomAccessInt::Address (_)) => { }, // TODO
            VariableAst::Unknown             { .. } => {}, // Likely from garbage data

            VariableAst::LongtermAccessFloat (_) | VariableAst::LongtermAccessBool (_) |
            VariableAst::RandomAccessFloat (_) | VariableAst::RandomAccessBool (_)
                => panic!("Called set_variable_int_inner on a variable that is not an int. '{:?}' It is a brawllib_rs logic error if this is reached.", variable),
        }
    }

    fn get_variable_float_inner(&self, variable: &VariableAst) -> f32 {
        match variable {
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::SpecialLandingLag) => self.special_landing_lag,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::SpecialFallMobilityMultiplier) => self.special_fall_mobility_multiplier,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::ShieldCharge) => self.shield_charge,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::CurryAngle1) => self.curry_angle1,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::CurryRandomness) => self.curry_randomness,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::Address (_)) => 0.0, // TODO
            VariableAst::RandomAccessFloat   (RandomAccessFloat::EnableTurnWhenBelowZero) => self.enable_turn_when_below_zero,
            VariableAst::RandomAccessFloat   (RandomAccessFloat::Address (_)) => 0.0, // TODO
            VariableAst::Unknown             { .. } => 0.0, // Likely from garbage data

            VariableAst::LongtermAccessInt (_) | VariableAst::LongtermAccessBool (_) |
            VariableAst::RandomAccessInt (_) | VariableAst::RandomAccessBool (_) |
            VariableAst::InternalConstantInt (_) => panic!("Called get_variable_float on a variable that is not a float. '{:?}' It is a brawllib_rs logic error if this is reached.", variable),
        }
    }

    fn set_variable_float_inner(&mut self, variable: &VariableAst, value: f32) {
        match variable {
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::SpecialLandingLag) => self.special_landing_lag = value,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::SpecialFallMobilityMultiplier) => self.special_fall_mobility_multiplier = value,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::ShieldCharge) => self.shield_charge = value,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::CurryAngle1) => self.curry_angle1 = value,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::CurryRandomness) => self.curry_randomness = value,
            VariableAst::LongtermAccessFloat (LongtermAccessFloat::Address (_)) => { }, // TODO
            VariableAst::RandomAccessFloat   (RandomAccessFloat::EnableTurnWhenBelowZero) => self.enable_turn_when_below_zero = value,
            VariableAst::RandomAccessFloat   (RandomAccessFloat::Address (_)) => { } // TODO
            VariableAst::Unknown             { .. } => { }, // Likely from garbage data

            VariableAst::LongtermAccessInt (_) | VariableAst::LongtermAccessBool (_) |
            VariableAst::RandomAccessInt (_) | VariableAst::RandomAccessBool (_) |
            VariableAst::InternalConstantInt (_) => panic!("Called set_variable_float_inner on a variable that is not a float. '{:?}' It is a brawllib_rs logic error if this is reached.", variable),
        }
    }

    fn get_variable_bool_inner(&mut self, variable: &VariableAst) -> bool {
        match variable {
            VariableAst::LongtermAccessBool (LongtermAccessBool::IsDead) => self.is_dead,
            VariableAst::LongtermAccessBool (LongtermAccessBool::CannotDie) => self.cannot_die,
            VariableAst::LongtermAccessBool (LongtermAccessBool::AutomaticFootstool) => self.automatic_footstool,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasFinal) => self.has_final,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasFinalAura) => self.has_final_aura,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasCurry) => self.has_curry,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasHammer) => self.has_hammer,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HitByParalyze) => self.hit_by_paralyze,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasScrewAttack) => self.has_screw_attack,
            VariableAst::LongtermAccessBool (LongtermAccessBool::StaminaDead) => self.stamina_dead,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasTag) => self.has_tag,
            VariableAst::LongtermAccessBool (LongtermAccessBool::CanNotLedgeGrab) => self.can_not_ledge_grab,
            VariableAst::LongtermAccessBool (LongtermAccessBool::CanNotTeeter) => self.can_not_teeter,
            VariableAst::LongtermAccessBool (LongtermAccessBool::VelocityIgnoreHitstun) => self.velocity_ignore_hitstun,
            VariableAst::LongtermAccessBool (LongtermAccessBool::Deflection) => self.deflection,
            VariableAst::LongtermAccessBool (LongtermAccessBool::Address (_)) => false, // TODO

            VariableAst::RandomAccessBool (RandomAccessBool::CharacterFloat) => self.character_float,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableFastFall) => self.enable_fast_fall,
            VariableAst::RandomAccessBool (RandomAccessBool::Shorthop) => self.shorthop,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableActionTransition) => self.enable_action_transition,
            VariableAst::RandomAccessBool (RandomAccessBool::SpecialsMovement) => self.specials_movement,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableGlide) => self.enable_glide,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableJabLoop) => self.enable_jab_loop,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableAutoJab) => self.enable_auto_jab,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableJabEnd) => self.enable_jab_end,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableLandingLag) => self.landing_lag,
            VariableAst::RandomAccessBool (RandomAccessBool::Address (_)) => false, // TODO
            VariableAst::Unknown          { .. } => false, // Likely from garbage data

            VariableAst::LongtermAccessInt (_) | VariableAst::LongtermAccessFloat (_) |
            VariableAst::RandomAccessInt (_) | VariableAst::RandomAccessFloat (_) |
            VariableAst::InternalConstantInt (_) => panic!("Called get_variable_bool on a variable that is not a bool. '{:?}' It is a brawllib_rs logic error if this is reached.", variable),
        }
    }

    fn set_variable_bool_inner(&mut self, variable: &VariableAst, value: bool) {
        match variable {
            VariableAst::LongtermAccessBool (LongtermAccessBool::IsDead) => self.is_dead = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::CannotDie) => self.cannot_die = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::AutomaticFootstool) => self.automatic_footstool = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasFinal) => self.has_final = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasFinalAura) => self.has_final_aura = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasCurry) => self.has_curry = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasHammer) => self.has_hammer = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HitByParalyze) => self.hit_by_paralyze = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasScrewAttack) => self.has_screw_attack = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::StaminaDead) => self.stamina_dead = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::HasTag) => self.has_tag = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::CanNotLedgeGrab) => self.can_not_ledge_grab = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::CanNotTeeter) => self.can_not_teeter = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::VelocityIgnoreHitstun) => self.velocity_ignore_hitstun = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::Deflection) => self.deflection = value,
            VariableAst::LongtermAccessBool (LongtermAccessBool::Address (_)) => {}

            VariableAst::RandomAccessBool (RandomAccessBool::CharacterFloat) => self.character_float = value,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableFastFall) => self.enable_fast_fall = value,
            VariableAst::RandomAccessBool (RandomAccessBool::Shorthop) => self.shorthop = value,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableActionTransition) => self.enable_action_transition = value,
            VariableAst::RandomAccessBool (RandomAccessBool::SpecialsMovement) => self.specials_movement = value,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableGlide) => self.enable_glide = value,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableJabLoop) => self.enable_jab_loop = value,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableAutoJab) => self.enable_auto_jab = value,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableJabEnd) => self.enable_jab_end = value,
            VariableAst::RandomAccessBool (RandomAccessBool::EnableLandingLag) => self.landing_lag = value,
            VariableAst::RandomAccessBool (RandomAccessBool::Address (_)) => { }
            VariableAst::Unknown          { .. } => {}, // Likely from garbage data

            VariableAst::LongtermAccessInt (_) | VariableAst::LongtermAccessFloat (_) |
            VariableAst::RandomAccessInt (_) | VariableAst::RandomAccessFloat (_) |
            VariableAst::InternalConstantInt (_) => panic!("Called set_variable_bool_inner on a variable that is not a bool. '{:?}' It is a brawllib_rs logic error if this is reached.", variable),
        }
    }
}

enum StepEventResult<'a> {
    WaitUntil      (f32),
    NewForLoop     { block: &'a Block, iterations: i32 },
    NewCall        (&'a Block),
    Goto           (&'a Block),
    Subroutine     (&'a Block),
    CallEveryFrame { block: &'a Block, thread_id: i32 },
    Return,
    None
}

#[derive(Debug)]
enum ExprResult {
    Int   (i32),
    Float (f32),
    Bool  (bool),
}

impl ExprResult {
    fn unwrap_bool(&self) -> bool {
        match self {
            ExprResult::Bool (result) => *result,
            _ => panic!("ExprResult was {:?} instead of bool", self)
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum VelModify {
    Set (f32),
    Add (f32),
    None,
}

impl VelModify {
    pub fn value(&self) -> f32 {
        match self {
            VelModify::Set (a) => *a,
            VelModify::Add (a) => *a,
            VelModify::None    => 0.0,
        }
    }
}
