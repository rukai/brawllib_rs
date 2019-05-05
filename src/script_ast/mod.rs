use crate::script::{Script, Event, Requirement, Argument, Offset};
use crate::script;

use std::iter::Iterator;
use std::slice;
use std::f32;

pub mod variable_ast;

use variable_ast::VariableAst;

#[derive(Serialize, Clone, Debug)]
pub struct ScriptAst {
    pub block:  Block,
    pub offset: i32,
}

impl ScriptAst {
    pub fn new(script: &Script) -> ScriptAst {
        let block = if let ProcessedBlock::Finished(events) = process_block(&mut script.events.iter().peekable()) {
            events
        } else {
            error!("A block in the script did not terminate.");
            Block { events: vec!() }
        };

        ScriptAst {
            block,
            offset: script.offset
        }
    }
}

fn process_block(events: &mut std::iter::Peekable<slice::Iter<Event>>) -> ProcessedBlock {
    let mut event_asts = vec!();
    while let Some(event) = events.next() {
        let args = &event.arguments;
        use crate::script::Argument::*;
        let event_ast = match (event.namespace, event.code, args.get(0), args.get(1), args.get(2)) {
            (0x00, 0x01, Some(&Scalar(v0)), None, None) => EventAst::SyncWait (v0),
            (0x00, 0x02, None,              None, None) => EventAst::Nop,
            (0x00, 0x02, Some(&Scalar(v0)), None, None) => EventAst::AsyncWait (v0),
            (0x00, 0x04, Some(&Value(v0)),  None, None) => { // Loop
                let iterations = if v0 == -1 {
                    Iterations::Infinite
                } else {
                    Iterations::Finite (v0)
                };

                match process_block(events) {
                    ProcessedBlock::EndForLoop (block) => EventAst::ForLoop (ForLoop { iterations, block }),
                    _ => {
                        error!("ForLoop did not terminate");
                        EventAst::Unknown (event.clone())
                    }
                }
            }
            (0x00, 0x05, None, None, None) => { // End loop
                return ProcessedBlock::EndForLoop (Block { events: event_asts })
            }
            (0x00, 0x07, Some(&Offset(ref v0)), None, None) => EventAst::Subroutine (v0.clone()),
            (0x00, 0x07, Some(&Value (ref v0)), None, None) => EventAst::Subroutine (script::Offset { offset: *v0, origin: -1 }),
            (0x00, 0x08, None,                  None, None) => EventAst::Return,
            (0x00, 0x09, Some(&Offset(ref v0)), None, None) => EventAst::Goto (v0.clone()),
            (0x00, 0x09, Some(&Value (ref v0)), None, None) => EventAst::Goto (script::Offset { offset: *v0, origin: -1 }),
            (0x00, 0x0A, Some(&Requirement { ref ty, flip }), v1, v2) => { // If
                if let Some(test) = Expression::from_args(ty, flip, v1, v2, args.get(3)) {
                    match process_block(events) {
                        ProcessedBlock::EndIf { then_branch } => {
                            EventAst::IfStatement (IfStatement { test, then_branch, else_branch: None })
                        }
                        ProcessedBlock::EndIfAndElse { then_branch, else_branch } => {
                            EventAst::IfStatement (IfStatement { test, then_branch, else_branch })
                        }
                        _ => {
                            error!("IfStatement did not terminate");
                            EventAst::Unknown (event.clone())
                        }
                    }
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x00, 0x0E, None, None, None) => { // Else
                match process_block(events) {
                    ProcessedBlock::EndIf { then_branch: else_branch } => {
                        let then_branch = Block { events: event_asts };
                        let else_branch = Some(Box::new(else_branch));
                        return ProcessedBlock::EndIfAndElse { then_branch, else_branch }
                    }
                    _ => {
                        error!("IfStatement did not terminate");
                        EventAst::Unknown (event.clone())
                    }
                }
            }
            (0x00, 0x0B, Some(&Requirement { ref ty, flip }), v1, v2) => { // And
                // It is tempting to combine this event with the previous IfStatement event.
                // However that is a terrible idea, as an And/Or will alter the execution of the
                // current IfStatement even if other events have occured since the IfStatement.
                //
                // e.g. If an IfStatement branch is currently running and an `And` is ran that
                // means it would not have started running, execution of the block will end immediately.
                // All events before the `And` are still executed.
                // While all events after the `And` are not executed.
                //
                // I have tested this with Nop and FrameSpeedModifier events in between the IfStatement and And/Or.
                // It would probably also occur with an And/Or at a Goto/Subroutine destination
                if let Some(test) = Expression::from_args(ty, flip, v1, v2, args.get(3)) {
                    EventAst::IfStatementAnd (test)
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x00, 0x0C, Some(&Requirement { ref ty, flip }), v1, v2) => { // Or
                if let Some(test) = Expression::from_args(ty, flip, v1, v2, args.get(3)) {
                    EventAst::IfStatementOr (test)
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x00, 0x0D, Some(&Requirement { ref ty, flip }), v1, v2) => { // Else If
                if let Some(test) = Expression::from_args(ty, flip, v1, v2, args.get(3)) {
                    let event = match process_block(events) {
                        ProcessedBlock::EndIf { then_branch } => {
                            EventAst::IfStatement (IfStatement { test, then_branch, else_branch: None })
                        }
                        ProcessedBlock::EndIfAndElse { then_branch, else_branch } => {
                            EventAst::IfStatement (IfStatement { test, then_branch, else_branch })
                        }
                        _ => {
                            error!("IfStatement did not terminate");
                            return ProcessedBlock::Finished (Block { events: event_asts });
                        }
                    };
                    let else_branch = Some(Box::new(Block { events: vec!(event) }));
                    let then_branch = Block { events: event_asts };
                    return ProcessedBlock::EndIfAndElse { then_branch, else_branch };
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x00, 0x0F, None, None, None) => { return ProcessedBlock::EndIf { then_branch: Block { events: event_asts } } }
            (0x00, 0x10, Some(&Value(v0)), Some(&Value(v1)),      None) => EventAst::Switch (v0, v1),
            (0x00, 0x11, Some(&Value(v0)), None,                  None) => EventAst::Case (v0),
            (0x00, 0x11, None,             None,                  None) => EventAst::DefaultCase,
            (0x00, 0x13, None,             None,                  None) => EventAst::EndSwitch,
            (0x01, 0x01, None,             None,                  None) => EventAst::LoopRest,
            (0x0D, 0x00, Some(&Value(v0)), Some(&Offset(ref v1)), None) => EventAst::CallEveryFrame { thread_id: v0, offset: v1.clone() },
            (0x0D, 0x01, Some(&Value(v0)), None,                  None) => EventAst::RemoveCallEveryFrame { thread_id: v0 },

            // change action
            (0x02, 0x00, Some(&Value(v0)), Some(&Value(v1)), Some(&Requirement { ref ty, flip })) => {
                if let Some(test) = Expression::from_args(ty, flip, args.get(3), args.get(4), args.get(5)) {
                    EventAst::CreateInterrupt (Interrupt { interrupt_id: Some(v0), action: v1, test })
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x02, 0x01, Some(&Value(v0)), Some(&Requirement { ref ty, flip }), v2) => {
                if let Some(test) = Expression::from_args(ty, flip, v2, args.get(3), args.get(4)) {
                    EventAst::CreateInterrupt (Interrupt { interrupt_id: None, action: v0, test })
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x02, 0x04, Some(&Requirement { ref ty, flip }), v1, v2) => {
                // It is tempting to combine this event with the previous CreateInterrupt event.
                // However that is a terrible idea as a InterruptAddRequirement will
                // modify the last CreateInterrupt regardless of any events in between.
                // I have tested this with Nop and FrameSpeedModifier events in between.
                // It would probably also occur with a InterruptAddRequirement in an IfStatement, at a Goto/Subroutine
                if let Some(test) = Expression::from_args(ty, flip, v1, v2, args.get(3)) {
                    EventAst::PreviousInterruptAddRequirement { test }
                }
                else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x02, 0x05, Some(&Value(v0)), Some(&Value(v1)), Some(&Requirement { ref ty, flip })) => {
                if let Some(test) = Expression::from_args(ty, flip, args.get(3), args.get(4), args.get(5)) {
                    EventAst::InterruptAddRequirement { interrupt_type: InterruptType::new(v0), interrupt_id: v1, test }
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x02, 0x06, Some(&Value(v0)),  None,             None) => EventAst::EnableInterrupt (v0),
            (0x02, 0x08, Some(&Value(v0)),  None,             None) => EventAst::DisableInterrupt (v0),
            (0x02, 0x09, Some(&Value(v0)),  Some(&Value(v1)), None) => EventAst::ToggleInterrupt { interrupt_type: InterruptType::new(v0), interrupt_id: v1 },
            (0x02, 0x0A, Some(&Value(v0)),  None,             None) => EventAst::EnableInterruptGroup (InterruptType::new(v0)),
            (0x02, 0x0B, Some(&Value(v0)),  None,             None) => EventAst::DisableInterruptGroup (InterruptType::new(v0)),
            (0x02, 0x0C, Some(&Value(v0)),  None,             None) => EventAst::ClearInterruptGroup (InterruptType::new(v0)),
            (0x64, 0x00, None,              None,             None) => EventAst::AllowInterrupts,
            (0x04, 0x00, Some(&Value(v0)),  None,             None) => EventAst::ChangeSubactionRestartFrame (v0),
            (0x04, 0x00, Some(&Value(v0)),  Some(&Bool(v1)),  None) => if v1 { EventAst::ChangeSubaction (v0) } else { EventAst::ChangeSubactionRestartFrame (v0) }

            // timing
            (0x04, 0x06, Some(&Scalar(v0)), None,             None) => EventAst::SetFrame (v0),
            (0x04, 0x07, Some(&Scalar(v0)), None,             None) => EventAst::FrameSpeedModifier { multiplier: v0, unk: 0 },
            (0x04, 0x07, Some(&Scalar(v0)), Some(&Value(v1)), None) => EventAst::FrameSpeedModifier { multiplier: v0, unk: v1 },
            (0x0c, 0x23, Some(&Value(v0)),  Some(&Value(v1)), None) => EventAst::TimeManipulation (v0, v1),

            // misc state
            (0x0e, 0x00, Some(&Value(v0)), None, None) => EventAst::SetAirGround (v0),
            (0x08, 0x00, Some(&Value(v0)), None, None) => EventAst::SetEdgeSlide (EdgeSlide::new(v0)),
            (0x05, 0x00, None,             None, None) => EventAst::ReverseDirection,

            // hitboxes
            (0x06, 0x04, None,             None,     None) => EventAst::DeleteAllHitBoxes,
            (0x06, 0x00, Some(&Value(v0)), Some(v1), Some(&Value(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12)) {
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Scalar(v11)), Some(&Value(v12))) => {
                        let damage = match v1 {
                            Value(constant)        => Some(FloatValue::Constant (*constant as f32)),
                            Variable(ref variable) => Some(FloatValue::Variable (VariableAst::new(variable))),
                            _ => None,
                        };

                        if let Some(damage) = damage {
                            let v12u = v12 as u32;
                            EventAst::CreateHitBox (HitBoxArguments {
                                bone_index:                  (v0 >> 16) as i16,
                                set_id:                      (v0 >> 8)  as u8,
                                hitbox_id:                    v0        as u8,
                                damage,
                                trajectory:                   v2,
                                wdsk:                        (v3 >> 16) as i16,
                                kbg:                          v3        as i16,
                                shield_damage:               (v4 >> 16) as i16,
                                bkb:                          v4        as i16,
                                size:                         v5,
                                x_offset:                     v6,
                                y_offset:                     v7,
                                z_offset:                     v8,
                                tripping_rate:                v9,
                                hitlag_mult:                  v10,
                                sdi_mult:                     v11,
                                effect:     HitBoxEffect::new(v12 & 0b0000_0000_0000_0000_0000_0000_0001_1111),
                                unk1:                        (v12 & 0b0000_0000_0000_0000_0000_0000_0010_0000) != 0,
                                sound_level:                ((v12 & 0b0000_0000_0000_0000_0000_0000_1100_0000) >> 6) as u8,
                                unk2:                       ((v12 & 0b0000_0000_0000_0000_0000_0001_0000_0000) != 0),
                                sound:      HitBoxSound::new((v12 & 0b0000_0000_0000_0000_0011_1110_0000_0000) >> 9),
                                unk3:                       ((v12 & 0b0000_0000_0000_0000_1100_0000_0000_0000) >> 14) as u8,
                                ground:                      (v12 & 0b0000_0000_0000_0001_0000_0000_0000_0000) != 0,
                                aerial:                      (v12 & 0b0000_0000_0000_0010_0000_0000_0000_0000) != 0,
                                unk4:                       ((v12 & 0b0000_0000_0011_1100_0000_0000_0000_0000) >> 18) as u8,
                                sse_type: HitBoxSseType::new((v12 & 0b0000_0111_1100_0000_0000_0000_0000_0000) >> 22),
                                clang:                       (v12 & 0b0000_1000_0000_0000_0000_0000_0000_0000) != 0,
                                unk5:                        (v12 & 0b0001_0000_0000_0000_0000_0000_0000_0000) != 0,
                                direct:                      (v12 & 0b0010_0000_0000_0000_0000_0000_0000_0000) != 0,
                                unk6:                      ((v12u & 0b1100_0000_0000_0000_0000_0000_0000_0000) >> 30) as u8,
                            })
                        } else {
                            EventAst::Unknown (event.clone())
                        }
                    }
                    _ => EventAst::Unknown (event.clone())
                }
            }
            (0x06, 0x2B, Some(&Value(v0)), Some(&Value(v1)), Some(&Value(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12)) {
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Scalar(v11)), Some(&Value(v12))) => {
                        let v12u = v12 as u32;
                        EventAst::ThrownHitBox (HitBoxArguments {
                            bone_index:                  (v0 >> 16) as i16,
                            set_id:                      (v0 >> 8)  as u8,
                            hitbox_id:                    v0        as u8,
                            damage:                       FloatValue::Constant (v1 as f32),
                            trajectory:                   v2,
                            wdsk:                        (v3 >> 16) as i16,
                            kbg:                          v3        as i16,
                            shield_damage:               (v4 >> 16) as i16,
                            bkb:                          v4        as i16,
                            size:                         v5,
                            x_offset:                     v6,
                            y_offset:                     v7,
                            z_offset:                     v8,
                            tripping_rate:                v9,
                            hitlag_mult:                  v10,
                            sdi_mult:                     v11,
                            effect:     HitBoxEffect::new(v12 & 0b0000_0000_0000_0000_0000_0000_0001_1111),
                            unk1:                        (v12 & 0b0000_0000_0000_0000_0000_0000_0010_0000) != 0,
                            sound_level:                ((v12 & 0b0000_0000_0000_0000_0000_0000_1100_0000) >> 6) as u8,
                            unk2:                       ((v12 & 0b0000_0000_0000_0000_0000_0001_0000_0000) != 0),
                            sound:      HitBoxSound::new((v12 & 0b0000_0000_0000_0000_0011_1110_0000_0000) >> 9),
                            unk3:                       ((v12 & 0b0000_0000_0000_0000_1100_0000_0000_0000) >> 14) as u8,
                            ground:                      (v12 & 0b0000_0000_0000_0001_0000_0000_0000_0000) != 0,
                            aerial:                      (v12 & 0b0000_0000_0000_0010_0000_0000_0000_0000) != 0,
                            unk4:                       ((v12 & 0b0000_0000_0011_1100_0000_0000_0000_0000) >> 18) as u8,
                            sse_type: HitBoxSseType::new((v12 & 0b0000_0111_1100_0000_0000_0000_0000_0000) >> 22),
                            clang:                       (v12 & 0b0000_1000_0000_0000_0000_0000_0000_0000) != 0,
                            unk5:                        (v12 & 0b0001_0000_0000_0000_0000_0000_0000_0000) != 0,
                            direct:                      (v12 & 0b0010_0000_0000_0000_0000_0000_0000_0000) != 0,
                            unk6:                      ((v12u & 0b1100_0000_0000_0000_0000_0000_0000_0000) >> 30) as u8,
                        })
                    }
                    _ => EventAst::Unknown (event.clone())
                }
            }
            (0x06, 0x15, Some(&Value(v0)), Some(v1), Some(&Value(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12), args.get(13), args.get(14)) {
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Scalar(v11)), Some(&Value(v12)), Some(&Value(v13)), Some(&Value(v14))) => {
                        let damage = match v1 {
                            Value(constant)        => Some(FloatValue::Constant (*constant as f32)),
                            Variable(ref variable) => Some(FloatValue::Variable (VariableAst::new(variable))),
                            _ => None,
                        };

                        if let Some(damage) = damage {
                            let v12u = v12 as u32;
                            let v14u = v14 as u32;
                            EventAst::CreateSpecialHitBox (SpecialHitBoxArguments {
                                hitbox_args: HitBoxArguments {
                                    bone_index:                  (v0 >> 16) as i16,
                                    set_id:                      (v0 >> 8)  as u8,
                                    hitbox_id:                    v0        as u8,
                                    damage,
                                    trajectory:                   v2,
                                    wdsk:                        (v3 >> 16) as i16,
                                    kbg:                          v3        as i16,
                                    shield_damage:               (v4 >> 16) as i16,
                                    bkb:                          v4        as i16,
                                    size:                         v5,
                                    x_offset:                     v6,
                                    y_offset:                     v7,
                                    z_offset:                     v8,
                                    tripping_rate:                v9,
                                    hitlag_mult:                  v10,
                                    sdi_mult:                     v11,
                                    effect:     HitBoxEffect::new(v12 & 0b0000_0000_0000_0000_0000_0000_0001_1111),
                                    unk1:                        (v12 & 0b0000_0000_0000_0000_0000_0000_0010_0000) != 0,
                                    sound_level:                ((v12 & 0b0000_0000_0000_0000_0000_0000_1100_0000) >> 6) as u8,
                                    unk2:                       ((v12 & 0b0000_0000_0000_0000_0000_0001_0000_0000) != 0),
                                    sound:      HitBoxSound::new((v12 & 0b0000_0000_0000_0000_0011_1110_0000_0000) >> 6),
                                    unk3:                       ((v12 & 0b0000_0000_0000_0000_1100_0000_0000_0000) >> 14) as u8,
                                    ground:                      (v12 & 0b0000_0000_0000_0001_0000_0000_0000_0000) != 0,
                                    aerial:                      (v12 & 0b0000_0000_0000_0010_0000_0000_0000_0000) != 0,
                                    unk4:                       ((v12 & 0b0000_0000_0011_1100_0000_0000_0000_0000) >> 18) as u8,
                                    sse_type: HitBoxSseType::new((v12 & 0b0000_0111_1100_0000_0000_0000_0000_0000) >> 22),
                                    clang:                       (v12 & 0b0000_1000_0000_0000_0000_0000_0000_0000) != 0,
                                    unk5:                        (v12 & 0b0001_0000_0000_0000_0000_0000_0000_0000) != 0,
                                    direct:                      (v12 & 0b0010_0000_0000_0000_0000_0000_0000_0000) != 0,
                                    unk6:                      ((v12u & 0b1100_0000_0000_0000_0000_0000_0000_0000) >> 30) as u8,
                                },
                                rehit_rate: v13,
                                angle_flipping: AngleFlip::new(v14 & 0b0000_0000_0000_0000_0000_0000_0000_0111),
                                unk1:                         (v14 & 0b0000_0000_0000_0000_0000_0000_0000_1000) != 0,
                                stretches_to_bone:            (v14 & 0b0000_0000_0000_0000_0000_0000_0001_0000) != 0,
                                unk2:                         (v14 & 0b0000_0000_0000_0000_0000_0000_0010_0000) != 0,
                                can_hit1:                     (v14 & 0b0000_0000_0000_0000_0000_0000_0100_0000) != 0,
                                can_hit2:                     (v14 & 0b0000_0000_0000_0000_0000_0000_1000_0000) != 0,
                                can_hit3:                     (v14 & 0b0000_0000_0000_0000_0000_0001_0000_0000) != 0,
                                can_hit4:                     (v14 & 0b0000_0000_0000_0000_0000_0010_0000_0000) != 0,
                                can_hit5:                     (v14 & 0b0000_0000_0000_0000_0000_0100_0000_0000) != 0,
                                can_hit6:                     (v14 & 0b0000_0000_0000_0000_0000_1000_0000_0000) != 0,
                                can_hit7:                     (v14 & 0b0000_0000_0000_0000_0001_0000_0000_0000) != 0,
                                can_hit8:                     (v14 & 0b0000_0000_0000_0000_0010_0000_0000_0000) != 0,
                                can_hit9:                     (v14 & 0b0000_0000_0000_0000_0100_0000_0000_0000) != 0,
                                can_hit10:                    (v14 & 0b0000_0000_0000_0000_1000_0000_0000_0000) != 0,
                                can_hit11:                    (v14 & 0b0000_0000_0000_0001_0000_0000_0000_0000) != 0,
                                can_hit12:                    (v14 & 0b0000_0000_0000_0010_0000_0000_0000_0000) != 0,
                                can_hit13:                    (v14 & 0b0000_0000_0000_0100_0000_0000_0000_0000) != 0,
                                enabled:                      (v14 & 0b0000_0000_0000_1000_0000_0000_0000_0000) != 0,
                                unk3:                        ((v14 & 0b0000_0000_0011_0000_0000_0000_0000_0000) >> 20) as u8,
                                can_be_shielded:              (v14 & 0b0000_0000_0100_0000_0000_0000_0000_0000) != 0,
                                can_be_reflected:             (v14 & 0b0000_0000_1000_0000_0000_0000_0000_0000) != 0,
                                can_be_absorbed:              (v14 & 0b0000_0001_0000_0000_0000_0000_0000_0000) != 0,
                                unk4:                        ((v14 & 0b0000_0110_0000_0000_0000_0000_0000_0000) >> 25) as u8,
                                remain_grabbed:               (v14 & 0b0000_1000_0000_0000_0000_0000_0000_0000) != 0,
                                ignore_invincibility:         (v14 & 0b0001_0000_0000_0000_0000_0000_0000_0000) != 0,
                                freeze_frame_disable:         (v14 & 0b0010_0000_0000_0000_0000_0000_0000_0000) != 0,
                                unk5:                         (v14 & 0b0100_0000_0000_0000_0000_0000_0000_0000) != 0,
                                flinchless:                  (v14u & 0b1000_0000_0000_0000_0000_0000_0000_0000) != 0,
                            })
                        } else {
                            EventAst::Unknown (event.clone())
                        }
                    }
                    _ => EventAst::Unknown (event.clone())
                }
            }
            (0x06, 0x1B, Some(&Value(v0)), Some(&Value(v1)), Some(&Scalar(v2))) => {
                if let (Some(&Scalar(v3)), Some(&Scalar(v4))) = (args.get(3), args.get(4)) {
                    EventAst::MoveHitBox (MoveHitBox {
                        hitbox_id:    v0,
                        new_bone:     v1,
                        new_x_offset: v2,
                        new_y_offset: v3,
                        new_z_offset: v4,
                    })
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x06, 0x01, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::ChangeHitBoxDamage { hitbox_id: v0, new_damage: v1 },
            (0x06, 0x02, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::ChangeHitBoxSize   { hitbox_id: v0, new_size:   v1 },
            (0x06, 0x03, Some(&Value(v0)), None,             None) => EventAst::DeleteHitBox (v0),
            (0x06, 0x0A, Some(&Value(v0)), Some(&Value(v1)), Some(&Scalar(v2))) => {
                if let (Some(&Scalar(v3)), Some(&Scalar(v4)), Some(&Scalar(v5)), Some(&Value(v6)), Some(&Value(v7))) =
                    (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7))
                {
                    let unk = if let Some(&Value(value)) = args.get(8) { Some(value) } else { None };

                    assert!(v0 >= 0 && v0 <= 0xFF, "grab boxes shouldn't include any extra data with the hitbox_id");

                    EventAst::CreateGrabBox(GrabBoxArguments {
                        hitbox_id:    v0,
                        bone_index:   v1,
                        size:         v2,
                        x_offset:     v3,
                        y_offset:     v4,
                        z_offset:     v5,
                        set_action:   v6,
                        target:       GrabTarget::new(v7),
                        unk
                    })
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x06, 0x0C, Some(&Value(v0)), None, None) => EventAst::DeleteGrabBox (v0),
            (0x06, 0x0D, None,             None, None) => EventAst::DeleteAllGrabBoxes,
            (0x06, 0x0E, Some(&Value(v0)), Some(&Value(v1)), Some(&Value(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12), args.get(13), args.get(14), args.get(15), args.get(16)) {
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Value(v5)), Some(&Value(v6)), Some(&Value(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Value(v11)), Some(&Value(v12)), Some(&Value(v13)), Some(&Bool(v14)), Some(&Bool(v15)), Some(&Value(v16))) => {
                        EventAst::SpecifyThrow (SpecifyThrow {
                            throw_use:   ThrowUse::new(v0),
                            bone:        v1,
                            damage:      v2,
                            trajectory:  v3,
                            kbg:         v4,
                            wdsk:        v5,
                            bkb:         v6,
                            effect:      HitBoxEffect::new(v7),
                            unk0:        v8,
                            unk1:        v9,
                            unk2:        v10,
                            unk3:        v11,
                            sfx:         HitBoxSound::new(v12),
                            grab_target: GrabTarget::new(v13),
                            unk4:        v14,
                            unk5:        v15,
                            i_frames:    v16,
                        })
                    }
                    _ => EventAst::Unknown(event.clone())
                }
            }
            (0x06, 0x0F, Some(&Value(v0)), Some(&Value(v1)), Some(&Variable(ref v2))) => {
                if let (Some(&Variable(ref v3)), Some(&Variable(ref v4))) = (args.get(3), args.get(4)) {
                    EventAst::ApplyThrow (ApplyThrow {
                        unk0: v0,
                        bone: v1,
                        unk1: VariableAst::new(v2),
                        unk2: VariableAst::new(v3),
                        unk3: VariableAst::new(v4),
                    })
                } else {
                    EventAst::Unknown(event.clone())
                }
            }

            // hurtboxes
            (0x06, 0x05, Some(&Value(v0)), None,             None) => EventAst::ChangeHurtBoxStateAll { state: HurtBoxState::new(v0) },
            (0x06, 0x08, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::ChangeHurtBoxStateSpecific { bone: v0, state: HurtBoxState::new(v1) },
            (0x06, 0x06, Some(&Value(v0)), None,             None) => {
                if v0 != 0 {
                    error!("Unsual UnchangeHurtBoxStateSpecific argument: All known cases of this event have an argument of 0")
                }
                EventAst::UnchangeHurtBoxStateSpecific
            }

            // controller
            (0x07, 0x00, None,              None,              None) => EventAst::ControllerClearBuffer,
            (0x07, 0x01, None,              None,              None) => EventAst::ControllerUnk01,
            (0x07, 0x02, None,              None,              None) => EventAst::ControllerUnk02,
            (0x07, 0x06, Some(&Bool(v0)),   None,              None) => EventAst::ControllerUnk06 (v0),
            (0x07, 0x06, None,              None,              None) => EventAst::ControllerUnk0C,
            (0x07, 0x07, Some(&Value(v0)),  Some(&Value(v1)),  None) => EventAst::Rumble { unk1: v0, unk2: v1 },
            (0x07, 0x0B, Some(&Value(v0)),  Some(&Value(v1)),  None) => EventAst::RumbleLoop { unk1: v0, unk2: v1 },

            // misc
            (0x18, 0x00, Some(&Value(v0)),  None,              None) => EventAst::SlopeContourStand { leg_bone_parent: v0 },
            (0x18, 0x01, Some(&Value(v0)),  Some(&Value(v1)),  None) => EventAst::SlopeContourFull { hip_n_or_top_n: v0, trans_bone: v1 },
            (0x10, 0x00, Some(&Value(v0)),  None,              None) => EventAst::GenerateArticle { article_id: v0, subaction_only: true }, // TODO: subaction_only?
            (0x10, 0x00, Some(&Value(v0)),  Some(&Bool(v1)),   None) => EventAst::GenerateArticle { article_id: v0, subaction_only: v1 },
            (0x10, 0x01, Some(&Value(v0)),  None,              None) => EventAst::ArticleEvent (v0),
            (0x10, 0x02, Some(&Value(v0)),  None,              None) => EventAst::ArticleAnimation (v0),
            (0x10, 0x03, Some(&Value(v0)),  None,              None) => EventAst::ArticleRemove (v0),
            (0x10, 0x05, Some(&Value(v0)),  Some(&Bool(v1)),   None) => EventAst::ArticleVisibility { article_id: v0, visibility: v1 },
            (0x0C, 0x06, None,              None,              None) => EventAst::FinalSmashEnter,
            (0x0C, 0x07, None,              None,              None) => EventAst::FinalSmashExit,
            (0x0C, 0x08, None,              None,              None) => EventAst::TerminateSelf,
            (0x0C, 0x09, Some(&Value(v0)),  None,              None) => EventAst::LedgeGrabEnable (LedgeGrabEnable::new(v0)),
            (0x0C, 0x25, Some(&Bool(v0)),   None,              None) => EventAst::TagDisplay (v0),
            (0x1E, 0x00, Some(&Value(v0)),  Some(&Scalar(v1)), None) => EventAst::Armor { armor_type: ArmorType::new(v0), tolerance: v1 },
            (0x1E, 0x03, Some(&Scalar(v0)), None,              None) => EventAst::AddDamage (v0),

            // posture
            (0x05, 0x01, None, None, None) => EventAst::Posture (0x01),
            (0x05, 0x02, None, None, None) => EventAst::Posture (0x02),
            (0x05, 0x03, None, None, None) => EventAst::Posture (0x03),
            (0x05, 0x04, None, None, None) => EventAst::Posture (0x04),
            (0x05, 0x07, None, None, None) => EventAst::Posture (0x07),
            (0x05, 0x0D, None, None, None) => EventAst::Posture (0x0D),

            // movement
            (0x0E, 0x08, Some(&Scalar(v0)), Some(&Scalar(v1)), Some(&Value(v2))) => {
                if let Some(&Value(v3)) = args.get(3) {
                    EventAst::SetOrAddVelocity (SetOrAddVelocity {
                        x_vel: v0,
                        y_vel: v1,
                        x_set: v2 != 0,
                        y_set: v3 != 0,
                    })
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x0E, 0x08, Some(&Scalar(v0)),       Some(&Scalar(v1)),       None) => EventAst::SetVelocity { x_vel: v0, y_vel: v1 },
            (0x0E, 0x01, Some(&Scalar(v0)),       Some(&Scalar(v1)),       None) => EventAst::AddVelocity { x_vel: FloatValue::Constant(v0),                   y_vel: FloatValue::Constant(v1) },
            (0x0E, 0x01, Some(&Variable(ref v0)), Some(&Scalar(v1)),       None) => EventAst::AddVelocity { x_vel: FloatValue::Variable(VariableAst::new(v0)), y_vel: FloatValue::Constant(v1) },
            (0x0E, 0x01, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::AddVelocity { x_vel: FloatValue::Constant(v0),                   y_vel: FloatValue::Variable(VariableAst::new(v1)) },
            (0x0E, 0x01, Some(&Variable(ref v0)), Some(&Variable(ref v1)), None) => EventAst::AddVelocity { x_vel: FloatValue::Variable(VariableAst::new(v0)), y_vel: FloatValue::Variable(VariableAst::new(v1)) },
            (0x0E, 0x06, Some(&Value(v0)),        None,                    None) => EventAst::DisableMovement (DisableMovement::new(v0)),
            (0x0E, 0x07, Some(&Value(v0)),        None,                    None) => EventAst::DisableMovement2 (DisableMovement::new(v0)),
            (0x0E, 0x02, Some(&Value(v0)),        None,                    None) => EventAst::ResetVerticalVelocityAndAcceleration (v0 == 1),

            // sound
            (0x0A, 0x00, Some(&Value(v0)), None, None) => EventAst::SoundEffect1 (v0),
            (0x0A, 0x01, Some(&Value(v0)), None, None) => EventAst::SoundEffect2 (v0),
            (0x0A, 0x02, Some(&Value(v0)), None, None) => EventAst::SoundEffectTransient (v0),
            (0x0A, 0x03, Some(&Value(v0)), None, None) => EventAst::SoundEffectStop (v0),
            (0x0A, 0x05, Some(&Value(v0)), None, None) => EventAst::SoundEffectVictory (v0),
            (0x0A, 0x07, Some(&Value(v0)), None, None) => EventAst::SoundEffectUnk (v0),
            (0x0A, 0x09, Some(&Value(v0)), None, None) => EventAst::SoundEffectOther1 (v0),
            (0x0A, 0x0A, Some(&Value(v0)), None, None) => EventAst::SoundEffectOther2 (v0),
            (0x0C, 0x0B, None,             None, None) => EventAst::SoundVoiceLow,
            (0x0C, 0x19, None,             None, None) => EventAst::SoundVoiceDamage,
            (0x0C, 0x1D, None,             None, None) => EventAst::SoundVoiceOttotto,
            (0x0C, 0x1F, None,             None, None) => EventAst::SoundVoiceEating,

            // Modify variables
            (0x12, 0x00, Some(&Value(v0)),        Some(&Variable(ref v1)), None) => EventAst::IntVariableSet { value: v0, variable: VariableAst::new(v1) },
            (0x12, 0x01, Some(&Value(v0)),        Some(&Variable(ref v1)), None) => EventAst::IntVariableAdd { value: v0, variable: VariableAst::new(v1) },
            (0x12, 0x02, Some(&Value(v0)),        Some(&Variable(ref v1)), None) => EventAst::IntVariableSubtract { value: v0, variable: VariableAst::new(v1) },
            (0x12, 0x03, Some(&Variable(ref v0)), None,                    None) => EventAst::IntVariableIncrement { variable: VariableAst::new(v0) },
            (0x12, 0x04, Some(&Variable(ref v0)), None,                    None) => EventAst::IntVariableDecrement { variable: VariableAst::new(v0) },
            (0x12, 0x06, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableSet      { value: FloatValue::Constant(v0),                   variable: VariableAst::new(v1) },
            (0x12, 0x06, Some(&Variable(ref v0)), Some(&Variable(ref v1)), None) => EventAst::FloatVariableSet      { value: FloatValue::Variable(VariableAst::new(v0)), variable: VariableAst::new(v1) },
            (0x12, 0x07, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableAdd      { value: FloatValue::Constant(v0),                   variable: VariableAst::new(v1) },
            (0x12, 0x07, Some(&Variable(ref v0)), Some(&Variable(ref v1)), None) => EventAst::FloatVariableAdd      { value: FloatValue::Variable(VariableAst::new(v0)), variable: VariableAst::new(v1) },
            (0x12, 0x08, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableSubtract { value: FloatValue::Constant(v0),                   variable: VariableAst::new(v1) },
            (0x12, 0x08, Some(&Variable(ref v0)), Some(&Variable(ref v1)), None) => EventAst::FloatVariableSubtract { value: FloatValue::Variable(VariableAst::new(v0)), variable: VariableAst::new(v1) },
            (0x12, 0x0F, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableMultiply { value: FloatValue::Constant(v0),                   variable: VariableAst::new(v1) },
            (0x12, 0x0F, Some(&Variable(ref v0)), Some(&Variable(ref v1)), None) => EventAst::FloatVariableMultiply { value: FloatValue::Variable(VariableAst::new(v0)), variable: VariableAst::new(v1) },
            (0x12, 0x10, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableDivide   { value: FloatValue::Constant(v0),                   variable: VariableAst::new(v1) },
            (0x12, 0x10, Some(&Variable(ref v0)), Some(&Variable(ref v1)), None) => EventAst::FloatVariableDivide   { value: FloatValue::Variable(VariableAst::new(v0)), variable: VariableAst::new(v1) },
            (0x12, 0x0A, Some(&Variable(ref v0)), None,                    None) => EventAst::BoolVariableSetTrue { variable: VariableAst::new(v0) },
            (0x12, 0x0B, Some(&Variable(ref v0)), None,                    None) => EventAst::BoolVariableSetFalse { variable: VariableAst::new(v0) },

            // graphics
            (0x0B, 0x00, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::ModelChanger { reference: 0, switch_index: v0, bone_group_index: v1 },
            (0x0B, 0x01, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::ModelChanger { reference: 1, switch_index: v0, bone_group_index: v1 },
            (0x11, 0x1A, Some(&Value(v0)), Some(&Value(v1)), Some(&Scalar(v2))) |
            (0x11, 0x1B, Some(&Value(v0)), Some(&Value(v1)), Some(&Scalar(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12), args.get(13), args.get(14), args.get(15)) {
                    (Some(&Scalar(v3)), Some(&Scalar(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Scalar(v11)), Some(&Scalar(v12)), Some(&Scalar(v13)), Some(&Scalar(v14)), Some(&Bool(v15))) => {
                        EventAst::GraphicEffect (GraphicEffect {
                            graphic:                  v0,
                            bone:                     v1,
                            x_offset:                 v4,
                            y_offset:                 v3,
                            z_offset:                 v2,
                            x_rotation:               v7,
                            y_rotation:               v6,
                            z_rotation:               v5,
                            scale:                    v8,
                            random_x_offset:          v11,
                            random_y_offset:          v10,
                            random_z_offset:          v9,
                            random_x_rotation:        v14,
                            random_y_rotation:        v13,
                            random_z_rotation:        v12,
                            terminate_with_animation: v15
                        })
                    }
                    _ => EventAst::Unknown (event.clone())
                }
            }
            (0x11, 0x00, Some(&Value(v0)), Some(&Value(v1)), Some(&Scalar(v2))) => {
                if let (Some(&Scalar(v3)), Some(&Scalar(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Scalar(v11)), Some(&Scalar(v12)), Some(&Scalar(v13)), Some(&Scalar(v14)), Some(&Bool(v15))) =
                    (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12), args.get(13), args.get(14), args.get(15))
                {
                    EventAst::ExternalGraphicEffect (ExternalGraphicEffect {
                        file:       (v0 >> 16) as i16,
                        graphic:    v0 as i16,
                        bone:       v1,
                        x_offset:   v4,
                        y_offset:   v3,
                        z_offset:   v2,
                        x_rotation: v7,
                        y_rotation: v6,
                        z_rotation: v5,
                        scale:      v8,
                        randomize:  Some(ExternalGraphicEffectRandomize {
                            random_x_offset:   v11,
                            random_y_offset:   v10,
                            random_z_offset:   v9,
                            random_x_rotation: v14,
                            random_y_rotation: v13,
                            random_z_rotation: v12,
                        }),
                        terminate_with_animation: v15,
                    })
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x11, 0x01, Some(&Value(v0)), Some(&Value(v1)), Some(&Scalar(v2))) |
            (0x11, 0x02, Some(&Value(v0)), Some(&Value(v1)), Some(&Scalar(v2))) => {
                if let (Some(&Scalar(v3)), Some(&Scalar(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Bool(v9))) =
                    (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9))
                {
                    EventAst::ExternalGraphicEffect (ExternalGraphicEffect {
                        file:                     (v0 >> 16) as i16,
                        graphic:                  v0 as i16,
                        bone:                     v1,
                        x_offset:                 v4,
                        y_offset:                 v3,
                        z_offset:                 v2,
                        x_rotation:               v7,
                        y_rotation:               v6,
                        z_rotation:               v5,
                        scale:                    v8,
                        terminate_with_animation: v9,
                        randomize:                None,
                    })
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x11, 0x17, Some(&Value(v0)), Some(&Value(v1)), Some(&Value(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6)) {
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Value(v5)), Some(&Value(v6))) => {
                        EventAst::LimitedScreenTint (LimitedScreenTint {
                            transition_in_time: v0,
                            red: v1,
                            green: v2,
                            blue: v3,
                            alpha: v4,
                            frame_count: v5,
                            transition_out_time: v6,
                        })
                    }
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Value(v5)), None) => {
                        EventAst::UnlimitedScreenTint (UnlimitedScreenTint {
                            tint_id: v0,
                            transition_in_time: v1,
                            red: v2,
                            green: v3,
                            blue: v4,
                            alpha: v5,
                        })
                    }
                    _ => EventAst::Unknown (event.clone())
                }
            }
            (0x11, 0x18, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::EndUnlimitedScreenTint { tint_id: v0, transition_out_time: v1 },
            (0x11, 0x03, Some(&Value(v0)), Some(&Value(v1)), Some(&Value(v2))) => {
                if let (Some(&Scalar(v3)), Some(&Scalar(v4)), Some(&Scalar(v5)), Some(&Value(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Bool(v10)), Some(&Value(v11)), Some(&Value(v12)), Some(&Scalar(v13)), Some(&Scalar(v14)), Some(&Scalar(v15)), Some(&Scalar(v16)), Some(&Scalar(v17)), Some(&Scalar(v18)), Some(&Scalar(v19))) =
                    (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12), args.get(13), args.get(14), args.get(15), args.get(16), args.get(17), args.get(18), args.get(19))
                {
                    EventAst::SwordGlow (SwordGlow {
                        color:                  v0,
                        blur_length:            v1,

                        point1_bone:            v2,
                        point1_x_offset:        v3,
                        point1_y_offset:        v4,
                        point1_z_offset:        v5,

                        point2_bone:            v6,
                        point2_x_offset:        v7,
                        point2_y_offset:        v8,
                        point2_z_offset:        v9,

                        delete_after_subaction: v10,
                        graphic_id:             v11,
                        bone_id:                v12,
                        x_offset:               v13,
                        y_offset:               v14,
                        z_offset:               v15,
                        x_rotation:             v16,
                        y_rotation:             v17,
                        z_rotation:             v18,
                        glow_length:            v19,
                    })
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x11, 0x05, Some(&Value(v0)), None,              None) => EventAst::DeleteSwordGlow { fade_time: v0 },
            (0x14, 0x07, Some(&Value(v0)), Some(&Scalar(v1)), Some(&Scalar(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9)) {
                    (Some(&Scalar(v3)), Some(&Scalar(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Value(v9))) => {
                        EventAst::AestheticWindEffect (AestheticWindEffect {
                            unk1:    v0,
                            unk2:    v1,
                            stength: v2,
                            speed:   v3,
                            size:    v4,
                            unk3:    v5,
                            unk4:    v6,
                            unk5:    v7,
                            unk6:    v8,
                            unk7:    v8,
                            unk8:    v9,
                        })
                    }
                    _ => EventAst::Unknown (event.clone())
                }
            }
            (0x14, 0x04, Some(&Value(v0)), None, None) => EventAst::EndAestheticWindEffect { unk: v0 },
            (0x1A, 0x00, Some(&Value(v0)), None, None) => EventAst::ScreenShake { magnitude: v0 },
            (0x1A, 0x04, Some(&Value(v0)), Some(&Value(v1)), Some(&Scalar(v2))) => {
                if let (Some(&Scalar(v3)), Some(&Scalar(v4))) = (args.get(3), args.get(4)) {
                    EventAst::CameraCloseup (CameraCloseup {
                        zoom_time: v0,
                        unk: v1,
                        distance: v2,
                        x_angle: v3,
                        y_angle: v4,
                    })
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x1A, 0x08, None,             None,             None) => EventAst::CameraNormal,
            (0x1F, 0x00, Some(&Value(v0)), None,             None) => EventAst::ItemPickup { unk1: v0, unk2: None, unk3: None, unk4: None },
            (0x1F, 0x00, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::ItemPickup { unk1: v0, unk2: Some(v1), unk3: None, unk4: None },
            (0x1F, 0x00, Some(&Value(v0)), Some(&Value(v1)), Some(&Value(v2))) => 
                if let Some(&Value(v3)) = args.get(3) {
                    EventAst::ItemPickup { unk1: v0, unk2: Some(v1), unk3: Some(v2), unk4: Some(v3) }
                } else {
                    EventAst::Unknown (event.clone())
                },
            (0x1F, 0x01, Some(&Variable(ref v0)), Some(&Variable(ref v1)), Some(&Variable(ref v2))) =>
                if let (Some(&Variable(ref v3)), Some(&Variable(ref v4))) = (args.get(3), args.get(4)) {
                    EventAst::ItemThrow { unk1: VariableAst::new(v0), unk2: VariableAst::new(v1), unk3: VariableAst::new(v2), unk4: Some(VariableAst::new(v3)), unk5: Some(VariableAst::new(v4)) }
                } else {
                    EventAst::Unknown (event.clone())
                },
            (0x1F, 0x0E, Some(&Variable(ref v0)), Some(&Variable(ref v1)), Some(&Variable(ref v2)))
                => EventAst::ItemThrow { unk1: VariableAst::new(v0), unk2: VariableAst::new(v1), unk3: VariableAst::new(v2), unk4: None, unk5: None },
            (0x1F, 0x01, Some(&Scalar(v0)), Some(&Scalar(v1)), Some(&Variable(ref v2))) =>
                    EventAst::ItemThrow2 { unk1: v0, unk2: v1, unk3: VariableAst::new(v2) },
            (0x1F, 0x02, None,             None,              None) => EventAst::ItemDrop,
            (0x1F, 0x03, Some(&Value(v0)), None,              None) => EventAst::ItemConsume { unk: v0 },
            (0x1F, 0x04, Some(&Value(v0)), Some(&Scalar(v1)), None) => EventAst::ItemSetProperty { unk1: v0, unk2: v1 },
            (0x1F, 0x05, None,             None,              None) => EventAst::FireWeapon,
            (0x1F, 0x06, None,             None,              None) => EventAst::FireProjectile,
            (0x1F, 0x07, Some(&Value(v0)), None,              None) => EventAst::Item1F { unk: v0 },
            (0x1F, 0x08, Some(&Value(v0)), None,              None) => EventAst::ItemCreate { unk: v0 },
            (0x1F, 0x09, Some(&Bool(v0)),  None,              None) => EventAst::ItemVisibility (v0),
            (0x1F, 0x0C, Some(&Value(v0)), None,              None) => EventAst::BeamSwordTrail { unk: v0 },
            _ => EventAst::Unknown (event.clone())
        };
        // Brawlbox has some extra parameter types it uses to handle some special cases:
        // *    HitBoxFlags
        // *    Value2Half
        // *    ValueGFX
        // I dont use them because they are just subtypes of Argument::Value
        // Instead I handle them in the ast parser

        // These are the rules brawlbox uses for determining if an argument is one of these special types
        //let argument = if (event_id == 0x06000D00 || event_id == 0x06150F00 || event_id == 0x062B0D00) && i == 12 {
        //    Argument::HitBoxFlags (data)
        //} else if (event_id == 0x06000D00 || event_id == 0x05150F00 || event_id == 0x062B0D00) && (i == 0 || i == 3 || i == 4) {
        //    Argument::Value2Half (data)
        //} else if (event_id == 0x11150300 || event_id == 0x11001000 || event_id == 0x11020A00) && i == 0 {
        //    Argument::ValueGFX (data)
        // TODO: Delete each comment when actually implemented
        event_asts.push(event_ast);
    }
    ProcessedBlock::Finished(Block { events: event_asts })
}

impl Expression {
    fn from_args(requirement: &Requirement, flip: bool, v1: Option<&Argument>, v2: Option<&Argument>, v3: Option<&Argument>) -> Option<Expression> {
        let test = match (v1, v2, v3) {
            (None, None, None) => {
                Expression::Nullary(requirement.clone())
            }
            (Some(v1), None, None) => {
                let value = Box::new(match v1 {
                    &Argument::Scalar(v1)       => Expression::Scalar(v1),
                    &Argument::Variable(ref v1) => Expression::Variable(VariableAst::new(v1)),
                    &Argument::Value(v1)        => Expression::Value(v1),
                    _ => {
                        error!("Unhandled expression case: value: {:?}", v1);
                        return None;
                    }
                });
                Expression::Unary (UnaryExpression { requirement: requirement.clone(), value })
            }
            (Some(v1), Some(&Argument::Value(v2)), Some(v3)) => {
                let left = Box::new(match v1 {
                    &Argument::Scalar(v1)       => Expression::Scalar(v1),
                    &Argument::Variable(ref v1) => Expression::Variable(VariableAst::new(v1)),
                    &Argument::Value(v1)        => Expression::Value(v1),
                    _ => {
                        error!("Unhandled expression case: left");
                        return None;
                    }
                });
                let right = Box::new(match v3 {
                    &Argument::Scalar(v3)       => Expression::Scalar(v3),
                    &Argument::Variable(ref v3) => Expression::Variable(VariableAst::new(v3)),
                    &Argument::Value(v3)        => Expression::Value(v3),
                    _ => {
                        error!("Unhandled expression case: right");
                        return None;
                    }
                });

                match requirement {
                    script::Requirement::Comparison => Expression::Binary (BinaryExpression { left, right, operator: ComparisonOperator::from_arg(v2) }),
                    // Seems to be just modders using this as a quick hack.
                    script::Requirement::Always => Expression::Nullary(requirement.clone()),
                    _ => {
                        error!("Unhandled expression case: comparison v0: {:?} v1: {:?} v2: {:?}: v3: {:?}", requirement, v1, v2, v3);
                        return None;
                    }
                }
            }
            (v1, v2, v3) => {
                error!("Unhandled expression case: {:?} {:?} {:?}", v1, v2, v3);
                return None;
            }
        };

        Some(if flip { Expression::Not (Box::new(test)) } else { test })
    }
}

enum ProcessedBlock {
    Finished     (Block),
    EndForLoop   (Block),
    EndIf        { then_branch: Block },
    EndIfAndElse { then_branch: Block, else_branch: Option<Box<Block>> },
}

#[derive(Serialize, Clone, Debug)]
pub enum EventAst {
    ///Pause the current flow of events until the set time is reached. Synchronous timers count down when they are reached in the code.
    SyncWait (f32),
    /// Does nothing.
    Nop,
    /// Pause the current flow of events until the set time is reached. Asynchronous Timers start counting from the beginning of the animation.
    AsyncWait (f32),
    /// Execute the block of code N times.
    ForLoop (ForLoop),
    /// Enter the event routine specified and return after ending.
    Subroutine (Offset),
    /// Return from a Subroutine.
    Return,
    /// Goto the event location specified and execute.
    Goto (Offset),
    /// An expression decides which block of code to execute.
    IfStatement (IfStatement),
    /// An `And` to an If statement.
    /// If the expression is false then execution of all events other than IfStatementOr are skipped.
    /// Execution can be resumed by an IfStatementOr
    ///
    /// Has no effect outside of an IfStatement
    /// Havent tested if it would affect execution when called within a subroutine, but I would assume it is.
    IfStatementAnd (Expression),
    /// An `Or` to an If statement.
    /// If the expression is true then execution of all events is re-enabled
    /// Execution can be stopped by an IfStatementAnd
    ///
    /// Has no effect outside of an IfStatement
    /// Havent tested if it would affect execution when called within a subroutine, but I would assume it is.
    IfStatementOr (Expression),
    /// Begin a multiple case Switch block.
    Switch (i32, i32),
    /// Handler for if the variable in the switch statement equals the specified value.
    Case (i32),
    /// The case chosen if none of the others are executed.
    DefaultCase,
    /// End a Switch block.
    EndSwitch,
    /// Briefly return execution back to the system to prevent crashes during infinite loops.
    LoopRest,
    /// Runs a subroutine once per frame for the current action.
    CallEveryFrame { thread_id: i32, offset: Offset },
    /// Stops the execution of a loop created with CallEveryFrame
    RemoveCallEveryFrame { thread_id: i32 },
    /// Enables the given interrupt ID on any interrupt type.
    EnableInterrupt (i32),
    /// Disables the given interrupt ID on any interrupt type.
    DisableInterrupt (i32),
    /// Invert the given interrupt ID assosciated with the given interrupt type.
    ToggleInterrupt { interrupt_type: InterruptType, interrupt_id: i32 },
    /// Enables all interrupts associated with the given interrupt type.
    EnableInterruptGroup (InterruptType),
    /// Disables all interrupts associated with the given interrupt type.
    DisableInterruptGroup (InterruptType),
    /// Remove all actions currently assosciated with an interrupt type.
    ClearInterruptGroup (InterruptType),
    /// An interrupt with the given interrupt ID is assosciated with the interrupt type of that action.
    /// The interrupt type used, seems to be hardcoded to the action somehow.
    /// The current action will change upon test being true. (the requirement does not have to be met at the time this ID is executed - it can be used anytime after execution.)
    CreateInterrupt (Interrupt),
    /// Add an additional requirement to the preceeding CreateInterrupt statement.
    /// All requirements on the interrupt must be true for the interrupt to occur.
    PreviousInterruptAddRequirement { test: Expression },
    /// Add an additonal requirement to the specified interrupt type and interrupt id.
    /// All requirements on the interrupt must be true for the interrupt to occur.
    InterruptAddRequirement { interrupt_type: InterruptType, interrupt_id: i32, test: Expression },
    /// Allow the current action to be interrupted by another action.
    AllowInterrupts,
    /// Change the current subaction.
    ChangeSubaction (i32),
    /// Change the current subaction, restarting the frame count.
    ChangeSubactionRestartFrame (i32),
    /// Changes the current frame of the animation. Does not change the frame of the subaction (i.e. timers and such are unaffected).
    SetFrame (f32),
    /// Dictates the frame speed of the subaction. Example: setting to 2 makes the animation and timers occur twice as fast.
    FrameSpeedModifier { multiplier: f32, unk: i32 },
    /// Change the speed of time for various parts of the environment.
    TimeManipulation (i32, i32),
    /// Specify whether the character is on or off the ground.
    SetAirGround (i32),
    /// Determines whether or not the character will slide off the edge.
    SetEdgeSlide (EdgeSlide),
    /// Reverse the direction the character is facing after the animation ends.
    ReverseDirection,
    /// Create a hitbox with the specified parameters.
    CreateHitBox (HitBoxArguments), // brawlbox calls this "Offensive Collision"
    /// Create a hitbox with the specified parameters on the character to be thrown. The hitbox can not hit the throwing character or the thrown character.
    /// TODO: I actually dont understand this at all, it seems characters without this still have hitboxes regardless of who throws who.
    ThrownHitBox (HitBoxArguments),
    /// Remove all currently present hitboxes.
    DeleteAllHitBoxes, // brawlbox calls this "Terminate Collisions"
    /// Create a hitbox with the even more parameters.
    CreateSpecialHitBox (SpecialHitBoxArguments), // brawlbox calls this "Special Offensive Collision"
    /// Repositions an already-existing hitbox.
    MoveHitBox (MoveHitBox),
    /// Changes a specific hitbox's damage to the new amount. Only guaranteed to work on a HitBox
    ChangeHitBoxDamage { hitbox_id: i32, new_damage: i32 },
    /// Changes a specific hitbox's size to the new amount. Only guaranteed to work on a HitBox
    ChangeHitBoxSize { hitbox_id: i32, new_size: i32 },
    /// Deletes a hitbox of the specified ID. Only guaranteed to work on a HitBox
    DeleteHitBox (i32),
    /// Generate a grabbox with the specified parameters.
    CreateGrabBox (GrabBoxArguments),
    /// Deletes the grabbox with the specified ID.
    DeleteGrabBox (i32),
    /// Remove all currently present grabboxes
    DeleteAllGrabBoxes,
    /// Specify the throw
    SpecifyThrow (SpecifyThrow),
    /// Apply the previously specified throw
    ApplyThrow (ApplyThrow),
    /// Set the state of all of the characters hurtboxes.
    ChangeHurtBoxStateAll { state: HurtBoxState },
    /// Sets the state of a characters specific hurtbox.
    ChangeHurtBoxStateSpecific { bone: i32, state: HurtBoxState },
    /// Sets the state of a characters specific hurtbox to the global value.
    UnchangeHurtBoxStateSpecific,
    /// Possibly clears the controller buffer.
    ControllerClearBuffer,
    /// Unknown controller event
    ControllerUnk01,
    /// Unknown controller event
    ControllerUnk02,
    /// Unknown controller event
    ControllerUnk06 (bool),
    /// Unknown controller event
    ControllerUnk0C,
    /// Undefined. Affects the rumble feature of the controller.
    Rumble { unk1: i32, unk2: i32 },
    /// Creates a rumble loop on the controller.
    RumbleLoop { unk1: i32, unk2: i32 },
    /// Moves the character's feet if on sloped ground.
    SlopeContourStand { leg_bone_parent: i32 },
    /// Moves entire character to match sloped ground.
    SlopeContourFull { hip_n_or_top_n: i32, trans_bone: i32 },
    /// Generate a pre-made prop effect from the prop library.
    GenerateArticle { article_id: i32, subaction_only: bool },
    /// Makes the article preform an animation when set to 1.
    ArticleEvent (i32),
    /// Article Animation.
    ArticleAnimation (i32),
    /// Removes an article.
    ArticleRemove (i32),
    /// Makes an article visible or invisible.
    ArticleVisibility { article_id: i32, visibility: bool },
    /// Allows use of Final Smash locked articles, variables, etc. Highly unstable.
    FinalSmashEnter,
    /// Exit Final Smash state
    FinalSmashExit,
    /// Used by certain article instances to remove themselves.
    TerminateSelf,
    /// Allow or disallow grabbing ledges during the current subaction.
    LedgeGrabEnable (LedgeGrabEnable),
    /// Disables or enables tag display for the current subaction.
    TagDisplay (bool),
    /// Begins super armor or heavy armor. Set parameters to None and 0 to end the armor.
    Armor { armor_type: ArmorType, tolerance: f32 },
    /// Adds the specified amount of damage to the character's current percentage.
    AddDamage (f32),
    /// ???
    Posture (i32),
    /// Will either set or add the velocity amounts depending on the set_ flags.
    SetOrAddVelocity (SetOrAddVelocity),
    /// Sets the character's current velocity.
    SetVelocity { x_vel: f32, y_vel: f32 },
    /// Adds to the character's current velocity.
    AddVelocity { x_vel: FloatValue, y_vel: FloatValue },
    /// Does not allow the specified type of movement.
    DisableMovement (DisableMovement),
    /// This must be set to the same value as DisableMovement to work.
    DisableMovement2 (DisableMovement),
    /// When set to 1, vertical speed and acceleration are reset back to 0.
    ResetVerticalVelocityAndAcceleration (bool),
    /// Play a specified sound effect.
    SoundEffect1 (i32),
    /// Play a specified sound effect.
    SoundEffect2 (i32),
    /// Play a specified sound effect. The sound effect ends with the animation.
    SoundEffectTransient (i32),
    /// Stops the specified sound effect immediately.
    SoundEffectStop (i32),
    /// Play a specified sound effect. Is used during victory poses.
    SoundEffectVictory (i32),
    /// Unknown.
    SoundEffectUnk (i32),
    /// Play a specified sound effect.
    SoundEffectOther1 (i32),
    /// Play a specified sound effect.
    SoundEffectOther2 (i32),
    /// Play a random low voice clip.
    SoundVoiceLow,
    /// Play a random damage voice clip.
    SoundVoiceDamage,
    /// Play the Ottotto voice clip.
    SoundVoiceOttotto,
    /// Play a random eating voice clip.
    SoundVoiceEating,

    /// Set a specified value to an int variable.
    IntVariableSet { value: i32, variable: VariableAst },
    /// Add a specified value to an int variable.
    IntVariableAdd { value: i32, variable: VariableAst },
    /// Subtract a specified value from an int variable.
    IntVariableSubtract { value: i32, variable: VariableAst },
    /// Increment an int variable.
    IntVariableIncrement { variable: VariableAst },
    /// Decrement an int variable.
    IntVariableDecrement { variable: VariableAst },

    /// Set a specified value to a float variable.
    FloatVariableSet { value: FloatValue, variable: VariableAst },
    /// Add a specified value to a float variable.
    FloatVariableAdd { value: FloatValue, variable: VariableAst },
    /// Subtract a specified value from a float variable.
    FloatVariableSubtract { value: FloatValue, variable: VariableAst },
    /// Multiply a specified value on a float variable.
    FloatVariableMultiply { value: FloatValue, variable: VariableAst },
    /// Divide a specified value on a float variable.
    FloatVariableDivide { value: FloatValue, variable: VariableAst },

    /// Set a bool variable to true.
    BoolVariableSetTrue { variable: VariableAst },
    /// Set a bool variable to false.
    BoolVariableSetFalse { variable: VariableAst },

    /// Changes the visibility of certain bones attached to objects. Uses bone groups and switches set in the specified Reference of the Model Visibility section.
    ModelChanger { reference: u8, switch_index: i32, bone_group_index: i32 },
    /// Generate a generic graphical effect with the specified parameters.
    GraphicEffect (GraphicEffect),
    /// Generate a graphical effect from an external file. (usually the Ef_ file)
    ExternalGraphicEffect (ExternalGraphicEffect),
    /// Tint the screen to the specified color.
    LimitedScreenTint (LimitedScreenTint),
    /// Tint the screen to the specified color until terminated by `EndUnlimitedScreenTint`.
    UnlimitedScreenTint (UnlimitedScreenTint),
    /// Ends an unlimited screen tint with the specified ID.
    EndUnlimitedScreenTint { tint_id: i32, transition_out_time: i32 },
    /// Creates glow of sword. Only usable when the proper effects are loaded by their respective characters.
    SwordGlow (SwordGlow),
    /// Remove the sword flow in the specified time
    DeleteSwordGlow { fade_time: i32 },
    /// Moves nearby movable model parts (capes, hair, etc) with a wind specified by the parameters.
    AestheticWindEffect (AestheticWindEffect),
    /// Ends the wind effect spawned by the "Aesthetic Wind Effect" event
    EndAestheticWindEffect { unk: i32 },
    /// Shakes the screen.
    ScreenShake { magnitude: i32 },
    /// Zoom the camera on the character.
    CameraCloseup (CameraCloseup),
    /// Return the camera to its normal settings.
    CameraNormal,
    /// Cause the character to receive the closest item in range.
    ItemPickup { unk1: i32, unk2: Option<i32>, unk3: Option<i32>, unk4: Option<i32> },
    /// Cause the character to throw the currently held item.
    ItemThrow { unk1: VariableAst, unk2: VariableAst, unk3: VariableAst, unk4: Option<VariableAst>, unk5: Option<VariableAst> },
    /// Cause the character to throw the currently held item.
    ItemThrow2 { unk1: f32, unk2: f32, unk3: VariableAst },
    /// Cause the character to drop any currently held item.
    ItemDrop,
    /// Cause the character to consume the currently held item.
    ItemConsume { unk: i32 },
    /// Set a property of the currently held item.
    ItemSetProperty { unk1: i32, unk2: f32 },
    /// Fires a shot from the currently held item.
    FireWeapon,
    /// Fires a projectile.
    FireProjectile,
    /// Used when firing a cracker launcher.
    Item1F { unk: i32 },
    /// Create an item in the characters hand.
    ItemCreate { unk: i32 },
    /// Determines the visibility of the currently held item.
    ItemVisibility (bool),
    /// Creates a beam sword trail. Probably has more uses among battering weapons.
    BeamSwordTrail { unk: i32 },
    /// Unknown event.
    Unknown (Event)
}

#[derive(Serialize, Clone, Debug)]
pub enum FloatValue {
    Variable (VariableAst),
    Constant (f32),
}

#[derive(Serialize, Clone, Debug)]
pub struct Block {
    pub events: Vec<EventAst>
}

#[derive(Serialize, Clone, Debug)]
pub struct ForLoop {
    pub iterations: Iterations,
    pub block: Block,
}

#[derive(Serialize, Clone, Debug)]
pub enum Iterations {
    Finite (i32),
    Infinite
}

#[derive(Serialize, Clone, Debug)]
pub struct IfStatement {
    pub test: Expression,
    pub then_branch: Block,
    pub else_branch: Option<Box<Block>>
}

#[derive(Serialize, Clone, Debug)]
pub enum Expression {
    Nullary  (Requirement),
    Unary    (UnaryExpression),
    Binary   (BinaryExpression),
    Not      (Box<Expression>),
    Variable (VariableAst),
    Value    (i32),
    Scalar   (f32),
}

#[derive(Serialize, Clone, Debug)]
pub struct BinaryExpression {
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub operator: ComparisonOperator
}

#[derive(Serialize, Clone, Debug)]
pub struct UnaryExpression {
    pub requirement: Requirement,
    pub value: Box<Expression>,
}

#[derive(Serialize, Clone, Debug)]
pub enum ComparisonOperator {
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    GreaterThan,
    And,
    Or,
    UnknownArg (i32)
}

impl ComparisonOperator {
    fn from_arg(value: i32) -> ComparisonOperator {
        match value {
            0 => ComparisonOperator::LessThan,
            1 => ComparisonOperator::LessThanOrEqual,
            2 => ComparisonOperator::Equal,
            3 => ComparisonOperator::NotEqual,
            4 => ComparisonOperator::GreaterThanOrEqual,
            5 => ComparisonOperator::GreaterThan,
            v => ComparisonOperator::UnknownArg (v),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum EdgeSlide {
    SlideOff,
    StayOn,
    Airbourne,
    Unknown (i32)
}

impl EdgeSlide {
    fn new(value: i32) -> EdgeSlide {
        match value {
            0 => EdgeSlide::SlideOff,
            1 => EdgeSlide::StayOn,
            5 => EdgeSlide::Airbourne,
            v => EdgeSlide::Unknown (v)
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum HurtBoxState {
    Normal,
    Invincible,
    IntangibleFlashing,
    IntangibleNoFlashing,
    IntangibleQuickFlashing,
    Unknown (i32)
}

impl HurtBoxState {
    fn new(value: i32) -> HurtBoxState {
        match value {
            0 => HurtBoxState::Normal,
            1 => HurtBoxState::Invincible,
            2 => HurtBoxState::IntangibleFlashing,
            3 => HurtBoxState::IntangibleNoFlashing,
            4 => HurtBoxState::IntangibleQuickFlashing,
            v => HurtBoxState::Unknown (v)
        }
    }

    pub fn is_normal(&self) -> bool {
        match self {
            HurtBoxState::Normal => true,
            _ => false
        }
    }

    pub fn is_invincible(&self) -> bool {
        match self {
            HurtBoxState::Invincible => true,
            _ => false
        }
    }

    pub fn is_intangible(&self) -> bool {
        match self {
            HurtBoxState::IntangibleFlashing => true,
            HurtBoxState::IntangibleNoFlashing => true,
            HurtBoxState::IntangibleQuickFlashing => true,
            _ => false
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum AngleFlip {
    AwayFromAttacker,
    AttackerDir,
    AttackerDirReverse,
    FaceZaxis,
    Unknown (i32)
}

impl AngleFlip {
    fn new(value: i32) -> AngleFlip {
        match value {
            0 | 2 | 5 => AngleFlip::AwayFromAttacker,
            1 | 3     => AngleFlip::AttackerDir,
            4         => AngleFlip::AttackerDirReverse,
            6 | 7     => AngleFlip::FaceZaxis,
            v         => AngleFlip::Unknown (v),
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum HitBoxEffect {
    Normal,
    None,
    Slash,
    Electric,
    Freezing,
    Flame,
    Coin,
    Reverse,
    Trip,
    Sleep,
    //Unk1,
    Bury,
    Stun,
    //Unk2,
    Flower,
    //Unk3,
    //Unk4,
    Grass,
    Water,
    Darkness,
    Paralyze,
    Aura,
    Plunge,
    Down,
    Flinchless,
    Unknown (i32)
}

impl HitBoxEffect {
    fn new(value: i32) -> HitBoxEffect {
        match value {
             0 => HitBoxEffect::Normal,
             1 => HitBoxEffect::None,
             2 => HitBoxEffect::Slash,
             3 => HitBoxEffect::Electric,
             4 => HitBoxEffect::Freezing,
             5 => HitBoxEffect::Flame,
             6 => HitBoxEffect::Coin,
             7 => HitBoxEffect::Reverse,
             8 => HitBoxEffect::Trip,
             9 => HitBoxEffect::Sleep,
            //10 => HitBoxEffect::Unk1,
            11 => HitBoxEffect::Bury,
            12 => HitBoxEffect::Stun,
            //13 => HitBoxEffect::Unk2,
            14 => HitBoxEffect::Flower,
            //15 => HitBoxEffect::Unk3,
            //16 => HitBoxEffect::Unk4,
            17 => HitBoxEffect::Grass,
            18 => HitBoxEffect::Water,
            19 => HitBoxEffect::Darkness,
            20 => HitBoxEffect::Paralyze,
            21 => HitBoxEffect::Aura,
            22 => HitBoxEffect::Plunge,
            23 => HitBoxEffect::Down,
            24 => HitBoxEffect::Flinchless,
            v  => HitBoxEffect::Unknown (v),
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum HitBoxSound {
    Unique,
    Punch,
    Kick,
    Slash,
    Coin,
    HomeRunBat,
    Paper,
    Shock,
    Burn,
    Splash,
    Explosion,
    Thud,
    Slam,
    Thwomp,
    MagicZap,
    Shell,
    Slap,
    Pan,
    Club,
    Racket,
    Aura,
    NessBat,
    Unknown (i32)
}

impl HitBoxSound {
    fn new(value: i32) -> HitBoxSound {
        match value {
            0 => HitBoxSound::Unique,
            1 => HitBoxSound::Punch,
            2 => HitBoxSound::Kick,
            3 => HitBoxSound::Slash,
            4 => HitBoxSound::Coin,
            5 => HitBoxSound::HomeRunBat,
            6 => HitBoxSound::Paper,
            7 => HitBoxSound::Shock,
            8 => HitBoxSound::Burn,
            9 => HitBoxSound::Splash,
           11 => HitBoxSound::Explosion,
           13 => HitBoxSound::Thud,
           14 => HitBoxSound::Slam,
           15 => HitBoxSound::Thwomp,
           16 => HitBoxSound::MagicZap,
           17 => HitBoxSound::Shell,
           18 => HitBoxSound::Slap,
           19 => HitBoxSound::Pan,
           20 => HitBoxSound::Club,
           21 => HitBoxSound::Racket,
           22 => HitBoxSound::Aura,
           27 => HitBoxSound::NessBat,
            _ => HitBoxSound::Unknown (value)
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum HitBoxSseType {
    None,
    Head,
    Body,
    Butt,
    Hand,
    Elbow,
    Foot,
    Knee,
    Throwing,
    Weapon,
    Sword,
    Hammer,
    Explosive,
    Spin,
    Bite,
    Magic,
    Pk,
    Bow,
    NessBat,
    Umbrella,
    Pimin,
    Water,
    Whip,
    Tail,
    Energy,
    Unknown (i32)
}

impl HitBoxSseType {
    fn new(value: i32) -> HitBoxSseType {
        match value {
            0 => HitBoxSseType::None,
            1 => HitBoxSseType::Head,
            2 => HitBoxSseType::Body,
            3 => HitBoxSseType::Butt,
            4 => HitBoxSseType::Hand,
            5 => HitBoxSseType::Elbow,
            6 => HitBoxSseType::Foot,
            7 => HitBoxSseType::Knee,
            8 => HitBoxSseType::Throwing,
            9 => HitBoxSseType::Weapon,
           10 => HitBoxSseType::Sword,
           11 => HitBoxSseType::Hammer,
           12 => HitBoxSseType::Explosive,
           13 => HitBoxSseType::Spin,
           14 => HitBoxSseType::Bite,
           15 => HitBoxSseType::Magic,
           16 => HitBoxSseType::Pk,
           17 => HitBoxSseType::Bow,
         //18 => HitBoxSseType::Unk,
           19 => HitBoxSseType::NessBat,
           20 => HitBoxSseType::Umbrella,
           21 => HitBoxSseType::Pimin,
           22 => HitBoxSseType::Water,
           23 => HitBoxSseType::Whip,
           24 => HitBoxSseType::Tail,
           25 => HitBoxSseType::Energy,
            _ => HitBoxSseType::Unknown (value)
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct HitBoxArguments {
    pub bone_index:         i16,
    pub hitbox_id:          u8,
    pub set_id:             u8,
    pub damage:             FloatValue,
    pub trajectory:         i32,
    pub wdsk:               i16,
    pub kbg:                i16,
    pub shield_damage:      i16,
    pub bkb:                i16,
    pub size:               f32,
    pub x_offset:           f32,
    pub y_offset:           f32,
    pub z_offset:           f32,
    pub tripping_rate:      f32,
    pub hitlag_mult:        f32,
    pub sdi_mult:           f32,
    pub effect:             HitBoxEffect,
    pub unk1:               bool,
    pub sound_level:        u8,
    pub unk2:               bool,
    pub sound:              HitBoxSound,
    pub unk3:               u8,
    pub ground:             bool,
    pub aerial:             bool,
    pub unk4:               u8,
    pub sse_type:           HitBoxSseType,
    pub clang:              bool,
    pub unk5:               bool,
    pub direct:             bool,
    pub unk6:               u8,
}

#[derive(Serialize, Clone, Debug)]
pub struct SpecialHitBoxArguments {
    pub hitbox_args:       HitBoxArguments,
    pub rehit_rate:        i32,
    pub angle_flipping:    AngleFlip,
    pub unk1:              bool,
    pub stretches_to_bone: bool,
    pub unk2:              bool,
    /// Can hit fighters, waddle dee/doo and pikmin
    pub can_hit1:  bool,
    /// Can hit SSE enemies
    pub can_hit2:  bool,
    /// Unk
    pub can_hit3:  bool,
    /// Can hit ROB Gyro, Snake grenade and Mr Saturn
    pub can_hit4:  bool,
    /// Unk
    pub can_hit5:  bool,
    /// Unk
    pub can_hit6:  bool,
    /// Can hit Stage hurtboxes not including wall/ceiling/floor
    pub can_hit7:  bool,
    /// Can hit wall/ceiling/floor
    pub can_hit8:  bool,
    /// Link & Toon Link Bomb, Bob-omb
    pub can_hit9:  bool,
    /// Unk
    pub can_hit10: bool,
    /// Link & Toon Link Bomb, Bob-omb, ROB Gyro, Snake grenade, Bob-omb, Mr Saturn, All Stage related hurtboxes?
    pub can_hit11: bool,
    /// Waddle Dee/Doo pikmin
    pub can_hit12: bool,
    /// Unk
    pub can_hit13: bool,
    pub enabled:              bool,
    pub unk3:                 u8,
    pub can_be_shielded:      bool,
    pub can_be_reflected:     bool,
    pub can_be_absorbed:      bool,
    pub unk4:                 u8,
    pub remain_grabbed:       bool,
    pub ignore_invincibility: bool,
    pub freeze_frame_disable: bool,
    pub unk5:                 bool,
    pub flinchless:           bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct MoveHitBox {
    pub hitbox_id:    i32,
    pub new_bone:     i32,
    pub new_x_offset: f32,
    pub new_y_offset: f32,
    pub new_z_offset: f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct GrabBoxArguments {
    pub hitbox_id:    i32,
    pub bone_index:   i32,
    pub size:         f32,
    pub x_offset:     f32,
    pub y_offset:     f32,
    pub z_offset:     f32,
    pub set_action:   i32,
    pub target:       GrabTarget,
    pub unk:          Option<i32>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum GrabTarget {
    None,
    GroundedOnly,
    AerialOnly,
    AerialAndGrounded,
    Unknown (i32),
}

impl GrabTarget {
    fn new(value: i32) -> GrabTarget {
        match value {
            0 => GrabTarget::None,
            1 => GrabTarget::GroundedOnly,
            2 => GrabTarget::AerialOnly,
            3 => GrabTarget::AerialAndGrounded,
            v => GrabTarget::Unknown (v),
        }
    }

    pub fn grounded(&self) -> bool {
        match self {
            GrabTarget::GroundedOnly => true,
            GrabTarget::AerialAndGrounded => true,
            _ => false,
        }
    }

    pub fn aerial(&self) -> bool {
        match self {
            GrabTarget::AerialOnly => true,
            GrabTarget::AerialAndGrounded => true,
            _ => false,
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct SpecifyThrow {
    /// ID of throw data. Seemingly, a "0" indicates this is the throw data, while a "1" indicates this is used if the opponent escapes during the throw. "2" has also been seen (by Light Arrow)."
    pub throw_use:   ThrowUse,
    pub bone:        i32,
    pub damage:      i32,
    pub trajectory:  i32,
    pub kbg:         i32,
    pub wdsk:        i32,
    pub bkb:         i32,
    pub effect:      HitBoxEffect,
    pub unk0:        f32,
    pub unk1:        f32,
    pub unk2:        f32,
    pub unk3:        i32,
    pub sfx:         HitBoxSound,
    pub grab_target: GrabTarget,
    pub unk4:        bool,
    pub unk5:        bool,
    pub i_frames:    i32,
}

#[derive(Serialize, Clone, Debug)]
pub enum ThrowUse {
    Throw,
    GrabInterrupt,
    Unknown (i32),
}

impl ThrowUse {
    fn new(value: i32) -> ThrowUse {
        match value {
            0 => ThrowUse::Throw,
            1 => ThrowUse::GrabInterrupt,
            v => ThrowUse::Unknown (v),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ApplyThrow {
    pub unk0: i32,
    pub bone: i32,
    pub unk1: VariableAst,
    pub unk2: VariableAst,
    pub unk3: VariableAst,
}

#[derive(Serialize, Clone, Debug)]
pub enum LedgeGrabEnable {
    Disable,
    EnableInFront,
    EnableInFrontAndBehind,
    Unknown (i32),
}

impl LedgeGrabEnable {
    fn new(value: i32) -> LedgeGrabEnable {
        match value {
            0 => LedgeGrabEnable::Disable,
            1 => LedgeGrabEnable::EnableInFront,
            2 => LedgeGrabEnable::EnableInFrontAndBehind,
            v => LedgeGrabEnable::Unknown (v),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum ArmorType {
    None,
    SuperArmor,
    HeavyArmorKnockbackBased,
    HeavyArmorDamageBased,
    Unknown (i32),
}

impl ArmorType {
    fn new(value: i32) -> ArmorType {
        match value {
            0 => ArmorType::None,
            1 => ArmorType::SuperArmor,
            2 => ArmorType::HeavyArmorKnockbackBased,
            3 => ArmorType::HeavyArmorDamageBased,
            v => ArmorType::Unknown (v),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct SetOrAddVelocity {
    pub x_vel: f32,
    pub y_vel: f32,
    pub x_set: bool,
    pub y_set: bool,
}

#[derive(Serialize, Clone, Debug)]
pub enum DisableMovement {
    Enable,
    DisableVertical,
    DisableHorizontal,
    Unknown (i32),
}

impl DisableMovement {
    fn new(value: i32) -> DisableMovement {
        match value {
            0 => DisableMovement::Enable,
            1 => DisableMovement::DisableVertical,
            2 => DisableMovement::DisableHorizontal,
            v => DisableMovement::Unknown (v),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct GraphicEffect {
    pub graphic:                  i32,
    pub bone:                     i32,
    pub x_offset:                 f32,
    pub y_offset:                 f32,
    pub z_offset:                 f32,
    pub x_rotation:               f32,
    pub y_rotation:               f32,
    pub z_rotation:               f32,
    pub scale:                    f32,
    pub random_x_offset:          f32,
    pub random_y_offset:          f32,
    pub random_z_offset:          f32,
    pub random_x_rotation:        f32,
    pub random_y_rotation:        f32,
    pub random_z_rotation:        f32,
    pub terminate_with_animation: bool
}

#[derive(Serialize, Clone, Debug)]
pub struct ExternalGraphicEffect {
    pub file:                     i16,
    pub graphic:                  i16,
    pub bone:                     i32,
    pub x_offset:                 f32,
    pub y_offset:                 f32,
    pub z_offset:                 f32,
    pub x_rotation:               f32,
    pub y_rotation:               f32,
    pub z_rotation:               f32,
    pub scale:                    f32,
    pub randomize:                Option<ExternalGraphicEffectRandomize>,
    pub terminate_with_animation: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct ExternalGraphicEffectRandomize {
    pub random_x_offset:   f32,
    pub random_y_offset:   f32,
    pub random_z_offset:   f32,
    pub random_x_rotation: f32,
    pub random_y_rotation: f32,
    pub random_z_rotation: f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct LimitedScreenTint {
    pub transition_in_time: i32,
    pub red: i32,
    pub green: i32,
    pub blue: i32,
    pub alpha: i32,
    pub frame_count: i32,
    pub transition_out_time: i32,
}

#[derive(Serialize, Clone, Debug)]
pub struct UnlimitedScreenTint {
    pub tint_id: i32,
    pub transition_in_time: i32,
    pub red: i32,
    pub green: i32,
    pub blue: i32,
    pub alpha: i32,
}

#[derive(Serialize, Clone, Debug)]
pub struct SwordGlow {
    pub color:                  i32,
    pub blur_length:            i32,

    pub point1_bone:            i32,
    pub point1_x_offset:        f32,
    pub point1_y_offset:        f32,
    pub point1_z_offset:        f32,

    pub point2_bone:            i32,
    pub point2_x_offset:        f32,
    pub point2_y_offset:        f32,
    pub point2_z_offset:        f32,

    pub delete_after_subaction: bool,
    pub graphic_id:             i32,
    pub bone_id:                i32,
    pub x_offset:               f32,
    pub y_offset:               f32,
    pub z_offset:               f32,
    pub x_rotation:             f32,
    pub y_rotation:             f32,
    pub z_rotation:             f32,
    pub glow_length:            f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct AestheticWindEffect {
    pub unk1:    i32,
    pub unk2:    f32,
    pub stength: f32,
    pub speed:   f32,
    pub size:    f32,
    pub unk3:    f32,
    pub unk4:    f32,
    pub unk5:    f32,
    pub unk6:    f32,
    pub unk7:    f32,
    pub unk8:    i32,
}

#[derive(Serialize, Clone, Debug)]
pub struct Interrupt {
    pub interrupt_id: Option<i32>,
    pub action:       i32,
    pub test:         Expression
}

#[derive(Serialize, Clone, Debug)]
pub enum InterruptType {
    Main,
    GroundSpecial,
    GroundItem,
    GroundCatch,
    GroundAttack,
    GroundEscape,
    GroundGuard,
    GroundJump,
    GroundOther,
    AirLanding,
    CliffCatch,
    AirSpecial,
    AirItemThrow,
    AirLasso,
    AirDodge,
    AirAttack,
    AirTreadjump,
    AirWalljump,
    AirJump,
    /// Only works in squat
    PassThroughPlat,
    Unknown (i32),
}

impl InterruptType {
    pub fn new(value: i32) -> Self {
        match value {
            0x00 => InterruptType::Main,
            0x01 => InterruptType::GroundSpecial,
            0x02 => InterruptType::GroundItem,
            0x03 => InterruptType::GroundCatch,
            0x04 => InterruptType::GroundAttack,
            0x05 => InterruptType::GroundEscape,
            0x06 => InterruptType::GroundGuard,
            0x07 => InterruptType::GroundJump,
            0x08 => InterruptType::GroundOther,
            0x09 => InterruptType::AirLanding,
            0x0A => InterruptType::CliffCatch,
            0x0B => InterruptType::AirSpecial,
            0x0C => InterruptType::AirItemThrow,
            0x0D => InterruptType::AirLasso,
            0x0E => InterruptType::AirDodge,
            0x0F => InterruptType::AirAttack,
            0x10 => InterruptType::AirTreadjump,
            0x11 => InterruptType::AirWalljump,
            0x12 => InterruptType::AirJump,
            0x13 => InterruptType::PassThroughPlat,
            _ => InterruptType::Unknown (value)
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct CameraCloseup {
    pub zoom_time: i32,
    pub unk:       i32,
    pub distance:  f32,
    pub x_angle:   f32,
    pub y_angle:   f32,
}
