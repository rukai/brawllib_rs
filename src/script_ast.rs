use script::{Event, Requirement};
use script;
use std::iter::{Iterator, Peekable};
use std::slice;

pub fn script_ast(events: &[Event]) -> ScriptAst {
    if let ProcessedBlock::Finished(events) = process_block(&mut events.iter().peekable()) {
        ScriptAst { events }
    } else {
        error!("A block in the script did not terminate.");
        ScriptAst { events: vec!() }
    }
}

fn process_block(events: &mut Peekable<slice::Iter<Event>>) -> ProcessedBlock { // TODO: Ditch the peekable
    let mut event_asts = vec!();
    while let Some(event) = events.next() {
        let args = &event.arguments;
        // TODO: For now just matching each variant of Argument enum individually.
        //       If it turns out that I need to be able to handle the same event ID with different argument types,
        //       then I can just match generically and then call a method on Argument to retrieve a sensible value
        use script::Argument::*;
        let event_ast = match (event.namespace, event.code, args.get(0), args.get(1), args.get(2)) {
            (0x00, 0x01, Some(&Scalar(v0)),                   None,                None) => EventAst::SyncWait (v0),
            (0x00, 0x02, None,                                None,                None) => EventAst::Nop,
            (0x00, 0x02, Some(&Scalar(v0)),                   None,                None) => EventAst::AsyncWait (v0),
            (0x00, 0x04, Some(&Scalar(v0)),                   None,                None) => EventAst::SetLoop (v0),
            (0x00, 0x05, None,                                None,                None) => EventAst::ExecuteLoop,
            (0x00, 0x07, Some(&File(v0)),                     None,                None) => EventAst::Subroutine (v0),
            (0x00, 0x08, None,                                None,                None) => EventAst::Return,
            (0x00, 0x09, Some(&File(v0)),                     None,                None) => EventAst::Goto (v0),
            (0x00, 0x0A, Some(&Requirement { ref ty, flip }), v1, v2) => { // If
                let test = match (v1, v2, args.get(3)) {
                    (None, None, None) => {
                        Expression::Nullary (ty.clone())
                    }
                    (Some(v1), None, None) => {
                        let value = Box::new(match v1 {
                            &Scalar(v1)   => Expression::Scalar(v1),
                            &Variable(v1) => Expression::Variable(v1),
                            &Value(v1)    => Expression::Value(v1),
                            _ => {
                                error!("Unhandled if statement case: value: {:?}", v1);
                                continue;
                            }
                        });
                        Expression::Unary (UnaryExpression { requirement: ty.clone(), value })
                    }
                    (Some(v1), Some(&Value(v2)), Some(v3)) => {
                        let left = Box::new(match v1 {
                            &Scalar(v1)   => Expression::Scalar(v1),
                            &Variable(v1) => Expression::Variable(v1),
                            &Value(v1)    => Expression::Value(v1),
                            _ => {
                                error!("Unhandled if statement case: left");
                                continue;
                            }
                        });
                        let right = Box::new(match v3 {
                            &Scalar(v3)   => Expression::Scalar(v3),
                            &Variable(v3) => Expression::Variable(v3),
                            &Value(v3)    => Expression::Value(v3),
                            _ => {
                                error!("Unhandled if statement case: right");
                                continue;
                            }
                        });
                        if let script::Requirement::Comparison = ty {
                            Expression::Binary (BinaryExpression { left, right, operator: ComparisonOperator::new(v2) })
                        } else {
                            error!("Unhandled if statement case: comparison");
                            continue;
                        }
                    }
                    (v1, v2, v3) => {
                        error!("Unhandled if statement case: {:?} {:?} {:?}", v1, v2, v3);
                        continue;
                    }
                };
                let test = if flip { Expression::Not (Box::new(test)) } else { test };

                match process_block(events) {
                    ProcessedBlock::EndIf ( then_branch ) => {
                        EventAst::IfStatement (IfStatement { test, then_branch, else_branch: None })
                    }
                    ProcessedBlock::EndIfAndElse { then_branch, else_branch } => {
                        EventAst::IfStatement (IfStatement { test, then_branch, else_branch }) // TODO: Maybe I can just return this in the first place, lol
                    }
                    _ => {
                        error!("IfStatement did not terminate");
                        EventAst::Unknown (event.clone())
                    }
                }
            }
            (0x00, 0x0E, None, None, None) => { // Else
                match process_block(events) {
                    ProcessedBlock::EndIf (else_branch) => {
                        let then_branch = event_asts;
                        let else_branch = Some(Box::new(EventAst::Block(else_branch)));
                        return ProcessedBlock::EndIfAndElse { then_branch, else_branch }
                    }
                    _ => {
                        error!("IfStatement did not terminate");
                        EventAst::Unknown (event.clone()) // TODO: Maybe return ProcessedBlock::Error here
                    }
                }
            }
            (0x00, 0x0B, Some(&Requirement { ref ty, flip }), Some(&Variable(v1)), Some(&Value(v2))) => { // And
                if let Some(&Variable(v3)) = args.get(3) {
                    info!("USE THESE {:?} {} {:?} {:?} {}", ty, flip, v1, v2, v3);
                    //EventAst::AndComparison (v0, v1, ComparisonOperator::new(v2), v3)
                    EventAst::Unknown (event.clone())
                } else {
                    EventAst::Unknown (event.clone())
                }
            }
            (0x00, 0x0D, Some(&Requirement { ref ty, flip }), v1, v2) => { // Else If
                let test = match (v1, v2, args.get(3)) {
                    (None, None, None) => {
                        Expression::Nullary(ty.clone())
                    }
                    (Some(v1), None, None) => {
                        let value = Box::new(match v1 {
                            &Scalar(v1)   => Expression::Scalar(v1),
                            &Variable(v1) => Expression::Variable(v1),
                            &Value(v1)    => Expression::Value(v1),
                            _ => {
                                error!("Unhandled else if statement case: value: {:?}", v1);
                                continue;
                            }
                        });
                        Expression::Unary (UnaryExpression { requirement: ty.clone(), value })
                    }
                    (Some(v1), Some(&Value(v2)), Some(v3)) => {
                        let left = Box::new(match v1 {
                            &Scalar(v1)   => Expression::Scalar(v1),
                            &Variable(v1) => Expression::Variable(v1),
                            &Value(v1)    => Expression::Value(v1),
                            _ => {
                                error!("Unhandled else if statement case: left");
                                continue;
                            }
                        });
                        let right = Box::new(match v3 {
                            &Scalar(v3)   => Expression::Scalar(v3),
                            &Variable(v3) => Expression::Variable(v3),
                            &Value(v3)    => Expression::Value(v3),
                            _ => {
                                error!("Unhandled else if statement case: right");
                                continue;
                            }
                        });
                        if let script::Requirement::Comparison = ty {
                            Expression::Binary (BinaryExpression { left, right, operator: ComparisonOperator::new(v2) })
                        } else {
                            error!("Unhandled else if statement case: comparison");
                            continue;
                        }
                    }
                    (v1, v2, v3) => {
                        error!("Unhandled else if statement case: {:?} {:?} {:?}", v1, v2, v3);
                        continue;
                    }
                };
                let test = if flip { Expression::Not (Box::new(test)) } else { test };

                let event = match process_block(events) {
                    ProcessedBlock::EndIf ( then_branch ) => {
                        EventAst::IfStatement (IfStatement { test, then_branch, else_branch: None })
                    }
                    ProcessedBlock::EndIfAndElse { then_branch, else_branch } => {
                        EventAst::IfStatement (IfStatement { test, then_branch, else_branch })
                    }
                    _ => {
                        error!("IfStatement did not terminate");
                        EventAst::Unknown (event.clone())
                    }
                };
                return ProcessedBlock::EndIfAndElse { then_branch: event_asts, else_branch: Some(Box::new(event)) }; // TODO: Need to think about this
            }
            (0x00, 0x0F, None, None, None) => { return ProcessedBlock::EndIf (event_asts) }
            (0x00, 0x10, Some(&Value(v0)),       Some(&Value(v1)),     None) => EventAst::Switch (v0, v1),
            (0x00, 0x13, None,                   None,                 None) => EventAst::EndSwitch,
            (0x04, 0x00, Some(&Value(v0)),       None,                 None) => EventAst::ChangeSubActionRestartFrame (v0), // TODO: Does the default case restart?
            (0x04, 0x00, Some(&Value(v0)),       Some(&Bool(v1)),      None) =>
                if v1 { EventAst::ChangeSubAction (v0) } else { EventAst::ChangeSubActionRestartFrame (v0) }
            (0x06, 0x06, Some(&Scalar(v0)),      None,                 None) => EventAst::SetFrame (v0),
            (0x04, 0x07, Some(&Scalar(v0)),      None,                 None) => EventAst::FrameSpeedModifier (v0),
            (0x0c, 0x23, Some(&Value(v0)),       Some(&Value(v1)),     None) => EventAst::TimeManipulation (v0, v1),
            (0x0e, 0x00, Some(&Value(v0)),       None,                 None) => EventAst::SetAirGround (v0),
            (0x08, 0x00, Some(&Value(v0)),       None,                 None) => EventAst::SetEdgeSlide (EdgeSlide::new(v0)),
            (0x05, 0x00, None,                   None,                 None) => EventAst::ReverseDirection,
            (0x06, 0x04, None,                   None,                 None) => EventAst::RemoveAllHitBoxes,
            (0x64, 0x00, None,                   None,                 None) => EventAst::AllowInterrupt,
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

            // graphics
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
    ProcessedBlock::Finished(event_asts)
}

enum ProcessedBlock {
    Finished     (Vec<EventAst>),
    EndIf        (Vec<EventAst>),
    EndIfAndElse { then_branch: Vec<EventAst>, else_branch: Option<Box<EventAst>> }, // contains previous block
    //And      (Expression),
    //Or       (Expression),
}


/// An Abstract Syntax Tree representation of scripts
#[derive(Clone, Debug)]
pub struct ScriptAst {
    pub events: Vec<EventAst>
}

#[derive(Clone, Debug)]
pub enum EventAst {
    Block (Vec<EventAst>),
    SyncWait (f32),
    Nop,
    AsyncWait (f32),
    SetLoop (f32),
    ExecuteLoop,
    Subroutine (i32),
    Return,
    Goto (i32),
    IfStatement (IfStatement),
    IfValue (i32, i32),
    IfComparison (i32, i32, ComparisonOperator, i32),
    Else,
    AndComparison (i32, i32, ComparisonOperator, i32),
    ElseIfComparison (i32, i32, ComparisonOperator, i32),
    Switch (i32, i32),
    EndSwitch,
    AllowInterrupt,
    ChangeSubAction (i32),
    ChangeSubActionRestartFrame (i32),
    SetFrame (f32),
    FrameSpeedModifier (f32),
    TimeManipulation (i32, i32),
    SetAirGround (i32),
    SetEdgeSlide (EdgeSlide),
    ReverseDirection,
    CreateHitBox (HitBoxArguments), // brawlbox calls this "Offensive Collision"
    RemoveAllHitBoxes, // brawlbox calls this "Terminate Collisions"
    CreateSpecialHitBox (SpecialHitBoxArguments), // brawlbox calls this "Special Offensive Collision"
    MoveHitBox (MoveHitBox),
    ChangeHitBoxDamage { hitbox_id: i32, new_damage: i32 },
    ChangeHitBoxSize   { hitbox_id: i32, new_size:   i32 },
    DeleteHitBox (i32),
    SoundEffect1 (i32),
    SoundEffect2 (i32),
    SoundEffectTransient (i32),
    SoundEffectStop (i32),
    SoundEffectVictory (i32),
    SoundEffectUnk (i32),
    SoundEffectOther1 (i32),
    SoundEffectOther2 (i32),
    SoundVoiceLow,
    SoundVoiceDamage,
    SoundVoiceOttotto,
    SoundVoiceEating,
    GraphicEffect (GraphicEffect),
    Unknown (Event)
}

#[derive(Clone, Debug)]
pub struct IfStatement {
    test: Expression,
    then_branch: Vec<EventAst>,
    /// EventAst can only be IfStatement or Block
    else_branch: Option<Box<EventAst>>
}

#[derive(Clone, Debug)]
pub enum Expression {
    Nullary  (Requirement),
    Unary    (UnaryExpression),
    Binary   (BinaryExpression),
    Not      (Box<Expression>),
    Variable (i32),
    Value    (i32),
    Scalar   (f32),
}

#[derive(Clone, Debug)]
pub struct BinaryExpression {
    left: Box<Expression>,
    right: Box<Expression>,
    operator: ComparisonOperator
}

#[derive(Clone, Debug)]
pub struct UnaryExpression {
    requirement: Requirement,
    value: Box<Expression>,
}

#[derive(Clone, Debug)]
pub enum ComparisonOperator {
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    GreaterThan,
    Unknown (i32)
}

impl ComparisonOperator {
    fn new(value: i32) -> ComparisonOperator {
        match value {
            0 => ComparisonOperator::LessThan,
            1 => ComparisonOperator::LessThanOrEqual,
            2 => ComparisonOperator::Equal,
            3 => ComparisonOperator::NotEqual,
            4 => ComparisonOperator::GreaterThanOrEqual,
            5 => ComparisonOperator::GreaterThan,
            v => ComparisonOperator::Unknown (v),
        }
    }
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct MoveHitBox {
    pub hitbox_id:    i32,
    pub new_bone:     i32,
    pub new_x_offset: f32,
    pub new_y_offset: f32,
    pub new_z_offset: f32,
}

#[derive(Clone, Debug)]
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
