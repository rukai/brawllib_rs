use script::Event;

pub fn script_ast(events: &[Event]) -> ScriptAst {
    let mut event_asts = vec!();
    for event in events {
        let args = &event.arguments;
        // TODO: For now just matching each variant of Argument enum individually.
        //       If it turns out that I need to be able to handle the same event ID with different argument types,
        //       then I can just match generically and then call a method on Argument to retrieve a sensible value
        use script::Argument::*;
        let event_ast = match (event.namespace, event.code, args.get(0), args.get(1), args.get(2)) {
            (0x00, 0x01, Some(&Scalar(v0)),      None,                 None) => EventAst::SyncWait (v0),
            (0x00, 0x02, None,                   None,                 None) => EventAst::Nop,
            (0x00, 0x02, Some(&Scalar(v0)),      None,                 None) => EventAst::AsyncWait (v0),
            (0x00, 0x04, Some(&Scalar(v0)),      None,                 None) => EventAst::SetLoop (v0),
            (0x00, 0x05, None,                   None,                 None) => EventAst::ExecuteLoop,
            (0x00, 0x07, Some(&File(v0)),        None,                 None) => EventAst::Subroutine (v0),
            (0x00, 0x08, None,                   None,                 None) => EventAst::Return,
            (0x00, 0x09, Some(&File(v0)),        None,                 None) => EventAst::Goto (v0),
            (0x00, 0x0A, Some(&Requirement(v0)), None,                 None) => EventAst::If (v0),
            (0x00, 0x0A, Some(&Requirement(v0)), Some(&Value(v1)),     None) => EventAst::IfValue (v0, v1),
            (0x00, 0x0A, Some(&Requirement(v0)), Some(&EnumValue(v1)), Some(&Value(v2))) => {
                if let Some(&EnumValue(v3)) = args.get(3) {
                    EventAst::IfComparison (v0, v1, v2, v3)
                } else {
                    EventAst::Unknown
                }
            }
            (0x00, 0x0E, None,                   None,                 None) => EventAst::Else,
            // TODO: ...
            (0x04, 0x00, Some(&Value(v0)),       None,                 None) => EventAst::ChangeSubActionRestartFrame (v0), // TODO: Does the default case restart?
            (0x04, 0x00, Some(&Value(v0)),       Some(&Bool(v1)),      None) =>
                if v1 { EventAst::ChangeSubAction (v0) } else { EventAst::ChangeSubActionRestartFrame (v0) }
            (0x05, 0x00, None,                   None,                 None) => EventAst::ReverseDirection,
            (0x06, 0x04, None,                   None,                 None) => EventAst::RemoveAllHitBoxes,
            // TODO: ...
            (0x64, 0x00, None,                   None,                 None) => EventAst::AllowInterrupt,
            // TODO: ...
            (0x06, 0x00, Some(&Value(v0)), Some(&Value(v1)), Some(&Value(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12)) {
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Scalar(v11)), Some(&Value(v12))) => {
                        let v12u = v12 as u32;
                        EventAst::CreateHitBox (HitBoxArguments {
                            bone_index: (v0 >> 16) as i16,
                            hitbox_index: v0 as i16,
                            damage: v1,
                            trajectory: v2,
                            weight_knockback: (v3 >> 16) as i16,
                            kbg: v3 as i16,
                            shield_damage: (v4 >> 16) as i16,
                            bkb: v4 as i16,
                            size: v5,
                            x_offset: v6,
                            y_offset: v7,
                            z_offset: v8,
                            tripping_rate: v9,
                            hitlag_mult: v10,
                            di_mult: v11,
                            effect:  (v12 & 0b0000_0000_0000_0000_0000_0000_0001_1111) as u8,
                            unk1:    (v12 & 0b0000_0000_0000_0000_0000_0000_0010_0000) != 0,
                            sound:  ((v12 & 0b0000_0000_0000_0000_0011_1111_1100_0000) >> 6) as u8,
                            unk2:   ((v12 & 0b0000_0000_0000_0000_1100_0000_0000_0000) >> 14) as u8,
                            ground:  (v12 & 0b0000_0000_0000_0001_0000_0000_0000_0000) != 0,
                            aerial:  (v12 & 0b0000_0000_0000_0010_0000_0000_0000_0000) != 0,
                            unk3:   ((v12 & 0b0000_0000_0011_1100_0000_0000_0000_0000) >> 18) as u8,
                            ty:     ((v12 & 0b0000_0111_1100_0000_0000_0000_0000_0000) >> 22) as u8,
                            clang:   (v12 & 0b0000_1000_0000_0000_0000_0000_0000_0000) != 0,
                            unk4:    (v12 & 0b0001_0000_0000_0000_0000_0000_0000_0000) != 0,
                            direct:  (v12 & 0b0010_0000_0000_0000_0000_0000_0000_0000) != 0,
                            unk5:  ((v12u & 0b1100_0000_0000_0000_0000_0000_0000_0000) >> 30) as u8,
                        })
                    }
                    _ => EventAst::Unknown
                }
            }
            // TODO: ...
            (0x06, 0x15, Some(&Value(v0)), Some(&Value(v1)), Some(&Value(v2))) => {
                match (args.get(3), args.get(4), args.get(5), args.get(6), args.get(7), args.get(8), args.get(9), args.get(10), args.get(11), args.get(12), args.get(13), args.get(14)) {
                    (Some(&Value(v3)), Some(&Value(v4)), Some(&Scalar(v5)), Some(&Scalar(v6)), Some(&Scalar(v7)), Some(&Scalar(v8)), Some(&Scalar(v9)), Some(&Scalar(v10)), Some(&Scalar(v11)), Some(&Value(v12)), Some(&Value(v13)), Some(&Value(v14))) => {
                        let v12u = v12 as u32;
                        let v14u = v14 as u32;
                        EventAst::CreateSpecialHitBox (SpecialHitBoxArguments {
                            hitbox_args: HitBoxArguments {
                                bone_index: (v0 >> 16) as i16,
                                hitbox_index: v0 as i16,
                                damage: v1,
                                trajectory: v2,
                                weight_knockback: (v3 >> 16) as i16,
                                kbg: v3 as i16,
                                shield_damage: (v4 >> 16) as i16,
                                bkb: v4 as i16,
                                size: v5,
                                x_offset: v6,
                                y_offset: v7,
                                z_offset: v8,
                                tripping_rate: v9,
                                hitlag_mult: v10,
                                di_mult: v11,
                                effect:  (v12 & 0b0000_0000_0000_0000_0000_0000_0001_1111) as u8,
                                unk1:    (v12 & 0b0000_0000_0000_0000_0000_0000_0010_0000) != 0,
                                sound:  ((v12 & 0b0000_0000_0000_0000_0011_1111_1100_0000) >> 6) as u8,
                                unk2:   ((v12 & 0b0000_0000_0000_0000_1100_0000_0000_0000) >> 14) as u8,
                                ground:  (v12 & 0b0000_0000_0000_0001_0000_0000_0000_0000) != 0,
                                aerial:  (v12 & 0b0000_0000_0000_0010_0000_0000_0000_0000) != 0,
                                unk3:   ((v12 & 0b0000_0000_0011_1100_0000_0000_0000_0000) >> 18) as u8,
                                ty:     ((v12 & 0b0000_0111_1100_0000_0000_0000_0000_0000) >> 22) as u8,
                                clang:   (v12 & 0b0000_1000_0000_0000_0000_0000_0000_0000) != 0,
                                unk4:    (v12 & 0b0001_0000_0000_0000_0000_0000_0000_0000) != 0,
                                direct:  (v12 & 0b0010_0000_0000_0000_0000_0000_0000_0000) != 0,
                                unk5:  ((v12u & 0b1100_0000_0000_0000_0000_0000_0000_0000) >> 30) as u8,
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
                    _ => EventAst::Unknown
                }
            }
            _ => EventAst::Unknown
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
    ScriptAst { events: event_asts }
}

/// An Abstract Syntax Tree representation of scripts
#[derive(Clone, Debug)]
pub struct ScriptAst {
    pub events: Vec<EventAst>
}

#[derive(Clone, Debug)]
pub enum EventAst {
    SyncWait (f32),
    Nop,
    AsyncWait (f32),
    SetLoop (f32),
    ExecuteLoop,
    Subroutine (i32),
    Return,
    Goto (i32),
    If (i32),
    IfValue (i32, i32),
    IfComparison (i32, i32, i32, i32),
    Else,
    AllowInterrupt,
    ChangeSubAction (i32),
    ChangeSubActionRestartFrame (i32),
    ReverseDirection,
    CreateHitBox (HitBoxArguments), // brawlbox calls this "Offensive Collision"
    RemoveAllHitBoxes, // brawlbox calls this "Terminate Collisions"
    CreateSpecialHitBox (SpecialHitBoxArguments), // brawlbox calls this "Special Offensive Collision"
    Unknown,
}

#[derive(Clone, Debug)]
pub struct HitBoxArguments {
    pub bone_index: i16,
    pub hitbox_index: i16,
    pub damage: i32,
    pub trajectory: i32,
    pub weight_knockback: i16,
    pub kbg: i16,
    pub shield_damage: i16,
    pub bkb: i16,
    pub size: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub z_offset: f32,
    pub tripping_rate: f32,
    pub hitlag_mult: f32,
    pub di_mult: f32,
    pub effect: u8,
    pub unk1: bool,
    pub sound: u8,
    pub unk2: u8,
    pub ground: bool,
    pub aerial: bool,
    pub unk3: u8,
    pub ty: u8,
    pub clang: bool,
    pub unk4: bool,
    pub direct: bool,
    pub unk5: u8,
}

#[derive(Clone, Debug)]
pub enum AngleFlip {
    AwayFromAttacker,
    AttackerDir,
    AttackerDirReverse,
    FaceZaxis,
}

impl AngleFlip {
    fn new(value: i32) -> AngleFlip {
        match value {
            0 | 2 | 5 => AngleFlip::AwayFromAttacker,
            1 | 3     => AngleFlip::AttackerDir,
            4         => AngleFlip::AttackerDirReverse,
            6 | 7     => AngleFlip::FaceZaxis,
            _ => unreachable!()
        }
    }
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
