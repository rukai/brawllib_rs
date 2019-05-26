#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum WiiPixelFormat {
    I4,
    I8,
    IA4,
    IA8,
    RGB565,
    RGB5A3,
    RGBA8,
    CI4,
    CI8,
    CMPR4,
    Unknown (u32)
}

impl WiiPixelFormat {
    pub(crate) fn _new(value: u32) -> Self {
        match value {
            0x0 => WiiPixelFormat::I4,
            0x1 => WiiPixelFormat::I8,
            0x2 => WiiPixelFormat::IA4,
            0x3 => WiiPixelFormat::IA8,
            0x4 => WiiPixelFormat::RGB565,
            0x5 => WiiPixelFormat::RGB5A3,
            0x6 => WiiPixelFormat::RGBA8,
            0x8 => WiiPixelFormat::CI4,
            0x9 => WiiPixelFormat::CI8,
            0xE => WiiPixelFormat::CMPR4,
            _   => WiiPixelFormat::Unknown (value)
        }
    }

    pub(crate) fn _value(&self) -> u32 {
        match self {
            WiiPixelFormat::I4              => 0x0,
            WiiPixelFormat::I8              => 0x1,
            WiiPixelFormat::IA4             => 0x2,
            WiiPixelFormat::IA8             => 0x3,
            WiiPixelFormat::RGB565          => 0x4,
            WiiPixelFormat::RGB5A3          => 0x5,
            WiiPixelFormat::RGBA8           => 0x6,
            WiiPixelFormat::CI4             => 0x8,
            WiiPixelFormat::CI8             => 0x9,
            WiiPixelFormat::CMPR4           => 0xE,
            WiiPixelFormat::Unknown (value) => *value,
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum WiiPixelFormat2 {
    TfI4,
    TfI8,
    TfIA4,
    TfIA8,
    TfRGB565,
    TfRGB5A3,
    TfRGBA8,
    TfCMPR,

    CtfR4,
    CtfRA4,
    CtfRA8,
    CtfYUVA8,
    CtfA8,
    CtfR8,
    CtfG8,
    CtfB8,
    CtfRG8,
    CtfGB8,

    TfZ8,
    TfZ16,
    TfZ24X8,

    CtfZ4,
    CtfZ8M,
    CtfZ8L,
    CtfZ16L,
    Unknown (u32)
}

impl WiiPixelFormat2 {
    pub(crate) fn _new(value: u32) -> Self {
        match value {
            0x00 => WiiPixelFormat2::TfI4,
            0x01 => WiiPixelFormat2::TfI8,
            0x02 => WiiPixelFormat2::TfIA4,
            0x03 => WiiPixelFormat2::TfIA8,
            0x04 => WiiPixelFormat2::TfRGB565,
            0x05 => WiiPixelFormat2::TfRGB5A3,
            0x06 => WiiPixelFormat2::TfRGBA8,
            0x0E => WiiPixelFormat2::TfCMPR,

            0x20 => WiiPixelFormat2::CtfR4,
            0x22 => WiiPixelFormat2::CtfRA4,
            0x23 => WiiPixelFormat2::CtfRA8,
            0x26 => WiiPixelFormat2::CtfYUVA8,
            0x27 => WiiPixelFormat2::CtfA8,
            0x28 => WiiPixelFormat2::CtfR8,
            0x29 => WiiPixelFormat2::CtfG8,
            0x2A => WiiPixelFormat2::CtfB8,
            0x2B => WiiPixelFormat2::CtfRG8,
            0x2C => WiiPixelFormat2::CtfGB8,

            0x11 => WiiPixelFormat2::TfZ8,
            0x13 => WiiPixelFormat2::TfZ16,
            0x16 => WiiPixelFormat2::TfZ24X8,

            0x30 => WiiPixelFormat2::CtfZ4,
            0x39 => WiiPixelFormat2::CtfZ8M,
            0x3A => WiiPixelFormat2::CtfZ8L,
            0x3C => WiiPixelFormat2::CtfZ16L,

            _ => WiiPixelFormat2::Unknown (value)
        }
    }

    pub(crate) fn _value(&self) -> u32 {
        match self {
            WiiPixelFormat2::TfI4     => 0x00,
            WiiPixelFormat2::TfI8     => 0x01,
            WiiPixelFormat2::TfIA4    => 0x02,
            WiiPixelFormat2::TfIA8    => 0x03,
            WiiPixelFormat2::TfRGB565 => 0x04,
            WiiPixelFormat2::TfRGB5A3 => 0x05,
            WiiPixelFormat2::TfRGBA8  => 0x06,
            WiiPixelFormat2::TfCMPR   => 0x0E,

            WiiPixelFormat2::CtfR4    => 0x20,
            WiiPixelFormat2::CtfRA4   => 0x22,
            WiiPixelFormat2::CtfRA8   => 0x23,
            WiiPixelFormat2::CtfYUVA8 => 0x26,
            WiiPixelFormat2::CtfA8    => 0x27,
            WiiPixelFormat2::CtfR8    => 0x28,
            WiiPixelFormat2::CtfG8    => 0x29,
            WiiPixelFormat2::CtfB8    => 0x2A,
            WiiPixelFormat2::CtfRG8   => 0x2B,
            WiiPixelFormat2::CtfGB8   => 0x2C,

            WiiPixelFormat2::TfZ8    => 0x11,
            WiiPixelFormat2::TfZ16   => 0x13,
            WiiPixelFormat2::TfZ24X8 => 0x16,

            WiiPixelFormat2::CtfZ4   => 0x30,
            WiiPixelFormat2::CtfZ8M  => 0x39,
            WiiPixelFormat2::CtfZ8L  => 0x3A,
            WiiPixelFormat2::CtfZ16L => 0x3C,

            WiiPixelFormat2::Unknown (value) => *value,
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum WiiPaletteFormat {
    IA8,
    RGB565,
    RGB5A3,
    Unknown (u32)
}

impl WiiPaletteFormat {
    pub(crate) fn new(value: u32) -> Self {
        match value {
            0x0 => WiiPaletteFormat::IA8,
            0x1 => WiiPaletteFormat::RGB565,
            0x2 => WiiPaletteFormat::RGB5A3,
            _   => WiiPaletteFormat::Unknown (value)
        }
    }

    pub(crate) fn value(&self) -> u32 {
        match self {
            WiiPaletteFormat::IA8    => 0x0,
            WiiPaletteFormat::RGB565 => 0x1,
            WiiPaletteFormat::RGB5A3 => 0x2,
            WiiPaletteFormat::Unknown (value) => *value,
        }
    }
}
