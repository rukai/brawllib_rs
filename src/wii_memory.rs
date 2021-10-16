use byteorder::{BigEndian, ByteOrder};
use fancy_slice::FancySlice;

pub struct WiiMemory {
    mem1: Vec<u8>,
    mem2: Vec<u8>,
}

impl WiiMemory {
    pub fn new() -> Self {
        WiiMemory {
            mem1: vec![0; 0x180_0000],
            mem2: vec![0; 0x400_0000],
        }
    }

    pub fn write_u8(&mut self, address: usize, value: u8) {
        if address >= 0x8000_0000 && address < 0x8180_0000 {
            self.mem1[address - 0x8000_0000] = value;
        } else if address >= 0x9000_0000 && address < 0x9400_0000 {
            self.mem2[address - 0x9000_0000] = value;
        } else {
            error!(
                "Failed to write value: 0x{:x} Cannot map address 0x{:x} to wii memory",
                value, address
            );
        }
    }

    pub fn write_u16(&mut self, address: usize, value: u16) {
        if address >= 0x8000_0000 && address < 0x8180_0000 {
            BigEndian::write_u16(&mut self.mem1[address - 0x8000_0000..], value);
        } else if address >= 0x9000_0000 && address < 0x9400_0000 {
            BigEndian::write_u16(&mut self.mem2[address - 0x9000_0000..], value);
        } else {
            error!(
                "Failed to write value: 0x{:x} Cannot map address 0x{:x} to wii memory",
                value, address
            );
        }
    }

    pub fn write_u32(&mut self, address: usize, value: u32) {
        if address >= 0x8000_0000 && address < 0x8180_0000 {
            BigEndian::write_u32(&mut self.mem1[address - 0x8000_0000..], value);
        } else if address >= 0x9000_0000 && address < 0x9400_0000 {
            BigEndian::write_u32(&mut self.mem2[address - 0x9000_0000..], value);
        } else {
            error!(
                "Failed to write value: 0x{:x} Cannot map address 0x{:x} to wii memory",
                value, address
            );
        }
    }

    pub fn read_u8(&self, address: usize) -> u8 {
        if address >= 0x8000_0000 && address < 0x8180_0000 {
            return self.mem1[address - 0x8000_0000];
        } else if address >= 0x9000_0000 && address < 0x9400_0000 {
            return self.mem2[address - 0x9000_0000];
        } else {
            error!(
                "Failed to read value: Cannot map address 0x{:x} to wii memory",
                address
            );
            0
        }
    }

    pub fn read_u16(&self, address: usize) -> u16 {
        if address >= 0x8000_0000 && address < 0x8180_0000 {
            BigEndian::read_u16(&self.mem1[address - 0x8000_0000..])
        } else if address >= 0x9000_0000 && address < 0x9400_0000 {
            BigEndian::read_u16(&self.mem2[address - 0x9000_0000..])
        } else {
            error!(
                "Failed to read value: Cannot map address 0x{:x} to wii memory",
                address
            );
            0
        }
    }

    pub fn read_u32(&self, address: usize) -> u32 {
        if address >= 0x8000_0000 && address < 0x8180_0000 {
            BigEndian::read_u32(&self.mem1[address - 0x8000_0000..])
        } else if address >= 0x9000_0000 && address < 0x9400_0000 {
            BigEndian::read_u32(&self.mem2[address - 0x9000_0000..])
        } else {
            error!(
                "Failed to read value: Cannot map address 0x{:x} to wii memory",
                address
            );
            0
        }
    }

    pub fn read_f32(&self, address: usize) -> f32 {
        if address >= 0x8000_0000 && address < 0x8180_0000 {
            BigEndian::read_f32(&self.mem1[address - 0x8000_0000..])
        } else if address >= 0x9000_0000 && address < 0x9400_0000 {
            BigEndian::read_f32(&self.mem2[address - 0x9000_0000..])
        } else {
            error!(
                "Failed to read value: Cannot map address 0x{:x} to wii memory",
                address
            );
            0.0
        }
    }

    pub fn buffer_from(&self, address: usize) -> &[u8] {
        if address >= 0x8000_0000 && address < 0x8180_0000 {
            &self.mem1[address - 0x8000_0000..]
        } else if address >= 0x9000_0000 && address < 0x9400_0000 {
            &self.mem2[address - 0x9000_0000..]
        } else {
            error!(
                "Failed to get buffer: Cannot map address 0x{:x} to wii memory",
                address
            );
            &[]
        }
    }
    pub fn fancy_slice_from(&self, address: usize) -> FancySlice {
        let slice = if address >= 0x8000_0000 && address < 0x8180_0000 {
            &self.mem1[address - 0x8000_0000..]
        } else if address >= 0x9000_0000 && address < 0x9400_0000 {
            &self.mem2[address - 0x9000_0000..]
        } else {
            error!(
                "Failed to get buffer: Cannot map address 0x{:x} to wii memory",
                address
            );
            &[]
        };
        FancySlice::new(slice)
    }
}
