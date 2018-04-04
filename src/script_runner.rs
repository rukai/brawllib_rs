use std::collections::HashMap;
use script_ast::{EventAst, HitBoxArguments, SpecialHitBoxArguments, EdgeSlide};
use high_level_fighter::HighLevelScripts;

pub struct ScriptRunner {
    pub variables: HashMap<i32, i32>,
    pub event_indexes: Vec<usize>,
    pub frame_index: f32,
    pub wait_until: Option<f32>,
    pub interruptible: bool,
    pub hitboxes: Vec<HitBoxArguments>,
    pub special_hitboxes: Vec<SpecialHitBoxArguments>,
    pub frame_speed_modifier: f32,
    pub airbourne: bool,
    pub edge_slide: EdgeSlide, // TODO: This value seems inaccurate as its rarely set, is ledge cancel normally just hardcoded for say movement vs attack
    pub change_sub_action: ChangeSubAction,
}

pub enum ChangeSubAction {
    Continue,
    InfiniteLoop,
    ChangeSubAction (i32),
    ChangeSubActionRestartFrame (i32),
}

impl ScriptRunner {
    pub fn new() -> ScriptRunner {
        ScriptRunner {
            variables: HashMap::new(),
            event_indexes: vec!(0),
            frame_index: 0.0,
            wait_until: None,
            interruptible: false,
            hitboxes: vec!(),
            special_hitboxes: vec!(),
            frame_speed_modifier: 1.0,
            airbourne: false,
            edge_slide: EdgeSlide::SlideOff,
            change_sub_action: ChangeSubAction::Continue,
        }
    }

    pub fn step(&mut self, scripts: &Option<HighLevelScripts>) {
        self.frame_index += self.frame_speed_modifier;

        if let Some(wait_until) = self.wait_until {
            if self.frame_index >= wait_until {
                self.wait_until = None;
            }
        }

        if self.wait_until.is_none() {
            if let &Some(ref scripts) = scripts {
                self.step_recursive(&scripts.script_main.events);
            }
        }

        if self.frame_speed_modifier == 0.0 {
            self.change_sub_action = ChangeSubAction::InfiniteLoop
        }
    }

    fn step_recursive(&mut self, events: &Vec<EventAst>) {
        let event_index = self.event_indexes.last_mut().unwrap();
        while let Some(event) = events.get(*event_index) {
            match event {
                &EventAst::SyncWait (ref value) => {
                    self.wait_until = Some(self.frame_index + *value);
                    *event_index += 1;
                    return;
                }
                &EventAst::AsyncWait (ref value) => {
                    self.wait_until = Some(*value);
                    *event_index += 1;
                    return;
                }
                &EventAst::SetLoop (_) => { }
                &EventAst::ExecuteLoop => { }
                &EventAst::Subroutine (_) => { }
                &EventAst::Return => { }
                &EventAst::Goto (_) => { }
                &EventAst::If (_) => { }
                &EventAst::IfValue (_, _) => { }
                &EventAst::IfComparison (_, _, _, _) => { }
                &EventAst::Else => { }
                &EventAst::AndComparison (_, _, _, _)=> { }
                &EventAst::ElseIfComparison (_, _, _, _)=> { }
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
                    self.hitboxes.push(args.clone());
                }
                &EventAst::RemoveAllHitBoxes => {
                    self.hitboxes.clear();
                }
                &EventAst::CreateSpecialHitBox (ref args) => {
                    self.special_hitboxes.push(args.clone());
                }
                &EventAst::AllowInterrupt => {
                    self.interruptible = true;
                }
                &EventAst::Unknown | &EventAst::Nop => { }
            }
            *event_index += 1;
        }
    }
}
