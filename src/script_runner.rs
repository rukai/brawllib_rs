use crate::fighter::WiiRDFrameSpeedModifier;
use crate::high_level_fighter;
use crate::high_level_fighter::{CollisionBoxValues, SectionScriptAst};
use crate::sakurai::fighter_data::ArcFighterData;
use crate::script::{Requirement, VariableDataType};
use crate::script_ast::variable_ast::{
    InternalConstantInt, LongtermAccessBool, LongtermAccessFloat, LongtermAccessInt,
    RandomAccessBool, RandomAccessFloat, RandomAccessInt, VariableAst,
};
use crate::script_ast::{
    ArmorType, BinaryExpression, Block, ComparisonOperator, DisableMovement, EdgeSlide, EventAst,
    Expression, FloatValue, GrabBoxArguments, HitBoxArguments, HurtBoxState, Interrupt, Iterations,
    LedgeGrabEnable, ScriptAst, SpecialHitBoxArguments, SpecifyThrow, ThrowUse,
};

use std::collections::HashMap;

pub struct ScriptRunner<'a> {
    pub subaction_name: String,
    pub wiird_frame_speed_modifiers: &'a [WiiRDFrameSpeedModifier],
    pub call_stacks: Vec<CallStack<'a>>,
    pub fighter_scripts: &'a [&'a ScriptAst],
    pub common_scripts: &'a [&'a ScriptAst],
    pub section_scripts: &'a [SectionScriptAst],
    pub call_every_frame: HashMap<i32, CallEveryFrame<'a>>,
    pub visited_gotos: Vec<i32>,
    pub subaction_index: usize,
    pub frame_index: f32,     // affected by frame speed modifiers
    pub animation_index: f32, // affected by frame speed modifiers, usually in sync with frame_index but not always because some commands affect only animation_index
    pub frame_count: usize, // goes up by exactly 1 every frame, only used for external statistics like iasa
    pub interruptible: bool,
    pub hitboxes: [Option<ScriptCollisionBox>; 7],
    pub hurtbox_state_all: HurtBoxState,
    pub hurtbox_states: HashMap<i32, HurtBoxState>,
    pub ledge_grab_enable: LedgeGrabEnable,
    pub frame_speed_modifier: f32,
    pub tag_display: bool,
    /// State is maintained across frames
    pub x: f32,
    /// State is maintained across frames
    pub y: f32,
    /// State is maintained across frames
    pub x_vel: f32,
    /// State is maintained across frames
    pub y_vel: f32,
    /// Reset to None before processing each frame
    pub x_vel_modify: VelModify,
    /// Reset to None before processing each frame
    pub y_vel_modify: VelModify,
    pub disable_movement: DisableMovement,
    pub armor_type: ArmorType,
    pub armor_tolerance: f32,
    pub damage: f32,
    pub airbourne: bool,
    pub edge_slide: EdgeSlide, // TODO: This value seems inaccurate as its rarely set, is ledge cancel normally just hardcoded for say movement vs attack
    pub reverse_direction: bool,
    pub change_subaction: ChangeSubaction,
    /// Children of these bones are also visible
    pub invisible_bones: Vec<i32>,
    /// Each Interrupt is rechecked every frame after being created
    pub interrupts: Vec<Interrupt>,
    /// As a hack to get some subactions working we store "bad interrupts" here instead.
    pub bad_interrupts: Vec<Interrupt>,
    pub hitbox_sets_rehit: [bool; 10],
    pub slope_contour_stand: Option<i32>,
    pub slope_contour_full: Option<(i32, i32)>,
    pub rumble: Option<(i32, i32)>,
    pub rumble_loop: Option<(i32, i32)>,
    pub grab_interrupt_damage: Option<i32>,
    pub throw: Option<SpecifyThrow>,
    /// Reset to false before processing each frame.
    pub throw_activate: bool,

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
    pub longterm_access_int: Vec<i32>,

    // LongtermAccessFloat
    pub special_landing_lag: f32,
    pub special_fall_mobility_multiplier: f32,
    pub shield_charge: f32,
    pub curry_angle1: f32,
    pub curry_randomness: f32,
    pub longterm_access_float: Vec<f32>,

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
    pub longterm_access_bool: Vec<bool>,

    // RandomAccessInt
    pub throw_data_param1: i32,
    pub throw_data_param2: i32,
    pub throw_data_param3: i32,
    pub random_access_int: Vec<i32>,

    // RandomAccessFloat
    pub enable_turn_when_below_zero: f32,
    pub random_access_float: Vec<f32>,

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
    pub random_access_bool: Vec<bool>,
}

pub struct CallStack<'a> {
    pub calls: Vec<Call<'a>>,
    pub wait_until: f32,
}

pub struct Call<'a> {
    pub block: &'a Block,
    pub else_branch: Option<&'a Block>,
    pub index: usize,
    pub subroutine: bool,
    pub external: bool,
    /// I have tested the `And` and `Or` events to have no effect outside of any if statement
    pub if_statement: bool,
    pub execute: bool,
}

pub enum ChangeSubaction {
    Continue,
    InfiniteLoop,
    ChangeSubaction(i32),
    ChangeSubactionRestartFrame(i32),
    // TODO: Because we currently only operate at the subaction level, this is the best we can do.
    Interrupt(i32),
}

#[derive(Clone, Debug)]
pub struct ScriptCollisionBox {
    pub bone_index: i16,
    pub hitbox_id: u8,
    pub x_offset: f32,
    pub y_offset: f32,
    pub z_offset: f32,
    pub size: f32,
    pub values: CollisionBoxValues,
    pub interpolate: bool,
}

impl ScriptCollisionBox {
    fn from_hitbox(args: &HitBoxArguments, interpolate: bool, damage: f32) -> ScriptCollisionBox {
        ScriptCollisionBox {
            bone_index: args.bone_index,
            hitbox_id: args.hitbox_id,
            x_offset: args.x_offset,
            y_offset: args.y_offset,
            z_offset: args.z_offset,
            size: args.size,
            values: CollisionBoxValues::from_hitbox(args, damage),
            interpolate,
        }
    }

    fn from_special_hitbox(
        args: &SpecialHitBoxArguments,
        interpolate: bool,
        damage: f32,
    ) -> ScriptCollisionBox {
        ScriptCollisionBox {
            bone_index: args.hitbox_args.bone_index,
            hitbox_id: args.hitbox_args.hitbox_id,
            x_offset: args.hitbox_args.x_offset,
            y_offset: args.hitbox_args.y_offset,
            z_offset: args.hitbox_args.z_offset,
            size: args.hitbox_args.size,
            values: CollisionBoxValues::from_special_hitbox(args, damage),
            interpolate,
        }
    }

    fn from_grabbox(args: &GrabBoxArguments, interpolate: bool) -> ScriptCollisionBox {
        ScriptCollisionBox {
            bone_index: args.bone_index as i16,
            hitbox_id: args.hitbox_id as u8,
            x_offset: args.x_offset,
            y_offset: args.y_offset,
            z_offset: args.z_offset,
            size: args.size,
            values: CollisionBoxValues::from_grabbox(args),
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
    pub fn new(
        subaction_index: usize,
        wiird_frame_speed_modifiers: &'a [WiiRDFrameSpeedModifier],
        subaction_scripts: &[&'a ScriptAst],
        fighter_scripts: &'a [&'a ScriptAst],
        common_scripts: &'a [&'a ScriptAst],
        section_scripts: &'a [SectionScriptAst],
        init_hack_script: &Block,
        fighter_data: &ArcFighterData,
        subaction_name: String,
    ) -> ScriptRunner<'a> {
        let mut call_stacks = vec![];
        for script in subaction_scripts {
            let calls = vec![Call {
                block: &script.block,
                else_branch: None,
                index: 0,
                subroutine: false,
                external: false,
                if_statement: false,
                execute: true,
            }];
            call_stacks.push(CallStack {
                calls,
                wait_until: -1.0,
            });
        }

        let ledge_grab_enable = match subaction_name.as_ref() {
            "JumpF" | "JumpB" | "JumpAerialF" | "JumpAerialB" | "FallF" | "FallB" | "Fall"
            | "FallAerialF" | "FallAerialB" | "FallAerial" | "FallSpecialF" | "FallSpecialB"
            | "FallSpecial" => LedgeGrabEnable::EnableInFront,
            _ => LedgeGrabEnable::Disable,
        };

        // TODO: Need to understand what ModelVisibility.cs:269 is doing (ResetVisibility)
        //
        // Actually:
        // I'm pretty sure it doesnt affect children bones, so everything makes sense now.
        // Actually x2:
        // Why is bone 0 used then, its got nothing to render :O
        // ABORT ABORT
        //
        // I think I want a tree.
        // *   It lets me handle the initial values from the flags
        // *   It lets me easily set all children
        // *   It lets me easily set and reset values

        let mut invisible_bones = vec![];
        // TODO: populate with bone flags
        // This isnt actually part of the visibility reset that occurs at the start of a subaction
        // and should be only called during initialization if such a refactor occurs.

        for reference in &fighter_data.model_visibility.references {
            for default in &fighter_data.model_visibility.defaults {
                if let Some(bone_switch) =
                    reference.bone_switches.get(default.switch_index as usize)
                    && let Some(group) = bone_switch.groups.get(default.group_index as usize)
                {
                    for bone in &group.bones {
                        invisible_bones.push(*bone);
                    }
                }
            }
        }
        ////First, disable bones
        //foreach (ModelVisBoneSwitch Switch in entry)
        //{
        //    int i = 0;
        //    foreach (ModelVisGroup Group in Switch)
        //    {
        //        if (i != Switch._defaultGroup)
        //            foreach (BoneIndexValue b in Group._bones)
        //                if (b.BoneNode != null)
        //                    foreach (DrawCall p in b.BoneNode._visDrawCalls)
        //                        p._render = false;
        //        i++;
        //    }
        //}

        // TODO: enable bones.
        // This doesnt actually affect anything at the moment.
        // The two cases where this could affect things in the future are:
        // *   ScriptRunner is extended to run at the action level, in which case new subactions would cause the bones to reset after being potentially modified
        // *   invisible_bones is populated with the visible bone flags from the MDL0 bone data.
        for reference in &fighter_data.model_visibility.references {
            for default in &fighter_data.model_visibility.defaults {
                if let Some(bone_switch) =
                    reference.bone_switches.get(default.switch_index as usize)
                    && let Some(group) = bone_switch.groups.get(default.group_index as usize)
                {
                    for bone in &group.bones {
                        invisible_bones.retain(|x| x != bone);
                    }
                }
            }
        }

        ////Now, enable bones
        //foreach (ModelVisBoneSwitch Switch in entry)
        //    if (Switch._defaultGroup >= 0 && Switch._defaultGroup < Switch.Count)
        //    {
        //        ModelVisGroup Group = Switch[Switch._defaultGroup];
        //        foreach (BoneIndexValue b in Group._bones)
        //            if (b.BoneNode != null)
        //                foreach (DrawCall p in b.BoneNode._visDrawCalls)
        //                    p._render = true;
        //    }
        //
        //
        //
        //    FOR THE EVENT:
        //public void ApplyVisibility(int refId, int switchID, int groupID)
        //{
        //    if (refId < 0 || refId >= _references.Count)
        //        return;

        //    //Get the target reference
        //    ModelVisReference refEntry = _references[refId];

        //    //Check if the reference and switch id is usable
        //    if (switchID >= refEntry.Count || switchID < 0)
        //        return;

        //    //Turn off objects
        //    ModelVisBoneSwitch switchEntry = refEntry[switchID];
        //    foreach (ModelVisGroup grp in switchEntry)
        //        foreach (BoneIndexValue b in grp._bones)
        //            if (b.BoneNode != null)
        //                foreach (DrawCall obj in b.BoneNode._visDrawCalls)
        //                    obj._render = false;

        //    //Check if the group id is usable
        //    if (groupID >= switchEntry.Count || groupID < 0)
        //        return;

        //    //Turn on objects
        //    ModelVisGroup group = switchEntry[groupID];
        //    if (group != null)
        //        foreach (BoneIndexValue b in group._bones)
        //            if (b.BoneNode != null)
        //                foreach (DrawCall obj in b.BoneNode._visDrawCalls)
        //                    obj._render = true;
        //}

        let mut runner = ScriptRunner {
            subaction_name,
            wiird_frame_speed_modifiers,
            call_stacks,
            fighter_scripts,
            common_scripts,
            section_scripts,
            subaction_index,
            ledge_grab_enable,
            call_every_frame: HashMap::new(),
            visited_gotos: vec![],
            frame_index: 0.0,
            animation_index: 0.0,
            frame_count: 0,
            interruptible: false,
            hitboxes: [None, None, None, None, None, None, None],
            hurtbox_state_all: HurtBoxState::Normal,
            hurtbox_states: HashMap::new(),
            frame_speed_modifier: 1.0,
            tag_display: true,
            x: 0.0,
            y: 0.0,
            x_vel: 0.0,
            y_vel: 0.0,
            x_vel_modify: VelModify::None,
            y_vel_modify: VelModify::None,
            disable_movement: DisableMovement::Enable,
            armor_type: ArmorType::None,
            armor_tolerance: 0.0,
            damage: 0.0,
            airbourne: false,
            edge_slide: EdgeSlide::SlideOff,
            reverse_direction: false,
            change_subaction: ChangeSubaction::Continue,
            interrupts: vec![],
            bad_interrupts: vec![],
            hitbox_sets_rehit: [false; 10],
            slope_contour_stand: None,
            slope_contour_full: None,
            rumble: None,
            rumble_loop: None,
            grab_interrupt_damage: None,
            throw: None,
            throw_activate: false,
            invisible_bones,

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
            longterm_access_int: vec![0; 0x500], // TODO: How big should this be?

            // LongtermAccessFloat
            special_landing_lag: 0.0,
            special_fall_mobility_multiplier: 0.0,
            shield_charge: 0.0,
            curry_angle1: 0.0,
            curry_randomness: 0.0,
            longterm_access_float: vec![0.0; 0x500], // TODO: How big should this be?

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
            longterm_access_bool: vec![false; 0x500], // TODO: How big should this be?

            // RandomAccessInt
            throw_data_param1: 0,
            throw_data_param2: 0,
            throw_data_param3: 0,
            random_access_int: vec![0; 0x100], // Pika has 72 bytes allocated, so this should be plenty.

            // RandomAccessFloat
            enable_turn_when_below_zero: 0.0,
            random_access_float: vec![0.0; 0x100], // Pika has 56 bytes allocated, so this should be plenty.

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
            random_access_bool: vec![false; 0x100], // normally has 8-16 bytes allocated
        };

        for event in &init_hack_script.events {
            runner.step_event(event, false, &[], &[], &[]);
        }

        // Need to run the script until the first wait, so that the script is in the valid state
        // for the first frame.
        runner.step_script();

        runner
    }

    /// Steps the main, gfx, sfx and other scripts by 1 game frame.
    pub fn step(&mut self) {
        let mut fsms = vec![];
        for fsm in self.wiird_frame_speed_modifiers {
            // TODO: Because we currently only operate at the subaction level, this is the best we can do.
            if !fsm.action
                && fsm.action_subaction_id as usize == self.subaction_index
                && (self.frame_index as u16) >= fsm.frame as u16
            {
                fsms.push(fsm);
            }
        }

        fsms.sort_by_key(|x| x.frame);
        if let Some(fsm) = fsms.last() {
            self.frame_speed_modifier = fsm.frame_speed;
        }
        self.frame_index += self.frame_speed_modifier;
        self.animation_index += self.frame_speed_modifier;
        self.frame_count += 1;
        self.step_script();
    }

    fn step_script(&mut self) {
        for rehit in self.hitbox_sets_rehit.iter_mut() {
            *rehit = false;
        }
        self.throw_activate = false;
        self.rumble = None; // TODO: I guess rumble_loop shouldnt be reset?
        self.visited_gotos.clear();
        self.x_vel_modify = VelModify::None;
        self.y_vel_modify = VelModify::None;
        self.reverse_direction = false;

        // The hitbox existed last frame so should be interpolated.
        // (Unless it gets overwritten, but that will be handled when that happens)
        for hitbox in &mut self.hitboxes.iter_mut().flatten() {
            hitbox.interpolate = true;
        }

        for interrupt in self.interrupts.clone() {
            if self.evaluate_expression(&interrupt.test).unwrap_bool() {
                // TODO: Because we currently only operate at the subaction level, this is the best we can do.
                self.change_subaction = ChangeSubaction::Interrupt(interrupt.action);
            }
        }

        // create a callstack for CallEveryFrame block
        for call_every_frame in self.call_every_frame.values() {
            let calls = vec![Call {
                block: call_every_frame.block,
                else_branch: None,
                index: 0,
                subroutine: false,
                external: call_every_frame.external,
                if_statement: false,
                execute: true,
            }];
            self.call_stacks.push(CallStack {
                calls,
                wait_until: -1.0,
            });
        }

        // run the main, gfx, sfx and other scripts
        for i in 0..self.call_stacks.len() {
            while !self.call_stacks[i].calls.is_empty() {
                // reached the end of the script
                // Handle wait events
                if self.frame_index < self.call_stacks[i].wait_until {
                    break;
                }

                // Process the next event in the call_stack
                let call = self.call_stacks[i].calls.last().unwrap();
                if let Some(event) = call.block.events.get(call.index) {
                    self.call_stacks[i].calls.last_mut().unwrap().index += 1;
                    let external = self.call_stacks[i].calls.last().unwrap().external;

                    if self.call_stacks[i].calls.last().unwrap().execute {
                        match self.step_event(
                            event,
                            external,
                            self.fighter_scripts,
                            self.common_scripts,
                            self.section_scripts,
                        ) {
                            StepEventResult::WaitUntil(value) => {
                                self.call_stacks[i].wait_until = value;
                            }
                            StepEventResult::NewForLoop { block, iterations } => {
                                for _ in 0..iterations {
                                    self.call_stacks[i].calls.push(Call {
                                        block,
                                        else_branch: None,
                                        index: 0,
                                        subroutine: false,
                                        if_statement: false,
                                        execute: true,
                                        external,
                                    });
                                }
                            }
                            StepEventResult::NewCall { block } => {
                                self.call_stacks[i].calls.push(Call {
                                    block,
                                    else_branch: None,
                                    index: 0,
                                    subroutine: false,
                                    if_statement: false,
                                    execute: true,
                                    external,
                                });
                            }
                            StepEventResult::NewIfStatement {
                                then_branch,
                                else_branch,
                                execute,
                            } => {
                                self.call_stacks[i].calls.push(Call {
                                    block: then_branch,
                                    else_branch,
                                    index: 0,
                                    subroutine: false,
                                    if_statement: true,
                                    execute,
                                    external,
                                });
                            }
                            StepEventResult::IfStatementDisableExecution => {
                                if self.call_stacks[i].calls.last().unwrap().if_statement {
                                    self.call_stacks[i].calls.last_mut().unwrap().execute = false;
                                }
                            }
                            StepEventResult::Subroutine { block, external } => {
                                self.call_stacks[i].calls.push(Call {
                                    block,
                                    else_branch: None,
                                    index: 0,
                                    subroutine: true,
                                    if_statement: false,
                                    execute: true,
                                    external,
                                });
                            }
                            StepEventResult::CallEveryFrame {
                                block,
                                thread_id,
                                external,
                            } => {
                                self.call_every_frame
                                    .insert(thread_id, CallEveryFrame { block, external });
                            }
                            StepEventResult::Return => {
                                let mut run = false;
                                while run {
                                    run = self.call_stacks[i]
                                        .calls
                                        .pop()
                                        .map(|x| !x.subroutine)
                                        .unwrap_or(false);
                                }
                            }
                            StepEventResult::Goto { block, external } => {
                                self.call_stacks[i].calls.pop();
                                self.call_stacks[i].calls.push(Call {
                                    block,
                                    else_branch: None,
                                    index: 0,
                                    subroutine: false,
                                    if_statement: false,
                                    execute: true,
                                    external,
                                });
                            }
                            StepEventResult::None => {}
                        }
                    } else {
                        // when execution is disabled we run events that may resume execution, otherwise we do nothing
                        let new_execution = match event {
                            EventAst::IfStatementOr(test) => {
                                self.evaluate_expression(test).unwrap_bool()
                                    && self.call_stacks[i].calls.last().unwrap().if_statement
                            }
                            _ => false,
                        };
                        self.call_stacks[i].calls.last_mut().unwrap().execute = new_execution;
                    }
                } else {
                    // If there is an else branch, begin processing that as the main block, otherwise pop the call
                    let call = self.call_stacks[i].calls.last_mut().unwrap();
                    if let Some(else_branch) = call.else_branch {
                        call.block = else_branch;
                        call.index = 0;
                        call.else_branch = None;
                        call.execute = !call.execute;
                    } else {
                        self.call_stacks[i].calls.pop();
                    }
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
    fn step_event<'b>(
        &mut self,
        event: &'b EventAst,
        external: bool,
        fighter_scripts: &[&'b ScriptAst],
        common_scripts: &[&'b ScriptAst],
        section_scripts: &'b [SectionScriptAst],
    ) -> StepEventResult<'b> {
        match event {
            EventAst::SyncWait(value) => {
                return StepEventResult::WaitUntil(self.frame_index + *value);
            }
            EventAst::AsyncWait(value) => {
                return StepEventResult::WaitUntil(*value);
            }
            EventAst::ForLoop(for_loop) => {
                match for_loop.iterations {
                    Iterations::Finite(iterations) => {
                        return StepEventResult::NewForLoop {
                            block: &for_loop.block,
                            iterations,
                        };
                    }
                    Iterations::Infinite => {
                        // obviously an infinite loop should not be attempted :P
                        return StepEventResult::NewCall {
                            block: &for_loop.block,
                        };
                    }
                }
            }
            EventAst::Subroutine(offset) => {
                if !external
                    && let Some(script) = section_scripts
                        .iter()
                        .find(|x| x.callers.contains(&offset.origin))
                {
                    return StepEventResult::Subroutine {
                        block: &script.script.block,
                        external: true,
                    };
                }

                let all_scripts = if external {
                    common_scripts
                } else {
                    fighter_scripts
                };
                // TODO: Maybe I should implement a protection similar to visited_gotos for subroutines.
                // If that turns out to be a bad idea document why.
                for script in all_scripts.iter() {
                    if script.offset == offset.offset {
                        if !script.block.events.is_empty()
                            && std::ptr::eq(&script.block.events[0], event)
                        {
                            error!(
                                "Avoided hard Subroutine infinite loop (attempted to jump to the same location)"
                            );
                        } else {
                            return StepEventResult::Subroutine {
                                block: &script.block,
                                external,
                            };
                        }
                    }
                }
                error!("Couldnt find Subroutine offset");
            }
            EventAst::Return => {
                return StepEventResult::Return;
            }
            EventAst::Goto(offset) => {
                if !external
                    && let Some(script) = section_scripts
                        .iter()
                        .find(|x| x.callers.contains(&offset.origin))
                {
                    return StepEventResult::Goto {
                        block: &script.script.block,
                        external: true,
                    };
                }

                let all_scripts = if external {
                    common_scripts
                } else {
                    fighter_scripts
                };
                if !self.visited_gotos.contains(&offset.offset) {
                    self.visited_gotos.push(offset.offset);
                    for script in all_scripts.iter() {
                        if script.offset == offset.offset {
                            return StepEventResult::Goto {
                                block: &script.block,
                                external,
                            };
                        }
                    }
                    error!("Couldnt find Goto offset");
                }
                error!("Avoided Goto infinite loop");
            }
            EventAst::IfStatement(if_statement) => {
                let then_branch = &if_statement.then_branch;
                let else_branch = if_statement.else_branch.as_deref();
                let execute = self.evaluate_expression(&if_statement.test).unwrap_bool();
                return StepEventResult::NewIfStatement {
                    then_branch,
                    else_branch,
                    execute,
                };
            }
            EventAst::IfStatementAnd(test) => {
                if !self.evaluate_expression(test).unwrap_bool() {
                    return StepEventResult::IfStatementDisableExecution;
                }
            }
            EventAst::IfStatementOr(_) => {} // This is handled in the !execution branch
            EventAst::Switch(_, _) => {}     // TODO
            EventAst::EndSwitch => {}
            EventAst::Case(_) => {}
            EventAst::DefaultCase => {}
            EventAst::LoopRest => {
                error!("LoopRest: This means the code is expected to infinite loop")
            } // TODO: Handle infinite loops better
            EventAst::CallEveryFrame { thread_id, offset } => {
                if !external
                    && let Some(script) = section_scripts
                        .iter()
                        .find(|x| x.callers.contains(&offset.origin))
                {
                    return StepEventResult::CallEveryFrame {
                        thread_id: *thread_id,
                        block: &script.script.block,
                        external: true,
                    };
                }

                let all_scripts = if external {
                    common_scripts
                } else {
                    fighter_scripts
                };
                for script in all_scripts.iter() {
                    if script.offset == offset.offset {
                        return StepEventResult::CallEveryFrame {
                            thread_id: *thread_id,
                            block: &script.block,
                            external,
                        };
                    }
                }
            }
            EventAst::RemoveCallEveryFrame { thread_id } => {
                self.call_every_frame.remove(thread_id);
            }
            EventAst::IndependentSubroutine { .. } => {} // TODO
            EventAst::RemoveIndependentSubroutine { .. } => {} // TODO
            EventAst::SetIndependentSubroutineThreadType { .. } => {} // TODO
            EventAst::DisableInterrupt(_) => {}          // TODO
            EventAst::EnableInterrupt(_) => {}           // TODO
            EventAst::ToggleInterrupt { .. } => {}       // TODO
            EventAst::EnableInterruptGroup(_) => {}      // TODO
            EventAst::DisableInterruptGroup(_) => {}     // TODO
            EventAst::ClearInterruptGroup(_) => {}       // TODO
            EventAst::CreateInterrupt(interrupt) => {
                // If the interrupt would succeed on the first frame then ignore it.
                // This is a super hacky hack to get moves like DK's dash attack working.
                if !(self.frame_index == 0.0
                    && self.evaluate_expression(&interrupt.test).unwrap_bool())
                {
                    self.interrupts.push(interrupt.clone());
                } else {
                    self.bad_interrupts.push(interrupt.clone());
                }
            }
            EventAst::PreviousInterruptAddRequirement { test } => {
                if let Some(interrupt) = self.interrupts.last_mut() {
                    let left = Box::new(interrupt.test.clone());
                    let right = Box::new(test.clone());
                    let operator = ComparisonOperator::And;
                    interrupt.test = Expression::Binary(BinaryExpression {
                        left,
                        operator,
                        right,
                    });
                }
            }
            EventAst::InterruptAddRequirement { .. } => {} // TODO
            EventAst::AllowInterrupts => {
                self.interruptible = true;
            }
            EventAst::DisallowInterrupts => {
                self.interruptible = false;
            }
            EventAst::ChangeSubaction(v0) => {
                self.change_subaction = ChangeSubaction::ChangeSubaction(*v0);
            }
            EventAst::ChangeSubactionRestartFrame(v0) => {
                self.change_subaction = ChangeSubaction::ChangeSubactionRestartFrame(*v0);
            }

            // timing
            EventAst::SetAnimationFrame(v0) => {
                self.animation_index = *v0;
            }
            EventAst::SetAnimationAndTimerFrame(v0) => {
                self.animation_index = *v0;
                self.frame_index = *v0;
            }
            EventAst::FrameSpeedModifier { multiplier, .. } => {
                self.frame_speed_modifier = *multiplier;
            }
            EventAst::TimeManipulation(_, _) => {}

            // misc state
            EventAst::SetAirGround(v0) => {
                self.airbourne = *v0 == 0; // TODO: Seems like brawlbox is incomplete here e.g 36
            }
            EventAst::SetEdgeSlide(v0) => {
                self.edge_slide = v0.clone();
            }
            EventAst::ReverseDirection => {
                self.reverse_direction = !self.reverse_direction;
            }

            // hitboxes
            EventAst::CreateHitBox(args) => {
                if args.hitbox_id < self.hitboxes.len() as u8 {
                    // Need to check this here, as hitboxes can be deleted in the current frame but still exist in the previous frame.
                    // In this case interpolation should not occur.
                    let interpolate =
                        if let Some(prev_hitbox) = &self.hitboxes[args.hitbox_id as usize] {
                            match &prev_hitbox.values {
                                CollisionBoxValues::Hit(prev_hitbox) => {
                                    args.set_id == prev_hitbox.set_id
                                }
                                _ => false,
                            }
                        } else {
                            false
                        };

                    // Force rehit if no existing hitboxes.
                    // Both DeleteAllHitboxes and DeleteHitbox on the last hitbox will trigger rehit.
                    // http://localhost:8000/PM3.6/Squirtle/subactions/SpecialHi.html?frame=11
                    let mut empty_set = true;
                    for hitbox in self.hitboxes.iter().filter_map(|x| x.clone()) {
                        if let CollisionBoxValues::Hit(hitbox) = hitbox.values
                            && hitbox.set_id == args.set_id
                        {
                            empty_set = false;
                        }
                    }
                    if empty_set
                        && let Some(rehit) = self.hitbox_sets_rehit.get_mut(args.set_id as usize)
                    {
                        *rehit = true;
                    }

                    let damage = self.get_float_value(&args.damage);
                    self.hitboxes[args.hitbox_id as usize] =
                        Some(ScriptCollisionBox::from_hitbox(args, interpolate, damage));
                } else {
                    error!(
                        "invalid hitbox index {} {}",
                        args.hitbox_id, self.subaction_name
                    );
                }
            }
            EventAst::ThrownHitBox(_) => {}
            EventAst::CreateSpecialHitBox(args) => {
                if args.hitbox_args.hitbox_id < self.hitboxes.len() as u8 {
                    // Need to check this here, as hitboxes can be deleted in the current frame but still exist in the previous frame.
                    // In this case interpolation should not occur.
                    let index = args.hitbox_args.hitbox_id as usize;
                    let interpolate = if let Some(prev_hitbox) = &self.hitboxes[index] {
                        match &prev_hitbox.values {
                            CollisionBoxValues::Hit(prev_hitbox) => {
                                args.hitbox_args.set_id == prev_hitbox.set_id
                            }
                            _ => false,
                        }
                    } else {
                        false
                    };

                    // Force rehit if no existing hitboxes.
                    // Both DeleteAllHitboxes and DeleteHitbox on the last hitbox will trigger rehit.
                    // http://localhost:8000/PM3.6/Squirtle/subactions/SpecialHi.html
                    let mut empty_set = true;
                    for hitbox in self.hitboxes.iter().filter_map(|x| x.clone()) {
                        if let CollisionBoxValues::Hit(hitbox) = hitbox.values
                            && hitbox.set_id == args.hitbox_args.set_id
                        {
                            empty_set = false;
                        }
                    }
                    if empty_set
                        && let Some(rehit) = self
                            .hitbox_sets_rehit
                            .get_mut(args.hitbox_args.set_id as usize)
                    {
                        *rehit = true;
                    }

                    let damage = self.get_float_value(&args.hitbox_args.damage);
                    self.hitboxes[index] = Some(ScriptCollisionBox::from_special_hitbox(
                        args,
                        interpolate,
                        damage,
                    ));
                } else {
                    error!(
                        "invalid hitbox index {} {}",
                        args.hitbox_args.hitbox_id, self.subaction_name
                    );
                }
            }
            EventAst::DeleteAllHitBoxes => {
                for hitbox in self.hitboxes.iter_mut() {
                    if hitbox.as_ref().map(|x| x.is_hit()).unwrap_or(false) {
                        *hitbox = None;
                    }
                }
            }
            EventAst::DefensiveCollision { .. } => {}
            EventAst::MoveHitBox(move_hitbox) => {
                if let Some(hitbox) = &mut self.hitboxes[move_hitbox.hitbox_id as usize] {
                    hitbox.bone_index = move_hitbox.new_bone as i16;
                    hitbox.x_offset = move_hitbox.new_x_offset;
                    hitbox.y_offset = move_hitbox.new_y_offset;
                    hitbox.z_offset = move_hitbox.new_z_offset;
                }
            }
            EventAst::ChangeHitBoxDamage {
                hitbox_id,
                new_damage,
            } => {
                if let Some(hitbox) = &mut self.hitboxes[*hitbox_id as usize]
                    && let CollisionBoxValues::Hit(hitbox) = &mut hitbox.values
                {
                    hitbox.damage = *new_damage as f32;
                }
            }
            EventAst::ChangeHitBoxSize {
                hitbox_id,
                new_size,
            } => {
                if let Some(hitbox) = &mut self.hitboxes[*hitbox_id as usize]
                    && let CollisionBoxValues::Hit(hitbox) = &mut hitbox.values
                {
                    hitbox.size = *new_size as f32;
                }
            }
            EventAst::DeleteHitBox(hitbox_id) => {
                // Shock claims this doesnt work on special hitboxes but it seems to work fine here:
                // http://localhost:8000/PM3.6/Squirtle/subactions/SpecialHi.html
                if self.hitboxes[*hitbox_id as usize]
                    .as_ref()
                    .map(|x| x.is_hit())
                    .unwrap_or(false)
                {
                    self.hitboxes[*hitbox_id as usize] = None;
                }
            }
            EventAst::CreateGrabBox(args) => {
                let mut interpolate = false;
                if let Some(prev_hitbox) = &self.hitboxes[args.hitbox_id as usize] {
                    // TODO: Should a grabbox interpolate from an existing hitbox and vice versa
                    if let CollisionBoxValues::Grab(_) = prev_hitbox.values {
                        interpolate = true;
                    }
                }
                if args.hitbox_id < self.hitboxes.len() as i32 {
                    self.hitboxes[args.hitbox_id as usize] =
                        Some(ScriptCollisionBox::from_grabbox(args, interpolate));
                } else {
                    error!(
                        "invalid hitbox index {} {}",
                        args.hitbox_id, self.subaction_name
                    );
                }
            }
            EventAst::DeleteGrabBox(hitbox_id) => {
                if self.hitboxes[*hitbox_id as usize]
                    .as_ref()
                    .map(|x| x.is_grab())
                    .unwrap_or(false)
                {
                    self.hitboxes[*hitbox_id as usize] = None;
                }
            }
            EventAst::DeleteAllGrabBoxes => {
                for hitbox in self.hitboxes.iter_mut() {
                    if hitbox.as_ref().map(|x| x.is_grab()).unwrap_or(false) {
                        *hitbox = None;
                    }
                }
            }
            EventAst::SpecifyThrow(throw) => {
                match &throw.throw_use {
                    ThrowUse::Throw => {
                        self.throw = Some(throw.clone());
                    }
                    ThrowUse::GrabInterrupt => {
                        // The only known value in the specifier that affects the grab interrupt is damage
                        self.grab_interrupt_damage = Some(throw.damage);
                    }
                    ThrowUse::Unknown(_) => {}
                }
            }
            EventAst::ApplyThrow(_) => {
                self.throw_activate = true;
            }
            EventAst::AddHitBoxDamage {
                hitbox_id,
                add_damage,
            } => {
                let add_damage = self.get_float_value(add_damage);
                if let Some(hitbox) = &mut self.hitboxes[*hitbox_id as usize]
                    && let CollisionBoxValues::Hit(hitbox) = &mut hitbox.values
                {
                    hitbox.damage += add_damage;
                }
            }

            // hurtboxes
            EventAst::ChangeHurtBoxStateAll { state } => {
                self.hurtbox_state_all = state.clone();
            }
            EventAst::ChangeHurtBoxStateSpecific { bone, state } => {
                match state {
                    HurtBoxState::Invincible => {
                        // Setting HurtBoxState::Invincible state with this command is broken.
                        // It either has no effect or sets the state to Normal.
                        // I havent confirmed which, so the current implementation just does nothing.
                    }
                    state => {
                        self.hurtbox_states
                            .insert(high_level_fighter::get_bone_index(*bone), state.clone());
                    }
                }
            }
            EventAst::UnchangeHurtBoxStateSpecific => {
                self.hurtbox_states.clear();
            }

            // controller
            EventAst::ControllerClearBuffer => {}
            EventAst::ControllerUnk01 => {}
            EventAst::ControllerUnk02 => {}
            EventAst::ControllerUnk06(_) => {}
            EventAst::ControllerUnk0C => {}
            EventAst::Rumble { unk1, unk2 } => self.rumble = Some((*unk1, *unk2)),
            EventAst::RumbleLoop { unk1, unk2 } => self.rumble_loop = Some((*unk1, *unk2)),

            // misc
            EventAst::SlopeContourStand { leg_bone_parent } => {
                if *leg_bone_parent == 0 {
                    self.slope_contour_stand = None;
                } else {
                    self.slope_contour_stand = Some(*leg_bone_parent);
                }
            }
            EventAst::SlopeContourFull {
                hip_n_or_top_n,
                trans_bone,
            } => {
                if *hip_n_or_top_n == 0 && *trans_bone == 0 {
                    self.slope_contour_full = None;
                } else {
                    self.slope_contour_full = Some((*hip_n_or_top_n, *trans_bone));
                }
            }
            EventAst::GenerateArticle { .. } => {}
            EventAst::ArticleEvent(_) => {}
            EventAst::ArticleAnimation(_) => {}
            EventAst::ArticleRemove(_) => {}
            EventAst::ArticleVisibility { .. } => {}
            EventAst::FinalSmashEnter => {}
            EventAst::FinalSmashExit => {}
            EventAst::TerminateSelf => {}
            EventAst::Posture(_) => {}
            EventAst::LedgeGrabEnable(enable) => {
                self.ledge_grab_enable = enable.clone();
            }
            EventAst::TagDisplay(display) => {
                self.tag_display = *display;
            }
            EventAst::Armor {
                armor_type,
                tolerance,
            } => {
                self.armor_type = armor_type.clone();
                self.armor_tolerance = *tolerance;
            }
            EventAst::AddDamage(damage) => {
                self.damage += damage;
            }
            EventAst::SetOrAddVelocity(values) => {
                if values.x_set {
                    self.x_vel = values.x_vel;
                    self.x_vel_modify = VelModify::Set(values.x_vel);
                } else {
                    self.x_vel += values.x_vel;
                    self.x_vel_modify = VelModify::Add(self.x_vel_modify.value() + values.x_vel);
                }

                if values.y_set {
                    self.y_vel = values.y_vel;
                    self.y_vel_modify = VelModify::Set(values.y_vel);
                } else {
                    self.y_vel += values.y_vel;
                    self.y_vel_modify = VelModify::Add(self.y_vel_modify.value() + values.y_vel);
                }
            }
            EventAst::SetVelocity { x_vel, y_vel } => {
                self.x_vel = *x_vel;
                self.y_vel = *y_vel;

                self.x_vel_modify = VelModify::Set(*x_vel);
                self.y_vel_modify = VelModify::Set(*y_vel);
            }
            EventAst::AddVelocity { x_vel, y_vel } => {
                let x_vel = self.get_float_value(x_vel);
                let y_vel = self.get_float_value(y_vel);

                self.x_vel += x_vel;
                self.y_vel += y_vel;

                self.x_vel_modify = VelModify::Add(self.x_vel_modify.value() + x_vel);
                self.y_vel_modify = VelModify::Add(self.y_vel_modify.value() + y_vel);
            }
            EventAst::DisableMovement(disable_movement) => {
                self.disable_movement = disable_movement.clone();
            }
            EventAst::DisableMovement2(_) => {} // TODO: What!?!?
            EventAst::ResetVerticalVelocityAndAcceleration(reset) => {
                if *reset {
                    self.y_vel = 0.0;

                    self.y_vel_modify = VelModify::Set(0.0);
                }
            }
            EventAst::NormalizePhysics => {} // TODO

            // sound
            EventAst::SoundEffect1(_) => {}
            EventAst::SoundEffect2(_) => {}
            EventAst::SoundEffectTransient(_) => {}
            EventAst::SoundEffectStop(_) => {}
            EventAst::SoundEffectVictory(_) => {}
            EventAst::SoundEffectUnk(_) => {}
            EventAst::SoundEffectOther1(_) => {}
            EventAst::SoundEffectOther2(_) => {}
            EventAst::SoundVoiceLow => {}
            EventAst::SoundVoiceDamage => {}
            EventAst::SoundVoiceOttotto => {}
            EventAst::SoundVoiceEating => {}

            // variables
            EventAst::IntVariableSet { value, variable } => {
                self.set_variable_int(variable, *value);
            }
            EventAst::IntVariableAdd { value, variable } => {
                let old_value = self.get_variable_int(variable);
                self.set_variable_int(variable, old_value + *value);
            }
            EventAst::IntVariableSubtract { value, variable } => {
                let old_value = self.get_variable_int(variable);
                self.set_variable_int(variable, old_value - value);
            }
            EventAst::IntVariableIncrement { variable } => {
                let old_value = self.get_variable_int(variable);
                self.set_variable_int(variable, old_value + 1);
            }
            EventAst::IntVariableDecrement { variable } => {
                let old_value = self.get_variable_int(variable);
                self.set_variable_int(variable, old_value - 1);
            }
            EventAst::FloatVariableSet { value, variable } => {
                let value = self.get_float_value(value);
                self.set_variable_float(variable, value);
            }
            EventAst::FloatVariableAdd { value, variable } => {
                let value = self.get_float_value(value);
                let old_value = self.get_variable_float(variable);
                self.set_variable_float(variable, old_value + value);
            }
            EventAst::FloatVariableSubtract { value, variable } => {
                let value = self.get_float_value(value);
                let old_value = self.get_variable_float(variable);
                self.set_variable_float(variable, old_value - value);
            }
            EventAst::FloatVariableMultiply { value, variable } => {
                let value = self.get_float_value(value);
                let old_value = self.get_variable_float(variable);
                self.set_variable_float(variable, old_value * value);
            }
            EventAst::FloatVariableDivide { value, variable } => {
                let value = self.get_float_value(value);
                let old_value = self.get_variable_float(variable);
                self.set_variable_float(variable, old_value / value);
            }
            EventAst::BoolVariableSetTrue { variable } => match variable.data_type() {
                VariableDataType::Int => self.set_variable_int_inner(variable, 1),
                VariableDataType::Float => self.set_variable_float_inner(variable, 1.0),
                VariableDataType::Bool => self.set_variable_bool_inner(variable, true),
                VariableDataType::Unknown { .. } => {}
            },
            EventAst::BoolVariableSetFalse { variable } => match variable.data_type() {
                VariableDataType::Int => self.set_variable_int_inner(variable, 0),
                VariableDataType::Float => self.set_variable_float_inner(variable, 0.0),
                VariableDataType::Bool => self.set_variable_bool_inner(variable, false),
                VariableDataType::Unknown { .. } => {}
            },

            // graphics
            EventAst::GraphicEffect(_) => {}
            EventAst::ExternalGraphicEffect(_) => {}
            EventAst::LimitedScreenTint(_) => {}
            EventAst::UnlimitedScreenTint(_) => {}
            EventAst::EndUnlimitedScreenTint { .. } => {}
            EventAst::SwordGlow(_) => {}
            EventAst::DeleteSwordGlow { .. } => {}
            EventAst::AestheticWindEffect(_) => {}
            EventAst::EndAestheticWindEffect { .. } => {}
            EventAst::ScreenShake { .. } => {}
            //EventAst::ModelChanger { reference, switch_index, bone_group_index } => {
            EventAst::ModelChanger { .. } => {
                // TODO: Model visibility change
            }
            EventAst::CameraCloseup(_) => {}
            EventAst::CameraNormal => {}
            EventAst::RemoveFlashEffect => {}
            EventAst::FlashEffectOverlay { .. } => {}
            EventAst::SetColorOfFlashEffectOverlay { .. } => {}
            EventAst::FlashEffectLight { .. } => {}
            EventAst::SetColorOfFlashEffectLight { .. } => {}

            // items
            EventAst::ItemPickup { .. } => {}
            EventAst::ItemThrow { .. } => {}
            EventAst::ItemThrow2 { .. } => {}
            EventAst::ItemDrop => {}
            EventAst::ItemConsume { .. } => {}
            EventAst::ItemSetProperty { .. } => {}
            EventAst::FireWeapon => {}
            EventAst::FireProjectile => {}
            EventAst::Item1F { .. } => {}
            EventAst::ItemCreate { .. } => {}
            EventAst::ItemVisibility(_) => {}
            EventAst::ItemDelete => {}
            EventAst::BeamSwordTrail { .. } => {}

            // do nothing
            EventAst::Unknown(event) => {
                debug!("unknown event: {:#?}", event);
            }
            EventAst::Nop => {}
        }
        StepEventResult::None
    }

    #[rustfmt::skip]
    fn evaluate_expression(&mut self, expression: &Expression) -> ExprResult {
        match expression {
            Expression::Nullary (requirement) => {
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
            Expression::Unary (unary) => {
                ExprResult::Bool (match unary.requirement {
                    Requirement::CharacterExists => true,
                    Requirement::OnGround => true,
                    Requirement::InAir => false,
                    Requirement::FacingRight => true,
                    Requirement::HasntTethered3Times => true,
                    Requirement::IsNotInDamagingLens => true,
                    Requirement::BoolIsTrue => self.evaluate_expression(&unary.value).unwrap_bool(),
                    _ => false
                })
            }
            #[allow(clippy::float_cmp)]
            Expression::Binary (binary) => {
                let result = match &binary.operator {
                    ComparisonOperator::LessThan => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          < right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) < right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          < right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          < right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    ComparisonOperator::LessThanOrEqual => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          <= right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) <= right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          <= right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          <= right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    ComparisonOperator::Equal => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          == right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) == right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          == right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          == right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    ComparisonOperator::NotEqual => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          != right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) != right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          != right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          != right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    ComparisonOperator::GreaterThanOrEqual => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          >= right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) >= right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          >= right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          >= right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    ComparisonOperator::GreaterThan => {
                        match (self.evaluate_expression(&binary.left), self.evaluate_expression(&binary.right)) {
                            (ExprResult::Float (left), ExprResult::Float (right)) => left          > right,
                            (ExprResult::Int (left),   ExprResult::Float (right)) => (left as f32) > right,
                            (ExprResult::Float (left), ExprResult::Int (right))   => left          > right as f32,
                            (ExprResult::Int (left),   ExprResult::Int (right))   => left          > right,
                            _ => panic!("Cannot evaluate expression: {:?}", binary),
                        }
                    }
                    ComparisonOperator::Or                 => self.evaluate_expression(&binary.left).unwrap_bool() || self.evaluate_expression(&binary.right).unwrap_bool(),
                    ComparisonOperator::And                => self.evaluate_expression(&binary.left).unwrap_bool() && self.evaluate_expression(&binary.right).unwrap_bool(),
                    ComparisonOperator::UnknownArg (_)     => false,
                };
                ExprResult::Bool (result)
            }
            Expression::Not (expression) => ExprResult::Bool (!self.evaluate_expression(expression).unwrap_bool()),
            Expression::Variable (variable) => {
                match variable.data_type() {
                    VariableDataType::Int            => ExprResult::Int   (self.get_variable_int_inner(variable)),
                    VariableDataType::Float          => ExprResult::Float (self.get_variable_float_inner(variable)),
                    VariableDataType::Bool           => ExprResult::Bool  (self.get_variable_bool_inner(variable)),
                    VariableDataType::Unknown { .. } => ExprResult::Bool (false), // can't be handled properly so just give a dummy value
                }
            }
            Expression::Value (int) => ExprResult::Int (*int),
            Expression::Scalar (float) => ExprResult::Float (*float),
        }
    }

    fn get_variable_int(&self, variable: &VariableAst) -> i32 {
        match variable.data_type() {
            VariableDataType::Int => self.get_variable_int_inner(variable),
            VariableDataType::Float => self.get_variable_float_inner(variable) as i32,
            VariableDataType::Bool => self.get_variable_bool_inner(variable) as i32,
            VariableDataType::Unknown { .. } => 0,
        }
    }

    fn get_variable_float(&self, variable: &VariableAst) -> f32 {
        match variable.data_type() {
            VariableDataType::Int => self.get_variable_int_inner(variable) as f32,
            VariableDataType::Float => self.get_variable_float_inner(variable),
            VariableDataType::Bool => {
                if self.get_variable_bool_inner(variable) {
                    1.0
                } else {
                    0.0
                }
            }
            VariableDataType::Unknown { .. } => 0.0,
        }
    }

    fn get_float_value(&self, value: &FloatValue) -> f32 {
        match value {
            FloatValue::Constant(value) => *value,
            FloatValue::Variable(variable) => self.get_variable_float(variable),
        }
    }

    fn set_variable_int(&mut self, variable: &VariableAst, value: i32) {
        match variable.data_type() {
            VariableDataType::Int => self.set_variable_int_inner(variable, value),
            VariableDataType::Float => self.set_variable_float_inner(variable, value as f32),
            VariableDataType::Bool => self.set_variable_bool_inner(variable, value != 0),
            VariableDataType::Unknown { .. } => {}
        }
    }

    fn set_variable_float(&mut self, variable: &VariableAst, value: f32) {
        match variable.data_type() {
            VariableDataType::Int => self.set_variable_int_inner(variable, value as i32),
            VariableDataType::Float => self.set_variable_float_inner(variable, value),
            VariableDataType::Bool => self.set_variable_bool_inner(variable, value != 0.0),
            VariableDataType::Unknown { .. } => {}
        }
    }

    fn get_variable_int_inner(&self, variable: &VariableAst) -> i32 {
        match variable {
            VariableAst::InternalConstantInt(InternalConstantInt::CurrentFrame) => {
                self.frame_index as i32
            }
            VariableAst::InternalConstantInt(InternalConstantInt::CharacterDirection) => 1,
            VariableAst::InternalConstantInt(InternalConstantInt::CharacterDirectionOpposite) => -1,
            VariableAst::InternalConstantInt(InternalConstantInt::CurrentFrameSpeed) => {
                self.frame_speed_modifier as i32
            }
            VariableAst::InternalConstantInt(InternalConstantInt::CurrentSubaction) => 0, // TODO: Get this passed as an argument to ScriptRunner::new
            VariableAst::InternalConstantInt(InternalConstantInt::CurrentAction) => 0, // TODO: Get this passed as an argument to ScriptRunner::new
            VariableAst::InternalConstantInt(InternalConstantInt::CrawlControlStickXOffsetMax) => {
                20
            } // TODO: probably character dependent?
            VariableAst::InternalConstantInt(InternalConstantInt::CrawlControlStickXOffsetMin) => {
                -20
            } // TODO: probably character dependent?
            VariableAst::InternalConstantInt(_) => 0, // Best we can do for everything else is 0
            VariableAst::LongtermAccessInt(LongtermAccessInt::JumpsUsed) => self.jumps_used,
            VariableAst::LongtermAccessInt(LongtermAccessInt::WallJumpCount) => {
                self.wall_jump_count
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::WallJumpInterval) => {
                self.wall_jump_interval
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::FootstoolCount) => {
                self.footstool_count
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::FallTime) => self.fall_time,
            VariableAst::LongtermAccessInt(LongtermAccessInt::SwimTime) => self.swim_time,
            VariableAst::LongtermAccessInt(LongtermAccessInt::LipStickRefresh) => {
                self.lip_stick_refresh
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::CurryRemainingTime) => {
                self.curry_remaining_time
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::CurryAngle2) => self.curry_angle2,
            VariableAst::LongtermAccessInt(LongtermAccessInt::StarRemainingTime) => {
                self.star_remaining_time
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::MushroomRemainingTime) => {
                self.mushroom_remaining_time
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::LightningRemainingTime) => {
                self.lightning_remaining_time
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::SizeFlag) => self.size_flag,
            VariableAst::LongtermAccessInt(LongtermAccessInt::MetalBlockRemainingTime) => {
                self.metal_block_remaining_time
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::ComboCount) => self.combo_count,
            VariableAst::LongtermAccessInt(LongtermAccessInt::BubbleTime) => self.bubble_time,
            VariableAst::LongtermAccessInt(LongtermAccessInt::AttacksPerformed) => {
                self.attacks_performed
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::CostumeID) => self.costume_id,
            VariableAst::LongtermAccessInt(LongtermAccessInt::HitstunFramesRemaining) => {
                self.hitstun_frames_remaining
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::MeteorCancelWindow) => {
                self.meteor_cancel_window
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::MissedTechs) => self.missed_techs,
            VariableAst::LongtermAccessInt(LongtermAccessInt::TetherCount) => self.tether_count,
            VariableAst::LongtermAccessInt(LongtermAccessInt::Temp1) => self.temp1,
            VariableAst::LongtermAccessInt(LongtermAccessInt::Temp2) => self.temp2,
            VariableAst::LongtermAccessInt(LongtermAccessInt::Address(address)) => self
                .longterm_access_int
                .get(*address as usize)
                .cloned()
                .unwrap_or(0),
            VariableAst::RandomAccessInt(RandomAccessInt::ThrowDataParam1) => {
                self.throw_data_param1
            }
            VariableAst::RandomAccessInt(RandomAccessInt::ThrowDataParam2) => {
                self.throw_data_param2
            }
            VariableAst::RandomAccessInt(RandomAccessInt::ThrowDataParam3) => {
                self.throw_data_param3
            }
            VariableAst::RandomAccessInt(RandomAccessInt::Address(address)) => self
                .random_access_int
                .get(*address as usize)
                .cloned()
                .unwrap_or(0),
            VariableAst::Unknown { .. } => 0, // Likely from garbage data

            VariableAst::LongtermAccessFloat(_)
            | VariableAst::LongtermAccessBool(_)
            | VariableAst::RandomAccessFloat(_)
            | VariableAst::RandomAccessBool(_) => panic!(
                "Called get_variable_int on a variable that is not an int. '{:?}' It is a brawllib_rs logic error if this is reached",
                variable
            ),
        }
    }

    fn set_variable_int_inner(&mut self, variable: &VariableAst, value: i32) {
        match variable {
            VariableAst::InternalConstantInt(_) => {} // Cant set a constant
            VariableAst::LongtermAccessInt(LongtermAccessInt::JumpsUsed) => self.jumps_used = value,
            VariableAst::LongtermAccessInt(LongtermAccessInt::WallJumpCount) => {
                self.wall_jump_count = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::WallJumpInterval) => {
                self.wall_jump_interval = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::FootstoolCount) => {
                self.footstool_count = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::FallTime) => self.fall_time = value,
            VariableAst::LongtermAccessInt(LongtermAccessInt::SwimTime) => self.swim_time = value,
            VariableAst::LongtermAccessInt(LongtermAccessInt::LipStickRefresh) => {
                self.lip_stick_refresh = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::CurryRemainingTime) => {
                self.curry_remaining_time = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::CurryAngle2) => {
                self.curry_angle2 = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::StarRemainingTime) => {
                self.star_remaining_time = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::MushroomRemainingTime) => {
                self.mushroom_remaining_time = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::LightningRemainingTime) => {
                self.lightning_remaining_time = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::SizeFlag) => self.size_flag = value,
            VariableAst::LongtermAccessInt(LongtermAccessInt::MetalBlockRemainingTime) => {
                self.metal_block_remaining_time = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::ComboCount) => {
                self.combo_count = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::BubbleTime) => {
                self.bubble_time = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::AttacksPerformed) => {
                self.attacks_performed = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::CostumeID) => self.costume_id = value,
            VariableAst::LongtermAccessInt(LongtermAccessInt::HitstunFramesRemaining) => {
                self.hitstun_frames_remaining = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::MeteorCancelWindow) => {
                self.meteor_cancel_window = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::MissedTechs) => {
                self.missed_techs = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::TetherCount) => {
                self.tether_count = value
            }
            VariableAst::LongtermAccessInt(LongtermAccessInt::Temp1) => self.temp1 = value,
            VariableAst::LongtermAccessInt(LongtermAccessInt::Temp2) => self.temp2 = value,
            VariableAst::LongtermAccessInt(LongtermAccessInt::Address(address)) => {
                if (*address as usize) < self.longterm_access_int.len() {
                    self.longterm_access_int[*address as usize] = value;
                }
            }

            VariableAst::RandomAccessInt(RandomAccessInt::ThrowDataParam1) => {
                self.throw_data_param1 = value
            }
            VariableAst::RandomAccessInt(RandomAccessInt::ThrowDataParam2) => {
                self.throw_data_param2 = value
            }
            VariableAst::RandomAccessInt(RandomAccessInt::ThrowDataParam3) => {
                self.throw_data_param3 = value
            }
            VariableAst::RandomAccessInt(RandomAccessInt::Address(address)) => {
                if (*address as usize) < self.random_access_int.len() {
                    self.random_access_int[*address as usize] = value;
                }
            }

            VariableAst::Unknown { .. } => {} // Likely from garbage data

            VariableAst::LongtermAccessFloat(_)
            | VariableAst::LongtermAccessBool(_)
            | VariableAst::RandomAccessFloat(_)
            | VariableAst::RandomAccessBool(_) => panic!(
                "Called set_variable_int_inner on a variable that is not an int. '{:?}' It is a brawllib_rs logic error if this is reached.",
                variable
            ),
        }
    }

    fn get_variable_float_inner(&self, variable: &VariableAst) -> f32 {
        match variable {
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::SpecialLandingLag) => {
                self.special_landing_lag
            }
            VariableAst::LongtermAccessFloat(
                LongtermAccessFloat::SpecialFallMobilityMultiplier,
            ) => self.special_fall_mobility_multiplier,
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::ShieldCharge) => {
                self.shield_charge
            }
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::CurryAngle1) => self.curry_angle1,
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::CurryRandomness) => {
                self.curry_randomness
            }
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::Address(address)) => self
                .longterm_access_float
                .get(*address as usize)
                .cloned()
                .unwrap_or(0.0),
            VariableAst::RandomAccessFloat(RandomAccessFloat::EnableTurnWhenBelowZero) => {
                self.enable_turn_when_below_zero
            }
            VariableAst::RandomAccessFloat(RandomAccessFloat::Address(address)) => self
                .random_access_float
                .get(*address as usize)
                .cloned()
                .unwrap_or(0.0),
            VariableAst::Unknown { .. } => 0.0, // Likely from garbage data

            VariableAst::LongtermAccessInt(_)
            | VariableAst::LongtermAccessBool(_)
            | VariableAst::RandomAccessInt(_)
            | VariableAst::RandomAccessBool(_)
            | VariableAst::InternalConstantInt(_) => panic!(
                "Called get_variable_float on a variable that is not a float. '{:?}' It is a brawllib_rs logic error if this is reached.",
                variable
            ),
        }
    }

    fn set_variable_float_inner(&mut self, variable: &VariableAst, value: f32) {
        match variable {
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::SpecialLandingLag) => {
                self.special_landing_lag = value
            }
            VariableAst::LongtermAccessFloat(
                LongtermAccessFloat::SpecialFallMobilityMultiplier,
            ) => self.special_fall_mobility_multiplier = value,
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::ShieldCharge) => {
                self.shield_charge = value
            }
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::CurryAngle1) => {
                self.curry_angle1 = value
            }
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::CurryRandomness) => {
                self.curry_randomness = value
            }
            VariableAst::LongtermAccessFloat(LongtermAccessFloat::Address(address)) => {
                if (*address as usize) < self.longterm_access_float.len() {
                    self.longterm_access_float[*address as usize] = value;
                }
            }
            VariableAst::RandomAccessFloat(RandomAccessFloat::EnableTurnWhenBelowZero) => {
                self.enable_turn_when_below_zero = value
            }
            VariableAst::RandomAccessFloat(RandomAccessFloat::Address(address)) => {
                if (*address as usize) < self.random_access_float.len() {
                    self.random_access_float[*address as usize] = value;
                }
            }
            VariableAst::Unknown { .. } => {} // Likely from garbage data

            VariableAst::LongtermAccessInt(_)
            | VariableAst::LongtermAccessBool(_)
            | VariableAst::RandomAccessInt(_)
            | VariableAst::RandomAccessBool(_)
            | VariableAst::InternalConstantInt(_) => panic!(
                "Called set_variable_float_inner on a variable that is not a float. '{:?}' It is a brawllib_rs logic error if this is reached.",
                variable
            ),
        }
    }

    fn get_variable_bool_inner(&self, variable: &VariableAst) -> bool {
        match variable {
            VariableAst::LongtermAccessBool(LongtermAccessBool::IsDead) => self.is_dead,
            VariableAst::LongtermAccessBool(LongtermAccessBool::CannotDie) => self.cannot_die,
            VariableAst::LongtermAccessBool(LongtermAccessBool::AutomaticFootstool) => {
                self.automatic_footstool
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasFinal) => self.has_final,
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasFinalAura) => {
                self.has_final_aura
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasCurry) => self.has_curry,
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasHammer) => self.has_hammer,
            VariableAst::LongtermAccessBool(LongtermAccessBool::HitByParalyze) => {
                self.hit_by_paralyze
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasScrewAttack) => {
                self.has_screw_attack
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::StaminaDead) => self.stamina_dead,
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasTag) => self.has_tag,
            VariableAst::LongtermAccessBool(LongtermAccessBool::CanNotLedgeGrab) => {
                self.can_not_ledge_grab
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::CanNotTeeter) => {
                self.can_not_teeter
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::VelocityIgnoreHitstun) => {
                self.velocity_ignore_hitstun
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::Deflection) => self.deflection,
            VariableAst::LongtermAccessBool(LongtermAccessBool::Address(address)) => self
                .longterm_access_bool
                .get(*address as usize)
                .cloned()
                .unwrap_or(false),

            VariableAst::RandomAccessBool(RandomAccessBool::CharacterFloat) => self.character_float,
            VariableAst::RandomAccessBool(RandomAccessBool::EnableFastFall) => {
                self.enable_fast_fall
            }
            VariableAst::RandomAccessBool(RandomAccessBool::Shorthop) => self.shorthop,
            VariableAst::RandomAccessBool(RandomAccessBool::EnableActionTransition) => {
                self.enable_action_transition
            }
            VariableAst::RandomAccessBool(RandomAccessBool::SpecialsMovement) => {
                self.specials_movement
            }
            VariableAst::RandomAccessBool(RandomAccessBool::EnableGlide) => self.enable_glide,
            VariableAst::RandomAccessBool(RandomAccessBool::EnableJabLoop) => self.enable_jab_loop,
            VariableAst::RandomAccessBool(RandomAccessBool::EnableAutoJab) => self.enable_auto_jab,
            VariableAst::RandomAccessBool(RandomAccessBool::EnableJabEnd) => self.enable_jab_end,
            VariableAst::RandomAccessBool(RandomAccessBool::EnableLandingLag) => self.landing_lag,
            VariableAst::RandomAccessBool(RandomAccessBool::Address(address)) => self
                .random_access_bool
                .get(*address as usize)
                .cloned()
                .unwrap_or(false),
            VariableAst::Unknown { .. } => false, // Likely from garbage data

            VariableAst::LongtermAccessInt(_)
            | VariableAst::LongtermAccessFloat(_)
            | VariableAst::RandomAccessInt(_)
            | VariableAst::RandomAccessFloat(_)
            | VariableAst::InternalConstantInt(_) => panic!(
                "Called get_variable_bool on a variable that is not a bool. '{:?}' It is a brawllib_rs logic error if this is reached.",
                variable
            ),
        }
    }

    fn set_variable_bool_inner(&mut self, variable: &VariableAst, value: bool) {
        match variable {
            VariableAst::LongtermAccessBool(LongtermAccessBool::IsDead) => self.is_dead = value,
            VariableAst::LongtermAccessBool(LongtermAccessBool::CannotDie) => {
                self.cannot_die = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::AutomaticFootstool) => {
                self.automatic_footstool = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasFinal) => self.has_final = value,
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasFinalAura) => {
                self.has_final_aura = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasCurry) => self.has_curry = value,
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasHammer) => {
                self.has_hammer = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::HitByParalyze) => {
                self.hit_by_paralyze = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasScrewAttack) => {
                self.has_screw_attack = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::StaminaDead) => {
                self.stamina_dead = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::HasTag) => self.has_tag = value,
            VariableAst::LongtermAccessBool(LongtermAccessBool::CanNotLedgeGrab) => {
                self.can_not_ledge_grab = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::CanNotTeeter) => {
                self.can_not_teeter = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::VelocityIgnoreHitstun) => {
                self.velocity_ignore_hitstun = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::Deflection) => {
                self.deflection = value
            }
            VariableAst::LongtermAccessBool(LongtermAccessBool::Address(address)) => {
                if (*address as usize) < self.longterm_access_bool.len() {
                    self.longterm_access_bool[*address as usize] = value;
                }
            }

            VariableAst::RandomAccessBool(RandomAccessBool::CharacterFloat) => {
                self.character_float = value
            }
            VariableAst::RandomAccessBool(RandomAccessBool::EnableFastFall) => {
                self.enable_fast_fall = value
            }
            VariableAst::RandomAccessBool(RandomAccessBool::Shorthop) => self.shorthop = value,
            VariableAst::RandomAccessBool(RandomAccessBool::EnableActionTransition) => {
                self.enable_action_transition = value
            }
            VariableAst::RandomAccessBool(RandomAccessBool::SpecialsMovement) => {
                self.specials_movement = value
            }
            VariableAst::RandomAccessBool(RandomAccessBool::EnableGlide) => {
                self.enable_glide = value
            }
            VariableAst::RandomAccessBool(RandomAccessBool::EnableJabLoop) => {
                self.enable_jab_loop = value
            }
            VariableAst::RandomAccessBool(RandomAccessBool::EnableAutoJab) => {
                self.enable_auto_jab = value
            }
            VariableAst::RandomAccessBool(RandomAccessBool::EnableJabEnd) => {
                self.enable_jab_end = value
            }
            VariableAst::RandomAccessBool(RandomAccessBool::EnableLandingLag) => {
                self.landing_lag = value
            }
            VariableAst::RandomAccessBool(RandomAccessBool::Address(address)) => {
                if (*address as usize) < self.random_access_bool.len() {
                    self.random_access_bool[*address as usize] = value;
                }
            }
            VariableAst::Unknown { .. } => {} // Likely from garbage data

            VariableAst::LongtermAccessInt(_)
            | VariableAst::LongtermAccessFloat(_)
            | VariableAst::RandomAccessInt(_)
            | VariableAst::RandomAccessFloat(_)
            | VariableAst::InternalConstantInt(_) => panic!(
                "Called set_variable_bool_inner on a variable that is not a bool. '{:?}' It is a brawllib_rs logic error if this is reached.",
                variable
            ),
        }
    }
}

enum StepEventResult<'a> {
    WaitUntil(f32),
    NewForLoop {
        block: &'a Block,
        iterations: i32,
    },
    NewCall {
        block: &'a Block,
    },
    NewIfStatement {
        then_branch: &'a Block,
        else_branch: Option<&'a Block>,
        execute: bool,
    },
    IfStatementDisableExecution,
    Goto {
        block: &'a Block,
        external: bool,
    },
    Subroutine {
        block: &'a Block,
        external: bool,
    },
    CallEveryFrame {
        block: &'a Block,
        external: bool,
        thread_id: i32,
    },
    Return,
    None,
}

pub struct CallEveryFrame<'a> {
    pub block: &'a Block,
    pub external: bool,
}

#[derive(Debug)]
enum ExprResult {
    Int(i32),
    Float(f32),
    Bool(bool),
}

impl ExprResult {
    fn unwrap_bool(&self) -> bool {
        match self {
            ExprResult::Bool(result) => *result,
            ExprResult::Int(value) => *value != 0,
            ExprResult::Float(value) => *value != 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VelModify {
    Set(f32),
    Add(f32),
    None,
}

impl VelModify {
    pub fn value(&self) -> f32 {
        match self {
            VelModify::Set(a) => *a,
            VelModify::Add(a) => *a,
            VelModify::None => 0.0,
        }
    }
}
