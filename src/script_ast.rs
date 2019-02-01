use crate::script::{Script, Event, Requirement, Argument, Variable};
use crate::script;

use std::iter::Iterator;
use std::slice;

#[derive(Serialize, Clone, Debug)]
pub struct ScriptAst {
    pub block:  Block,
    pub offset: u32,
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
    let mut boolean_expressions = vec!();
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
            (0x00, 0x07, Some(&Offset(v0)), None, None) => EventAst::Subroutine (v0),
            (0x00, 0x08, None,              None, None) => EventAst::Return,
            (0x00, 0x09, Some(&Offset(v0)), None, None) => EventAst::Goto (v0),
            (0x00, 0x0A, Some(&Requirement { ref ty, flip }), v1, v2) => { // If
                if let Some(mut test) = Expression::from_args(ty, flip, v1, v2, args.get(3)) {
                    match process_block(events) {
                        ProcessedBlock::EndIf { then_branch, boolean_expressions } => {
                            test.append_boolean_expressions(boolean_expressions);
                            EventAst::IfStatement (IfStatement { test, then_branch, else_branch: None })
                        }
                        ProcessedBlock::EndIfAndElse { then_branch, else_branch, boolean_expressions } => {
                            test.append_boolean_expressions(boolean_expressions);
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
                    ProcessedBlock::EndIf { then_branch: else_branch, .. } => {
                        let then_branch = Block { events: event_asts };
                        let else_branch = Some(Box::new(else_branch));
                        return ProcessedBlock::EndIfAndElse { then_branch, else_branch, boolean_expressions }
                    }
                    _ => {
                        error!("IfStatement did not terminate");
                        EventAst::Unknown (event.clone())
                    }
                }
            }
            (0x00, 0x0B, Some(&Requirement { ref ty, flip }), v1, v2) => { // And
                if let Some(right) = Expression::from_args(ty, flip, v1, v2, args.get(3)) {
                    boolean_expressions.push(AppendBooleanExpression { right, operator: ComparisonOperator::And });
                    continue;
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x00, 0x0C, Some(&Requirement { ref ty, flip }), v1, v2) => { // Or
                if let Some(right) = Expression::from_args(ty, flip, v1, v2, args.get(3)) {
                    boolean_expressions.push(AppendBooleanExpression { right, operator: ComparisonOperator::Or });
                    continue;
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x00, 0x0D, Some(&Requirement { ref ty, flip }), v1, v2) => { // Else If
                if let Some(mut test) = Expression::from_args(ty, flip, v1, v2, args.get(3)) {
                    let event = match process_block(events) {
                        ProcessedBlock::EndIf { then_branch, boolean_expressions } => {
                            test.append_boolean_expressions(boolean_expressions);
                            EventAst::IfStatement (IfStatement { test, then_branch, else_branch: None })
                        }
                        ProcessedBlock::EndIfAndElse { then_branch, else_branch, boolean_expressions } => {
                            test.append_boolean_expressions(boolean_expressions);
                            EventAst::IfStatement (IfStatement { test, then_branch, else_branch })
                        }
                        _ => {
                            error!("IfStatement did not terminate");
                            return ProcessedBlock::Finished (Block { events: event_asts });
                        }
                    };
                    let else_branch = Some(Box::new(Block { events: vec!(event) }));
                    let then_branch = Block { events: event_asts };
                    return ProcessedBlock::EndIfAndElse { then_branch, else_branch, boolean_expressions };
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x00, 0x0F, None, None, None) => { return ProcessedBlock::EndIf { then_branch: Block { events: event_asts }, boolean_expressions } }
            (0x00, 0x10, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::Switch (v0, v1),
            (0x00, 0x11, Some(&Value(v0)), None,             None) => EventAst::Case (v0),
            (0x00, 0x11, None,             None,             None) => EventAst::DefaultCase,
            (0x00, 0x13, None,             None,             None) => EventAst::EndSwitch,
            (0x01, 0x01, None,             None,             None) => EventAst::LoopRest,

            // change action
            (0x02, 0x06, Some(&Value(v0)), None,             None) => EventAst::EnableActionStatusID (v0),
            (0x02, 0x00, Some(&Value(v0)), Some(&Value(v1)), Some(&Requirement { ref ty, flip }))
             => EventAst::ChangeActionStatus { status_id: v0, action: v1, requirement: ty.clone(), flip },
            (0x02, 0x01, Some(&Value(v0)), Some(&Requirement { ref ty, flip }), v2) => {
                if let Some(mut test) = Expression::from_args(ty, flip, v2, args.get(3), args.get(4)) {
                    // Additional Change Action Requirements can be added in the next events
                    while let Some(peek_event) = events.peek() {
                        if peek_event.namespace == 0x02 && peek_event.code == 0x04 {
                            let args = &peek_event.arguments;
                            if let Some(&Requirement { ref ty, flip }) = args.get(0) {
                                if let Some(right) = Expression::from_args(ty, flip, args.get(1), args.get(2), args.get(3)) {
                                    test = Expression::Binary(BinaryExpression { left: Box::new(test), right: Box::new(right), operator: ComparisonOperator::And });
                                    events.next();
                                }
                                else {
                                    error!("Invalid Additional Change Action Requirement: Expression::from_args failed");
                                    return ProcessedBlock::Finished (Block { events: event_asts });
                                }
                            }
                            else {
                                error!("Invalid Additional Change Action Requirement: args.get(0) is not a requirement");
                                return ProcessedBlock::Finished (Block { events: event_asts });
                            }
                        }
                        else {
                            break;
                        }
                    }

                    EventAst::ChangeAction { action: v0, test }
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x02, 0x04, _,  _, _) => {
                error!("Additional Change Action Requirement without a previous Change Action");
                return ProcessedBlock::Finished (Block { events: event_asts });
            }
            (0x64, 0x00, None,              None,             None) => EventAst::AllowInterrupt,
            (0x04, 0x00, Some(&Value(v0)),  None,             None) => EventAst::ChangeSubactionRestartFrame (v0),
            (0x04, 0x00, Some(&Value(v0)),  Some(&Bool(v1)),  None) => if v1 { EventAst::ChangeSubaction (v0) } else { EventAst::ChangeSubactionRestartFrame (v0) }

            // timing
            (0x06, 0x06, Some(&Scalar(v0)), None,             None) => EventAst::SetFrame (v0),
            (0x04, 0x07, Some(&Scalar(v0)), None,             None) => EventAst::FrameSpeedModifier (v0),
            (0x0c, 0x23, Some(&Value(v0)),  Some(&Value(v1)), None) => EventAst::TimeManipulation (v0, v1),

            // misc state
            (0x0e, 0x00, Some(&Value(v0)), None, None) => EventAst::SetAirGround (v0),
            (0x08, 0x00, Some(&Value(v0)), None, None) => EventAst::SetEdgeSlide (EdgeSlide::new(v0)),
            (0x05, 0x00, None,             None, None) => EventAst::ReverseDirection,

            // hitboxes
            (0x06, 0x04, None,             None,             None) => EventAst::DeleteAllHitBoxes,
            (0x06, 0x00, Some(&Value(v0)), Some(&Value(v1)), Some(&Value(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12)) {
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Scalar(v11)), Some(&Value(v12))) => {
                        let v12u = v12 as u32;
                        EventAst::CreateHitBox (HitBoxArguments {
                            bone_index:         (v0 >> 16) as i16,
                            rehit_hitbox_index: (v0 >> 8)  as u8,
                            hitbox_index:        v0        as u8,
                            damage:              v1,
                            trajectory:          v2,
                            weight_knockback:   (v3 >> 16) as i16,
                            kbg:                 v3        as i16,
                            shield_damage:      (v4 >> 16) as i16,
                            bkb:                 v4        as i16,
                            size:                v5,
                            x_offset:            v6,
                            y_offset:            v7,
                            z_offset:            v8,
                            tripping_rate:       v9,
                            hitlag_mult:         v10,
                            di_mult:             v11,
                            effect:  Effect::new(v12 & 0b0000_0000_0000_0000_0000_0000_0001_1111),
                            unk1:               (v12 & 0b0000_0000_0000_0000_0000_0000_0010_0000) != 0,
                            sound_level:       ((v12 & 0b0000_0000_0000_0000_0011_1111_1100_0000) >> 6) as u8,
                            unk2:              ((v12 & 0b0000_0000_0000_0000_1100_0000_0000_0000) >> 14) as u8,
                            ground:             (v12 & 0b0000_0000_0000_0001_0000_0000_0000_0000) != 0,
                            aerial:             (v12 & 0b0000_0000_0000_0010_0000_0000_0000_0000) != 0,
                            unk3:              ((v12 & 0b0000_0000_0011_1100_0000_0000_0000_0000) >> 18) as u8,
                            ty:                ((v12 & 0b0000_0111_1100_0000_0000_0000_0000_0000) >> 22) as u8,
                            clang:              (v12 & 0b0000_1000_0000_0000_0000_0000_0000_0000) != 0,
                            unk4:               (v12 & 0b0001_0000_0000_0000_0000_0000_0000_0000) != 0,
                            direct:             (v12 & 0b0010_0000_0000_0000_0000_0000_0000_0000) != 0,
                            unk5:             ((v12u & 0b1100_0000_0000_0000_0000_0000_0000_0000) >> 30) as u8,
                        })
                    }
                    _ => EventAst::Unknown (event.clone())
                }
            }
            (0x06, 0x15, Some(&Value(v0)), Some(&Value(v1)), Some(&Value(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12), args.get(13), args.get(14)) {
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Scalar(v11)), Some(&Value(v12)), Some(&Value(v13)), Some(&Value(v14))) => {
                        let v12u = v12 as u32;
                        let v14u = v14 as u32;
                        EventAst::CreateSpecialHitBox (SpecialHitBoxArguments {
                            hitbox_args: HitBoxArguments {
                                bone_index:         (v0 >> 16) as i16,
                                rehit_hitbox_index: (v0 >> 8)  as u8,
                                hitbox_index:        v0        as u8,
                                damage:              v1,
                                trajectory:          v2,
                                weight_knockback:   (v3 >> 16) as i16,
                                kbg:                 v3        as i16,
                                shield_damage:      (v4 >> 16) as i16,
                                bkb:                 v4        as i16,
                                size:                v5,
                                x_offset:            v6,
                                y_offset:            v7,
                                z_offset:            v8,
                                tripping_rate:       v9,
                                hitlag_mult:         v10,
                                di_mult:             v11,
                                effect:  Effect::new(v12 & 0b0000_0000_0000_0000_0000_0000_0001_1111),
                                unk1:               (v12 & 0b0000_0000_0000_0000_0000_0000_0010_0000) != 0,
                                sound_level:       ((v12 & 0b0000_0000_0000_0000_0011_1111_1100_0000) >> 6) as u8,
                                unk2:              ((v12 & 0b0000_0000_0000_0000_1100_0000_0000_0000) >> 14) as u8,
                                ground:             (v12 & 0b0000_0000_0000_0001_0000_0000_0000_0000) != 0,
                                aerial:             (v12 & 0b0000_0000_0000_0010_0000_0000_0000_0000) != 0,
                                unk3:              ((v12 & 0b0000_0000_0011_1100_0000_0000_0000_0000) >> 18) as u8,
                                ty:                ((v12 & 0b0000_0111_1100_0000_0000_0000_0000_0000) >> 22) as u8,
                                clang:              (v12 & 0b0000_1000_0000_0000_0000_0000_0000_0000) != 0,
                                unk4:               (v12 & 0b0001_0000_0000_0000_0000_0000_0000_0000) != 0,
                                direct:             (v12 & 0b0010_0000_0000_0000_0000_0000_0000_0000) != 0,
                                unk5:             ((v12u & 0b1100_0000_0000_0000_0000_0000_0000_0000) >> 30) as u8,
                            },
                            rehit_rate: v13,
                            angle_flipping:    AngleFlip::new(v14 & 0b0000_0000_0000_0000_0000_0000_0000_0111),
                            unk1:                            (v14 & 0b0000_0000_0000_0000_0000_0000_0000_1000) != 0,
                            stretches:                       (v14 & 0b0000_0000_0000_0000_0000_0000_0001_0000) != 0,
                            unk2:                            (v14 & 0b0000_0000_0000_0000_0000_0000_0010_0000) != 0,
                            can_hit_multiplayer_characters:  (v14 & 0b0000_0000_0000_0000_0000_0000_0100_0000) != 0,
                            can_hit_sse_enemies:             (v14 & 0b0000_0000_0000_0000_0000_0000_1000_0000) != 0,
                            can_hit_unk1:                    (v14 & 0b0000_0000_0000_0000_0000_0001_0000_0000) != 0,
                            can_hit_unk2:                    (v14 & 0b0000_0000_0000_0000_0000_0010_0000_0000) != 0,
                            can_hit_unk3:                    (v14 & 0b0000_0000_0000_0000_0000_0100_0000_0000) != 0,
                            can_hit_unk4:                    (v14 & 0b0000_0000_0000_0000_0000_1000_0000_0000) != 0,
                            can_hit_unk5:                    (v14 & 0b0000_0000_0000_0000_0001_0000_0000_0000) != 0,
                            can_hit_damageable_ceilings:     (v14 & 0b0000_0000_0000_0000_0010_0000_0000_0000) != 0,
                            can_hit_damageable_walls:        (v14 & 0b0000_0000_0000_0000_0100_0000_0000_0000) != 0,
                            can_hit_damageable_floors:       (v14 & 0b0000_0000_0000_0000_1000_0000_0000_0000) != 0,
                            can_hit_unk6:                    (v14 & 0b0000_0000_0000_0001_0000_0000_0000_0000) != 0,
                            can_hit_unk7:                    (v14 & 0b0000_0000_0000_0010_0000_0000_0000_0000) != 0,
                            can_hit_unk8:                    (v14 & 0b0000_0000_0000_0100_0000_0000_0000_0000) != 0,
                            enabled:                         (v14 & 0b0000_0000_0000_1000_0000_0000_0000_0000) != 0,
                            unk3:                           ((v14 & 0b0000_0000_0011_0000_0000_0000_0000_0000) >> 20) as u8,
                            can_be_shielded:                 (v14 & 0b0000_0000_0100_0000_0000_0000_0000_0000) != 0,
                            can_be_reflected:                (v14 & 0b0000_0000_1000_0000_0000_0000_0000_0000) != 0,
                            can_be_absorbed:                 (v14 & 0b0000_0001_0000_0000_0000_0000_0000_0000) != 0,
                            unk4:                           ((v14 & 0b0000_0110_0000_0000_0000_0000_0000_0000) >> 25) as u8,
                            can_hit_gripped_character:       (v14 & 0b0000_1000_0000_0000_0000_0000_0000_0000) != 0, // TODO: I think?? brawlbox wording was weird
                            ignore_invincibility:            (v14 & 0b0001_0000_0000_0000_0000_0000_0000_0000) != 0,
                            freeze_frame_disable:            (v14 & 0b0010_0000_0000_0000_0000_0000_0000_0000) != 0,
                            unk5:                            (v14 & 0b0100_0000_0000_0000_0000_0000_0000_0000) != 0,
                            flinchless:                     (v14u & 0b1000_0000_0000_0000_0000_0000_0000_0000) != 0,
                        })
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

                    EventAst::CreateGrabBox(GrabBoxArguments {
                        hitbox_index: v0,
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

            // hurtboxes
            (0x06, 0x05, Some(&Value(v0)), None,             None) => EventAst::ChangeHurtBoxStateAll { state: HurtBoxState::new(v0) },
            (0x06, 0x08, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::ChangeHurtBoxStateSpecific { bone: v0, state: HurtBoxState::new(v1) },
            (0x06, 0x06, Some(&Value(v0)), None,             None) => {
                if v0 != 0 {
                    error!("Unsual UnchangeHurtBoxStateSpecific argument: All known cases of this event have an argument of 0")
                }
                EventAst::UnchangeHurtBoxStateSpecific
            }

            // misc
            (0x07, 0x07, Some(&Value(v0)),  Some(&Value(v1)),  None) => EventAst::Rumble { unk1: v0, unk2: v1 },
            (0x07, 0x0B, Some(&Value(v0)),  Some(&Value(v1)),  None) => EventAst::RumbleLoop { unk1: v0, unk2: v1 },
            (0x18, 0x00, Some(&Value(v0)),  None,              None) => EventAst::SlopeContourStand { leg_bone_parent: v0 },
            (0x18, 0x01, Some(&Value(v0)),  Some(&Value(v1)),  None) => EventAst::SlopeContourFull { hip_n_or_top_n: v0, trans_bone: v1 },
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
            (0x0E, 0x08, Some(&Scalar(v0)), Some(&Scalar(v1)), None) => EventAst::SetVelocity { x_vel: v0, y_vel: v1 },
            (0x0E, 0x01, Some(&Scalar(v0)), Some(&Scalar(v1)), None) => EventAst::AddVelocity { x_vel: v0, y_vel: v1 },
            (0x0E, 0x06, Some(&Value(v0)),  None,              None) => EventAst::DisableMovement (DisableMovement::new(v0)),
            (0x0E, 0x07, Some(&Value(v0)),  None,              None) => EventAst::DisableMovement2 (DisableMovement::new(v0)),
            (0x0E, 0x02, Some(&Value(v0)),  None,              None) => EventAst::ResetVerticalVelocityAndAcceleration (v0 == 1),

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
            (0x12, 0x00, Some(&Value(v0)),        Some(&Variable(ref v1)), None) => EventAst::IntVariableSet { value: v0, variable: v1.clone() },
            (0x12, 0x01, Some(&Value(v0)),        Some(&Variable(ref v1)), None) => EventAst::IntVariableAdd { value: v0, variable: v1.clone() },
            (0x12, 0x02, Some(&Value(v0)),        Some(&Variable(ref v1)), None) => EventAst::IntVariableSubtract { value: v0, variable: v1.clone() },
            (0x12, 0x03, Some(&Variable(ref v0)), None,                    None) => EventAst::IntVariableIncrement { variable: v0.clone() },
            (0x12, 0x04, Some(&Variable(ref v0)), None,                    None) => EventAst::IntVariableDecrement { variable: v0.clone() },
            (0x12, 0x06, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableSet { value: v0, variable: v1.clone() },
            (0x12, 0x07, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableAdd { value: v0, variable: v1.clone() },
            (0x12, 0x08, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableSubtract { value: v0, variable: v1.clone() },
            (0x12, 0x0F, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableMultiply { value: v0, variable: v1.clone() },
            (0x12, 0x10, Some(&Scalar(v0)),       Some(&Variable(ref v1)), None) => EventAst::FloatVariableDivide { value: v0, variable: v1.clone() },
            (0x12, 0x0A, Some(&Variable(ref v0)), None,                    None) => EventAst::BoolVariableSetTrue { variable: v0.clone() },
            (0x12, 0x0B, Some(&Variable(ref v0)), None,                    None) => EventAst::BoolVariableSetFalse { variable: v0.clone() },

            // graphics
            (0x0B, 0x00, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::ModelChanger { reference: 1, switch_index: v0, bone_group_index: v1 },
            (0x0B, 0x01, Some(&Value(v0)), Some(&Value(v1)), None) => EventAst::ModelChanger { reference: 2, switch_index: v0, bone_group_index: v1 },
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
            (0x1A, 0x00, Some(&Value(v0)), None, None) => EventAst::ScreenShake { magnitude: v0 },
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
                    &Argument::Variable(ref v1) => Expression::Variable(v1.clone()),
                    &Argument::Value(v1)        => Expression::Value(v1),
                    _ => {
                        error!("Unhandled else if statement case: value: {:?}", v1);
                        return None;
                    }
                });
                Expression::Unary (UnaryExpression { requirement: requirement.clone(), value })
            }
            (Some(v1), Some(&Argument::Value(v2)), Some(v3)) => {
                let left = Box::new(match v1 {
                    &Argument::Scalar(v1)       => Expression::Scalar(v1),
                    &Argument::Variable(ref v1) => Expression::Variable(v1.clone()),
                    &Argument::Value(v1)        => Expression::Value(v1),
                    _ => {
                        error!("Unhandled else if statement case: left");
                        return None;
                    }
                });
                let right = Box::new(match v3 {
                    &Argument::Scalar(v3)       => Expression::Scalar(v3),
                    &Argument::Variable(ref v3) => Expression::Variable(v3.clone()),
                    &Argument::Value(v3)        => Expression::Value(v3),
                    _ => {
                        error!("Unhandled else if statement case: right");
                        return None;
                    }
                });
                if let script::Requirement::Comparison = requirement {
                    Expression::Binary (BinaryExpression { left, right, operator: ComparisonOperator::from_arg(v2) })
                } else {
                    error!("Unhandled else if statement case: comparison");
                    return None;
                }
            }
            (v1, v2, v3) => {
                error!("Unhandled else if statement case: {:?} {:?} {:?}", v1, v2, v3);
                return None;
            }
        };

        Some(if flip { Expression::Not (Box::new(test)) } else { test })
    }
}

impl Expression {
    fn append_boolean_expressions(&mut self, boolean_expressions: Vec<AppendBooleanExpression>) {
        for boolean_expression in boolean_expressions {
            *self = Expression::Binary (BinaryExpression {
                left: Box::new(self.clone()),
                right: Box::new(boolean_expression.right),
                operator: boolean_expression.operator,
            });
        }
    }
}

struct AppendBooleanExpression {
    right:    Expression,
    operator: ComparisonOperator,
}

enum ProcessedBlock {
    Finished     (Block),
    EndForLoop   (Block),
    EndIf        { then_branch: Block, boolean_expressions: Vec<AppendBooleanExpression> },
    EndIfAndElse { then_branch: Block, else_branch: Option<Box<Block>>, boolean_expressions: Vec<AppendBooleanExpression> },
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
    Subroutine (i32),
    /// Return from a Subroutine.
    Return,
    /// Goto the event location specified and execute.
    Goto (i32),
    /// An expression decides which block of code to execute.
    IfStatement (IfStatement),
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
    /// Enables the given Status ID
    EnableActionStatusID (i32),
    /// Change the current action upon the specified requirement being met. (the requirement does not have to be met at the time this ID is executed - it can be used anytime after execution.)
    ChangeActionStatus { status_id: i32, action: i32, requirement: Requirement, flip: bool },
    /// Change the current action upon test being true. (the requirement does not have to be met at the time this ID is executed - it can be used anytime after execution.)
    ChangeAction { action: i32, test: Expression },
    /// Allow the current action to be interrupted by another action.
    AllowInterrupt,
    /// Change the current subaction.
    ChangeSubaction (i32),
    /// Change the current subaction, restarting the frame count.
    ChangeSubactionRestartFrame (i32),
    /// Changes the current frame of the animation. Does not change the frame of the subaction (i.e. timers and such are unaffected).
    SetFrame (f32),
    /// Dictates the frame speed of the subaction. Example: setting to 2 makes the animation and timers occur twice as fast.
    FrameSpeedModifier (f32),
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
    /// Set the state of all of the characters hurtboxes.
    ChangeHurtBoxStateAll { state: HurtBoxState },
    /// Sets the state of a characters specific hurtbox.
    ChangeHurtBoxStateSpecific { bone: i32, state: HurtBoxState },
    /// Sets the state of a characters specific hurtbox to the global value.
    UnchangeHurtBoxStateSpecific,
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
    AddVelocity { x_vel: f32, y_vel: f32 },
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
    IntVariableSet { value: i32, variable: Variable },
    /// Add a specified value to an int variable.
    IntVariableAdd { value: i32, variable: Variable },
    /// Subtract a specified value from an int variable.
    IntVariableSubtract { value: i32, variable: Variable },
    /// Increment an int variable.
    IntVariableIncrement { variable: Variable },
    /// Decrement an int variable.
    IntVariableDecrement { variable: Variable },
    /// Set a specified value to a float variable.
    FloatVariableSet { value: f32, variable: Variable },
    /// Add a specified value to a float variable.
    FloatVariableAdd { value: f32, variable: Variable },
    /// Subtract a specified value from a float variable.
    FloatVariableSubtract { value: f32, variable: Variable },
    /// Multiply a specified value on a float variable.
    FloatVariableMultiply { value: f32, variable: Variable },
    /// Divide a specified value on a float variable.
    FloatVariableDivide { value: f32, variable: Variable },
    /// Set a bool variable to true.
    BoolVariableSetTrue { variable: Variable },
    /// Set a bool variable to false.
    BoolVariableSetFalse { variable: Variable },
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
    /// Terminates an unlimited screen tint with the specified ID.
    EndUnlimitedScreenTint { tint_id: i32, transition_out_time: i32 },
    /// Creates glow of sword. Only usable when the proper effects are loaded by their respective characters.
    SwordGlow (SwordGlow),
    /// Remove the sword flow in the specified time
    DeleteSwordGlow { fade_time: i32 },
    /// Moves nearby movable model parts (capes, hair, etc) with a wind specified by the parameters.
    AestheticWindEffect (AestheticWindEffect),
    /// Shakes the screen.
    ScreenShake { magnitude: i32 },
    /// Unknown event.
    Unknown (Event)
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
    Variable (Variable),
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
}

#[derive(Serialize, Clone, Debug)]
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

#[derive(Serialize, Clone, Debug)]
pub enum Effect {
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
    Unk2,
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

impl Effect {
    fn new(value: i32) -> Effect {
        match value {
             0 => Effect::Normal,
             1 => Effect::None,
             2 => Effect::Slash,
             3 => Effect::Electric,
             4 => Effect::Freezing,
             5 => Effect::Flame,
             6 => Effect::Coin,
             7 => Effect::Reverse,
             8 => Effect::Trip,
             9 => Effect::Sleep,
            //10 => Effect::Unk1,
            11 => Effect::Bury,
            12 => Effect::Stun,
            //13 => Effect::Unk2,
            14 => Effect::Flower,
            //15 => Effect::Unk3,
            //16 => Effect::Unk4,
            17 => Effect::Grass,
            18 => Effect::Water,
            19 => Effect::Darkness,
            20 => Effect::Paralyze,
            21 => Effect::Aura,
            22 => Effect::Plunge,
            23 => Effect::Down,
            24 => Effect::Flinchless,
            v  => Effect::Unknown (v),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct HitBoxArguments {
    pub bone_index:         i16,
    pub hitbox_index:       u8,
    pub rehit_hitbox_index: u8,
    pub damage:             i32,
    pub trajectory:         i32,
    pub weight_knockback:   i16,
    pub kbg:                i16,
    pub shield_damage:      i16,
    pub bkb:                i16,
    pub size:               f32,
    pub x_offset:           f32,
    pub y_offset:           f32,
    pub z_offset:           f32,
    pub tripping_rate:      f32,
    pub hitlag_mult:        f32,
    pub di_mult:            f32,
    pub effect:             Effect,
    pub unk1:               bool,
    pub sound_level:        u8,
    pub unk2:               u8,
    pub ground:             bool,
    pub aerial:             bool,
    pub unk3:               u8,
    pub ty:                 u8,
    pub clang:              bool,
    pub unk4:               bool,
    pub direct:             bool,
    pub unk5:               u8,
}

#[derive(Serialize, Clone, Debug)]
pub struct SpecialHitBoxArguments {
    pub hitbox_args:                    HitBoxArguments,
    pub rehit_rate:                     i32,
    pub angle_flipping:                 AngleFlip,
    pub unk1:                           bool,
    pub stretches:                      bool,
    pub unk2:                           bool,
    pub can_hit_multiplayer_characters: bool,
    pub can_hit_sse_enemies:            bool,
    pub can_hit_unk1:                   bool,
    pub can_hit_unk2:                   bool,
    pub can_hit_unk3:                   bool,
    pub can_hit_unk4:                   bool,
    pub can_hit_unk5:                   bool,
    pub can_hit_damageable_ceilings:    bool,
    pub can_hit_damageable_walls:       bool,
    pub can_hit_damageable_floors:      bool,
    pub can_hit_unk6:                   bool,
    pub can_hit_unk7:                   bool,
    pub can_hit_unk8:                   bool,
    pub enabled:                        bool,
    pub unk3:                           u8,
    pub can_be_shielded:                bool,
    pub can_be_reflected:               bool,
    pub can_be_absorbed:                bool,
    pub unk4:                           u8,
    pub can_hit_gripped_character:      bool,
    pub ignore_invincibility:           bool,
    pub freeze_frame_disable:           bool,
    pub unk5:                           bool,
    pub flinchless:                     bool,
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
    pub hitbox_index: i32,
    pub bone_index:   i32,
    pub size:         f32,
    pub x_offset:     f32,
    pub y_offset:     f32,
    pub z_offset:     f32,
    pub set_action:   i32,
    pub target:       GrabTarget,
    pub unk:          Option<i32>,
}

#[derive(Serialize, Clone, Debug)]
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
    unk1:    i32,
    unk2:    f32,
    stength: f32,
    speed:   f32,
    size:    f32,
    unk3:    f32,
    unk4:    f32,
    unk5:    f32,
    unk6:    f32,
    unk7:    f32,
    unk8:    i32,
}
