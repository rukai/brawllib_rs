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
    None
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
    Unknown (u8),
}

impl GeckoOperation {
    pub(crate) fn new(value: u8) -> Self {
        match value {
            0  => GeckoOperation::Add,
            1  => GeckoOperation::Mul,
            2  => GeckoOperation::Or,
            3  => GeckoOperation::And,
            4  => GeckoOperation::Xor,
            5  => GeckoOperation::ShiftLeft,
            6  => GeckoOperation::ShiftRight,
            7  => GeckoOperation::RotateLeft,
            8  => GeckoOperation::ArithmeticShiftRight,
            10 => GeckoOperation::FloatAdd,
            11 => GeckoOperation::FloatMul,
            _  => GeckoOperation::Unknown (value),
        }
    }
}
