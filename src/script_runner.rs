use std::collections::HashMap;
use script_ast::{EventAst, HitBoxArguments, SpecialHitBoxArguments};
use high_level_fighter::HighLevelScripts;

pub struct ScriptRunner {
    pub variables: HashMap<i32, i32>,
    pub event_indexes: Vec<usize>,
    pub frame_index: f32,
    pub wait_until: Option<f32>,
    pub interruptible: bool,
    pub hitboxes: Vec<HitBoxArguments>,
    pub special_hitboxes: Vec<SpecialHitBoxArguments>,
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
        }
    }

    pub fn step(&mut self, scripts: &Option<HighLevelScripts>) {
        self.frame_index += 1.0;

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
                &EventAst::ChangeSubAction (_) => { }
                &EventAst::ChangeSubActionRestartFrame (_) => { }
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

pub struct HitBox { }
