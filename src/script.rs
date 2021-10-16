use fancy_slice::FancySlice;

use crate::wii_memory::WiiMemory;

pub(crate) fn scripts(
    parent_data: FancySlice,
    offset_data: FancySlice,
    num: usize,
    wii_memory: &WiiMemory,
) -> Vec<Script> {
    let mut result = vec![];
    for i in 0..num {
        let offset = offset_data.u32_be(i * 4);
        result.push(new_script(parent_data, offset, wii_memory));
    }
    result
}

/// finds any scripts that are pointed to by Goto's and Subroutines but dont exist yet.
pub(crate) fn fragment_scripts(
    parent_data: FancySlice,
    known_scripts: &[&[Script]],
    ignore_origins: &[i32],
    wii_memory: &WiiMemory,
) -> Vec<Script> {
    let mut fragments: Vec<Script> = vec![];
    for scripts in known_scripts.iter() {
        for script in scripts.iter() {
            for event in &script.events {
                let mut found_offset = None;
                if event.namespace == 0x00 && (event.code == 0x07 || event.code == 0x09) {
                    // if the event is a subroutine or goto
                    if let Some(Argument::Offset(Offset { offset, origin })) =
                        event.arguments.get(0)
                    {
                        if !ignore_origins.contains(origin) {
                            found_offset = Some(*offset);
                        }
                    }

                    if let Some(Argument::Value(offset)) = event.arguments.get(0) {
                        found_offset = Some(*offset);
                    }
                }
                if event.namespace == 0x0D && (event.code == 0x00 || event.code == 0x05) {
                    // if the event is a CallEveryFrame or IndependentSubroutine
                    if let Some(Argument::Offset(Offset { offset, origin })) =
                        event.arguments.get(1)
                    {
                        if !ignore_origins.contains(origin) {
                            found_offset = Some(*offset);
                        }
                    }
                }
                if let Some(offset) = found_offset {
                    let mut is_action = false;
                    'outer: for check_scripts in known_scripts.iter() {
                        for check_script in check_scripts.iter() {
                            if check_script.offset == offset {
                                is_action = true;
                                break 'outer;
                            }
                        }
                    }
                    let already_added = fragments.iter().any(|x| x.offset == offset);

                    if !is_action && !already_added {
                        fragments.push(new_script(parent_data, offset as u32, wii_memory));
                    }
                }
            }
        }
    }

    if fragments.len() > 0 {
        // the fragment scripts may refer to their own fragment scripts
        let mut all = known_scripts.to_vec();
        all.push(&fragments);
        let inner_fragments = fragment_scripts(parent_data, &all, ignore_origins, wii_memory);
        fragments.extend(inner_fragments);
    }
    fragments
}

pub fn new_script(parent_data: FancySlice, offset: u32, wii_memory: &WiiMemory) -> Script {
    let buffer = if offset == 0 || offset as i32 == -1 {
        return Script {
            events: vec![],
            offset: offset as i32,
        };
    } else if offset > 0 && offset < (parent_data.len() as u32) {
        parent_data.relative_fancy_slice(offset as usize..)
    } else if offset < 0x8000_0000 {
        return Script {
            events: vec![],
            offset: offset as i32,
        };
    } else {
        wii_memory.fancy_slice_from(offset as usize)
    };

    let mut events = vec![];
    let mut event_offset = 0;
    loop {
        let namespace = buffer.u8(event_offset as usize);
        let code = buffer.u8(event_offset as usize + 1);
        let num_arguments = buffer.u8(event_offset as usize + 2);
        let unk1 = buffer.u8(event_offset as usize + 3);
        let raw_id = buffer.u32_be(event_offset as usize);

        if code == 0 && namespace == 0 {
            // end of script
            break;
        }

        // PSA fills empty space with these bytes:
        // const long FADEDATA = 0xFADE0D8A; // Constant for the tag FADE0D8A representing the end of useable space.
        // const long FADEFOOD = 0xFADEF00D; // Constant for the tag FADEF00D representing empty, useable space.
        if raw_id != 0xFADEF00D && raw_id != 0xFADE0D8A {
            let argument_offset = buffer.u32_be(event_offset as usize + 4);

            let argument_buffer = if argument_offset as usize >= parent_data.len() {
                wii_memory.fancy_slice_from(argument_offset as usize)
            } else {
                parent_data.relative_fancy_slice(argument_offset as usize..)
            };

            let arguments = arguments(argument_buffer, argument_offset, num_arguments as usize);
            events.push(Event {
                namespace,
                code,
                unk1,
                arguments,
            });
        }

        event_offset += EVENT_SIZE as u32;
    }
    Script {
        events,
        offset: offset as i32,
    }
}

#[rustfmt::skip]
fn arguments(data: FancySlice, origin: u32, num_arguments: usize) -> Vec<Argument> {
    let mut arguments = vec!();
    for i in 0..num_arguments as i32 {
        let argument_offset = i * ARGUMENT_SIZE as i32;

        if argument_offset + 8 > data.len() as i32 {
            error!("Script argument parsing tried to read out of bounds via offset {} into data of size {}", argument_offset, data.len());
            break;
        }

        let ty    = data.i32_be(argument_offset as usize    );
        let value = data.i32_be(argument_offset as usize + 4);

        let argument = match ty {
            0 => Argument::Value (value),
            1 => Argument::Scalar (value as f32 / 60000.0),
            2 => Argument::Offset (Offset { offset: value, origin: origin as i32 + argument_offset + 4}),
            3 => Argument::Bool (value == 1),
            4 => Argument::File (value),
            5 => {
                let value = value as u32;
                let memory_type = ((value & 0xF0000000) >> 28) as u8;
                let data_type   = ((value & 0x0F000000) >> 24) as u8;
                let address     =  (value & 0x00FFFFFF)        as u32;

                let memory_type = VariableMemoryType::new(memory_type);
                let data_type = VariableDataType::new(data_type);

                Argument::Variable (Variable { memory_type, data_type, address })
            }
            6 => Requirement::new(value as u32),
            _ => Argument::Unknown (ty, value),
        };
        arguments.push(argument);
    }

    arguments
}

#[derive(Clone, Debug)]
pub struct Script {
    pub events: Vec<Event>,
    pub offset: i32,
}

// Events are like lines of code in a script
const EVENT_SIZE: usize = 0x8;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub namespace: u8,
    pub code: u8,
    pub unk1: u8,
    pub arguments: Vec<Argument>,
}

impl Event {
    pub fn raw_id(&self) -> u32 {
        let num_args = self.arguments.len();
        assert!(num_args < 0x100);
        (self.namespace as u32) << 24 | (self.code as u32) << 16 | (num_args as u32) << 8
    }
}

const ARGUMENT_SIZE: usize = 0x8;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Argument {
    Value(i32),
    Scalar(f32),
    Offset(Offset),
    Bool(bool),
    File(i32),
    Variable(Variable),
    Requirement { flip: bool, ty: Requirement },
    Unknown(i32, i32),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Variable {
    pub memory_type: VariableMemoryType,
    pub data_type: VariableDataType,
    pub address: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Offset {
    pub offset: i32,
    pub origin: i32,
}

#[derive(Debug)]
pub enum OffsetType {
    Internal(i32),
    External(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VariableMemoryType {
    /// Known as IC in existing tools
    InternalConstant,
    /// Known as LA in existing tools
    LongtermAccess,
    /// Known as RA in existing tools
    RandomAccess,
    Unknown(u8),
}

impl VariableMemoryType {
    fn new(value: u8) -> VariableMemoryType {
        match value {
            0 => VariableMemoryType::InternalConstant,
            1 => VariableMemoryType::LongtermAccess,
            2 => VariableMemoryType::RandomAccess,
            _ => VariableMemoryType::Unknown(value),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum VariableDataType {
    /// Known as Basic in existing tools
    Int,
    /// Known as Float in existing tools
    Float,
    /// Known as Bit in existing tools
    Bool,
    Unknown(u8),
}

impl VariableDataType {
    fn new(value: u8) -> VariableDataType {
        match value {
            0 => VariableDataType::Int,
            1 => VariableDataType::Float,
            2 => VariableDataType::Bool,
            _ => VariableDataType::Unknown(value),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Requirement {
    CharacterExists,
    AnimationEnd,
    AnimationHasLooped,
    OnGround,
    InAir,
    HoldingALedge,
    OnAPassableFloor,
    Comparison,
    BoolIsTrue,
    FacingRight,
    FacingLeft,
    HitboxConnects,
    TouchingAFloorWallOrCeiling,
    IsThrowingSomeone,
    ButtonTap,
    EnteringOrIsInHitLag,
    ArticleExists,
    IsOversteppingAnEdge,
    HasAFloorBelowThePlayer,
    ChangeInAirGroundState,
    ArticleAvailable,
    CurrentTriggeredStatusID,
    HoldingItem,
    HoldingItemOfType,
    LightItemIsInGrabRange,
    HeavyItemIsInGrabRange,
    ItemOfTypeIsInGrabbingRange,
    TurningWithItem,
    InWater,
    RollADie,
    SubactionExists,
    ButtonMashingOrStatusExpiredSleepBuryFreeze,
    IsNotInDamagingLens,
    ButtonPress,
    ButtonRelease,
    ButtonHeld,
    ButtonNotPressed,
    StickDirectionPressed,
    StickDirectionNotPressed,
    IsBeingThrownBySomeone1,
    IsBeingThrownBySomeone2,
    HasntTethered3Times,
    HasPassedOverAnEdgeForward,
    HasPassedOverAnEdgeBackward,
    IsHoldingSomeoneInGrab,
    HitboxHasConnected,
    PickUpItem,
    /// PM Only
    SDIInput,
    /// PM Only
    ShieldInputPress,
    /// PM Only
    ShieldInputHeld,
    /// PM Only
    TauntInputPress,
    /// PM Only
    TauntInputHeld,
    HitByCapeEffect,
    /// Independent Subroutine WiiRD code Only
    ThreadIsNull,
    Always,
    InWalljump,
    InWallCling,
    InFootstoolRange,
    IsFallingOrHitDown,
    HasSmashBall,
    CanPickupAnotherItem,
    FSmashShortcut,
    TapJumpOn,
    Unknown(u32),
}

impl Requirement {
    fn new(value: u32) -> Argument {
        let flip = value >> 31 == 1;
        let ty = match value & 0xFFFF {
            0x0000 => Requirement::CharacterExists,
            0x0001 => Requirement::AnimationEnd,
            0x0002 => Requirement::AnimationHasLooped,
            0x0003 => Requirement::OnGround,
            0x0004 => Requirement::InAir,
            0x0005 => Requirement::HoldingALedge,
            0x0006 => Requirement::OnAPassableFloor,
            0x0007 => Requirement::Comparison,
            0x0008 => Requirement::BoolIsTrue,
            0x0009 => Requirement::FacingRight,
            0x000A => Requirement::FacingLeft,
            0x000B => Requirement::HitboxConnects,
            0x000C => Requirement::TouchingAFloorWallOrCeiling,
            0x000D => Requirement::IsThrowingSomeone,
            0x000F => Requirement::ButtonTap,
            0x0014 => Requirement::EnteringOrIsInHitLag,
            0x0015 => Requirement::ArticleExists,
            0x0016 => Requirement::IsOversteppingAnEdge,
            0x0017 => Requirement::HasAFloorBelowThePlayer,
            0x001B => Requirement::ChangeInAirGroundState,
            0x001C => Requirement::ArticleAvailable,
            0x001D => Requirement::CurrentTriggeredStatusID,
            0x001F => Requirement::HoldingItem,
            0x0020 => Requirement::HoldingItemOfType,
            0x0021 => Requirement::LightItemIsInGrabRange,
            0x0022 => Requirement::HeavyItemIsInGrabRange,
            0x0023 => Requirement::ItemOfTypeIsInGrabbingRange,
            0x0024 => Requirement::TurningWithItem,
            0x002A => Requirement::InWater,
            0x002B => Requirement::RollADie,
            0x002C => Requirement::SubactionExists,
            0x002E => Requirement::ButtonMashingOrStatusExpiredSleepBuryFreeze,
            0x002F => Requirement::IsNotInDamagingLens,
            0x0030 => Requirement::ButtonPress,
            0x0031 => Requirement::ButtonRelease,
            0x0032 => Requirement::ButtonHeld,
            0x0033 => Requirement::ButtonNotPressed,
            0x0034 => Requirement::StickDirectionPressed,
            0x0035 => Requirement::StickDirectionNotPressed,
            0x0037 => Requirement::IsBeingThrownBySomeone1,
            0x0038 => Requirement::IsBeingThrownBySomeone2,
            0x0039 => Requirement::HasntTethered3Times,
            0x003a => Requirement::HasPassedOverAnEdgeForward,
            0x003b => Requirement::HasPassedOverAnEdgeBackward,
            0x003c => Requirement::IsHoldingSomeoneInGrab,
            0x003d => Requirement::HitboxHasConnected,
            0x0047 => Requirement::PickUpItem,
            0x004C => Requirement::HitByCapeEffect,
            0x004D => Requirement::SDIInput,
            0x004E => Requirement::ShieldInputPress,
            0x004f => Requirement::ShieldInputHeld,
            0x0050 => Requirement::TauntInputPress,
            0x0051 => Requirement::TauntInputHeld,
            0x0060 => Requirement::ThreadIsNull,
            0x00FF => Requirement::Always,
            0x2711 => Requirement::InWalljump,
            0x2712 => Requirement::InWallCling,
            0x2713 => Requirement::InFootstoolRange,
            0x2716 => Requirement::IsFallingOrHitDown,
            0x2717 => Requirement::HasSmashBall,
            0x2719 => Requirement::CanPickupAnotherItem,
            0x271D => Requirement::FSmashShortcut,
            0x2725 => Requirement::TapJumpOn,
            v => Requirement::Unknown(v),
        };
        Argument::Requirement { ty, flip }
    }
}
