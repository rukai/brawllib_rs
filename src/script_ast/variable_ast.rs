use crate::script::{Variable, VariableMemoryType, VariableDataType};

#[derive(Serialize, Clone, Debug)]
pub enum VariableAst {
    /// Known as IC in existing tools
    InternalConstantInt (InternalConstantInt),

    /// Known as LA in existing tools
    LongtermAccessInt (LongtermAccessInt),
    /// Known as LA in existing tools
    LongtermAccessFloat (LongtermAccessFloat),
    /// Known as LA in existing tools
    LongtermAccessBool (LongtermAccessBool),

    /// Known as RA in existing tools
    RandomAccessInt (RandomAccessInt),
    /// Known as RA in existing tools
    RandomAccessFloat (RandomAccessFloat),
    /// Known as RA in existing tools
    RandomAccessBool (RandomAccessBool),

    Unknown { memory_type: VariableMemoryType, data_type: VariableDataType, address: u32 }
}

impl VariableAst {
    pub fn new(var: &Variable) -> VariableAst {
        match (&var.memory_type, &var.data_type) {
            (VariableMemoryType::InternalConstant, VariableDataType::Int)   => VariableAst::InternalConstantInt (InternalConstantInt::new(var.address)),
            (VariableMemoryType::LongtermAccess,   VariableDataType::Int)   => VariableAst::LongtermAccessInt   (LongtermAccessInt::  new(var.address)),
            (VariableMemoryType::LongtermAccess,   VariableDataType::Float) => VariableAst::LongtermAccessFloat (LongtermAccessFloat::new(var.address)),
            (VariableMemoryType::LongtermAccess,   VariableDataType::Bool)  => VariableAst::LongtermAccessBool  (LongtermAccessBool:: new(var.address)),
            (VariableMemoryType::RandomAccess,     VariableDataType::Int)   => VariableAst::RandomAccessInt     (RandomAccessInt::    new(var.address)),
            (VariableMemoryType::RandomAccess,     VariableDataType::Float) => VariableAst::RandomAccessFloat   (RandomAccessFloat::  new(var.address)),
            (VariableMemoryType::RandomAccess,     VariableDataType::Bool)  => VariableAst::RandomAccessBool    (RandomAccessBool::   new(var.address)),
            _ => VariableAst::Unknown {
                memory_type: var.memory_type.clone(),
                data_type:   var.data_type.clone(),
                address:     var.address,
            }
        }
    }

    pub fn data_type(&self) -> VariableDataType {
        match self {
            VariableAst::InternalConstantInt (_) => VariableDataType::Int,

            VariableAst::LongtermAccessInt   (_) => VariableDataType::Int,
            VariableAst::LongtermAccessFloat (_) => VariableDataType::Float,
            VariableAst::LongtermAccessBool  (_) => VariableDataType::Bool,

            VariableAst::RandomAccessInt   (_) => VariableDataType::Int,
            VariableAst::RandomAccessFloat (_) => VariableDataType::Float,
            VariableAst::RandomAccessBool  (_) => VariableDataType::Bool,

            VariableAst::Unknown { ref data_type, .. } => data_type.clone(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum InternalConstantInt {
    CurrentFrame,
    Damage,
    CharacterXPosition,
    CharacterYPosition,
    CharacterDirection,
    CharacterDirectionOpposite,
    VerticalCharacterVelocity,
    CurrentFrameSpeed,
    HorizontalCharacterVelocity,
    Knockback,
    SurfaceTraction,
    XVelocity,
    LaunchSpeed,

    RightKnockback,
    LeftKnockback,
    UpKnockback,
    DownKnockback,

    ControlStickXAxis,
    ControlStickXAxisRelative,
    ControlStickXAxisRelativeReverse,
    ControlStickXAxisAbsolute,
    ControlStickXAxisReverse,
    ControlStickXAxisReverse2,
    ControlStickYAxis,
    ControlStickYAxisAbsolute,
    ControlStickYAxisReverse,
    ControlStickYAxis2,

    PreviousControlStickXAxis,
    PreviousControlStickXAxisRelative,
    PreviousControlStickXAxisRelativeReverse,
    PreviousControlStickXAxisAbsolute,
    PreviousControlStickXAxisReverse,
    PreviousControlStickXAxisReverse2,
    PreviousControlStickYAxis,
    PreviousControlStickYAxisAbsolute,
    PreviousControlStickYAxisReverse,
    PreviousControlStickYAxis2,
    CrawlControlStickXOffsetMax,
    CrawlControlStickXOffsetMin,

    CurrentSubaction,
    CurrentAction,
    PreviousAction,
    HeldItem,
    EffectOfAttack,

    FramesSinceNormal,
    FramesSinceSpecial,
    FramesSinceJump,
    FramesSinceShield,
    FramesSinceShield2,

    TurnRunFrameTimer,
    JumpStartTimer,
    MaxJumpCount,
    GlideStartTimer,
    TermVelFrameTimer,

    Address (u32)
}

impl InternalConstantInt {
    fn new(address: u32) -> InternalConstantInt {
        match address {
           00000 => InternalConstantInt::CurrentFrame,
           00002 => InternalConstantInt::Damage,
           00003 => InternalConstantInt::CharacterXPosition,
           00004 => InternalConstantInt::CharacterYPosition,
           00008 => InternalConstantInt::CharacterDirection,
           00009 => InternalConstantInt::CharacterDirectionOpposite,
           00023 => InternalConstantInt::VerticalCharacterVelocity,
           00024 => InternalConstantInt::CurrentFrameSpeed,
           00028 => InternalConstantInt::HorizontalCharacterVelocity,
           00038 => InternalConstantInt::Knockback,
           00039 => InternalConstantInt::SurfaceTraction,
           01000 => InternalConstantInt::XVelocity,
           01005 => InternalConstantInt::LaunchSpeed,
           01006 => InternalConstantInt::RightKnockback,
           01007 => InternalConstantInt::LeftKnockback,
           01008 => InternalConstantInt::UpKnockback,
           01009 => InternalConstantInt::DownKnockback,
           01010 => InternalConstantInt::ControlStickXAxis,
           01011 => InternalConstantInt::ControlStickXAxisRelative,
           01012 => InternalConstantInt::ControlStickXAxisRelativeReverse,
           01013 => InternalConstantInt::ControlStickXAxisAbsolute,
           01014 => InternalConstantInt::ControlStickXAxisReverse,
           01017 => InternalConstantInt::ControlStickXAxisReverse2,
           01018 => InternalConstantInt::ControlStickYAxis,
           01019 => InternalConstantInt::ControlStickYAxisAbsolute,
           01020 => InternalConstantInt::ControlStickYAxisReverse,
           01021 => InternalConstantInt::ControlStickYAxis2,
           01022 => InternalConstantInt::PreviousControlStickXAxis,
           01023 => InternalConstantInt::PreviousControlStickXAxisRelative,
           01024 => InternalConstantInt::PreviousControlStickXAxisRelativeReverse,
           01025 => InternalConstantInt::PreviousControlStickXAxisAbsolute,
           01026 => InternalConstantInt::PreviousControlStickYAxis,
           01027 => InternalConstantInt::PreviousControlStickYAxisAbsolute,
           01028 => InternalConstantInt::PreviousControlStickYAxisReverse,
           03134 => InternalConstantInt::CrawlControlStickXOffsetMax,
           03136 => InternalConstantInt::CrawlControlStickXOffsetMin,
           20000 => InternalConstantInt::CurrentSubaction,
           20001 => InternalConstantInt::CurrentAction,
           20003 => InternalConstantInt::PreviousAction,
           20009 => InternalConstantInt::HeldItem,
           21004 => InternalConstantInt::EffectOfAttack,
           21010 => InternalConstantInt::FramesSinceNormal,
           21012 => InternalConstantInt::FramesSinceSpecial,
           21014 => InternalConstantInt::FramesSinceJump,
           21016 => InternalConstantInt::FramesSinceShield,
           21018 => InternalConstantInt::FramesSinceShield2,
           23001 => InternalConstantInt::TurnRunFrameTimer,
           23002 => InternalConstantInt::JumpStartTimer,
           23003 => InternalConstantInt::MaxJumpCount,
           23004 => InternalConstantInt::GlideStartTimer,
           23007 => InternalConstantInt::TermVelFrameTimer,
            _    => InternalConstantInt::Address (address)
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum LongtermAccessInt {
    JumpsUsed,
    WallJumpCount,
    WallJumpInterval,
    FootstoolCount,
    FallTime,
    SwimTime,
    LipStickRefresh,
    CurryRemainingTime,
    CurryAngle2,
    StarRemainingTime,
    MushroomRemainingTime,
    LightningRemainingTime,
    SizeFlag,
    MetalBlockRemainingTime,
    ComboCount,
    BubbleTime,
    AttacksPerformed,
    CostumeID,
    HitstunFramesRemaining,
    MeteorCancelWindow,
    MissedTechs,
    TetherCount,
    Temp1,
    Temp2,
    Address (u32),
}

impl LongtermAccessInt {
    fn new(address: u32) -> LongtermAccessInt {
        match address {
           01 => LongtermAccessInt::JumpsUsed,
           03 => LongtermAccessInt::WallJumpCount,
           04 => LongtermAccessInt::WallJumpInterval,
           05 => LongtermAccessInt::FootstoolCount,
           13 => LongtermAccessInt::FallTime,
           20 => LongtermAccessInt::SwimTime,
           24 => LongtermAccessInt::LipStickRefresh,
           25 => LongtermAccessInt::CurryRemainingTime,
           26 => LongtermAccessInt::CurryAngle2,
           30 => LongtermAccessInt::StarRemainingTime,
           33 => LongtermAccessInt::MushroomRemainingTime,
           34 => LongtermAccessInt::LightningRemainingTime,
           35 => LongtermAccessInt::SizeFlag,
           37 => LongtermAccessInt::MetalBlockRemainingTime,
           44 => LongtermAccessInt::ComboCount,
           46 => LongtermAccessInt::BubbleTime,
           53 => LongtermAccessInt::AttacksPerformed,
           54 => LongtermAccessInt::CostumeID,
           56 => LongtermAccessInt::HitstunFramesRemaining,
           57 => LongtermAccessInt::MeteorCancelWindow,
           61 => LongtermAccessInt::MissedTechs,
           62 => LongtermAccessInt::TetherCount,
           64 => LongtermAccessInt::Temp1,
           65 => LongtermAccessInt::Temp2,
           _  => LongtermAccessInt::Address (address),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum LongtermAccessFloat {
    SpecialLandingLag,
    SpecialFallMobilityMultiplier,
    ShieldCharge,
    CurryAngle1,
    CurryRandomness,
    Address (u32),
}

impl LongtermAccessFloat {
    fn new(address: u32) -> LongtermAccessFloat {
        match address {
            0 => LongtermAccessFloat::SpecialLandingLag,
            1 => LongtermAccessFloat::SpecialFallMobilityMultiplier,
            3 => LongtermAccessFloat::ShieldCharge,
            7 => LongtermAccessFloat::CurryAngle1,
            8 => LongtermAccessFloat::CurryRandomness,
            _ => LongtermAccessFloat::Address (address),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum LongtermAccessBool {
    IsDead,
    CannotDie,
    AutomaticFootstool,
    HasFinal,
    HasFinalAura,
    HasCurry,
    HasHammer,
    HitByParalyze,
    HasScrewAttack,
    StaminaDead,
    HasTag,
    CanNotLedgeGrab,
    CanNotTeeter,
    VelocityIgnoreHitstun,
    Deflection,
    Address (u32),
}

impl LongtermAccessBool {
    fn new(address: u32) -> LongtermAccessBool {
        match address {
            00 => LongtermAccessBool::IsDead,
            01 => LongtermAccessBool::CannotDie,
            05 => LongtermAccessBool::AutomaticFootstool,
            08 => LongtermAccessBool::HasFinal,
            09 => LongtermAccessBool::HasFinalAura,
            10 => LongtermAccessBool::HasCurry,
            11 => LongtermAccessBool::HasHammer,
            17 => LongtermAccessBool::HitByParalyze,
            19 => LongtermAccessBool::HasScrewAttack,
            24 => LongtermAccessBool::StaminaDead,
            27 => LongtermAccessBool::HasTag,
            36 => LongtermAccessBool::CanNotLedgeGrab,
            57 => LongtermAccessBool::CanNotTeeter,
            61 => LongtermAccessBool::VelocityIgnoreHitstun,
            65 => LongtermAccessBool::Deflection,
            _  => LongtermAccessBool::Address (address),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum RandomAccessInt {
    ThrowDataParam1,
    ThrowDataParam2,
    ThrowDataParam3,
    Address (u32),
}

impl RandomAccessInt {
    fn new(address: u32) -> Self {
        match address {
            2 => RandomAccessInt::ThrowDataParam1,
            3 => RandomAccessInt::ThrowDataParam2,
            4 => RandomAccessInt::ThrowDataParam3,
            _ => RandomAccessInt::Address (address),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum RandomAccessFloat {
    EnableTurnWhenBelowZero,
    Address (u32),
}

impl RandomAccessFloat {
    fn new(address: u32) -> Self {
        match address {
            4 => RandomAccessFloat::EnableTurnWhenBelowZero,
            _ => RandomAccessFloat::Address (address),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum RandomAccessBool {
    CharacterFloat,
    EnableFastFall,
    Shorthop,
    EnableActionTransition,
    SpecialsMovement,
    EnableGlide,
    EnableJabLoop,
    EnableAutoJab,
    EnableJabEnd,
    EnableLandingLag,
    Address (u32),
}

impl RandomAccessBool {
    fn new(address: u32) -> Self {
        match address {
            00 => RandomAccessBool::CharacterFloat,
            02 => RandomAccessBool::EnableFastFall,
            06 => RandomAccessBool::Shorthop,
            16 => RandomAccessBool::EnableActionTransition,
            18 => RandomAccessBool::SpecialsMovement,
            19 => RandomAccessBool::EnableGlide,
            20 => RandomAccessBool::EnableJabLoop,
            22 => RandomAccessBool::EnableAutoJab,
            25 => RandomAccessBool::EnableJabEnd,
            30 => RandomAccessBool::EnableLandingLag,
            _  => RandomAccessBool::Address (address),
        }
    }
}
