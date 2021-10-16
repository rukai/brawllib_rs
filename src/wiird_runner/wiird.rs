#[derive(Serialize, Clone, Debug)]
pub enum JumpFlag {
    WhenTrue,
    WhenFalse,
    Always,
}

#[derive(Serialize, Clone, Debug)]
pub enum AddAddress {
    BaseAddress,
    PointerAddress,
    None,
}

#[derive(Serialize, Clone, Debug)]
pub enum GeckoOperation {
    Add,
    Mul,
    Or,
    And,
    Xor,
    ShiftLeft,
    ShiftRight,
    RotateLeft,
    ArithmeticShiftRight,
    FloatAdd,
    FloatMul,
    Unknown(u8),
}

impl GeckoOperation {
    pub(crate) fn new(value: u8) -> Self {
        match value {
            00 => GeckoOperation::Add,
            01 => GeckoOperation::Mul,
            02 => GeckoOperation::Or,
            03 => GeckoOperation::And,
            04 => GeckoOperation::Xor,
            05 => GeckoOperation::ShiftLeft,
            06 => GeckoOperation::ShiftRight,
            07 => GeckoOperation::RotateLeft,
            08 => GeckoOperation::ArithmeticShiftRight,
            10 => GeckoOperation::FloatAdd,
            11 => GeckoOperation::FloatMul,
            _ => GeckoOperation::Unknown(value),
        }
    }
}
