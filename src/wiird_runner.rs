use byteorder::{BigEndian, ByteOrder};

use crate::wiird::{WiiRDBlock, WiiRDCode, AddAddress, JumpFlag};
use crate::wii_memory::WiiMemory;

pub fn process(codeset: &WiiRDBlock, buffer: &mut [u8], buffer_ram_location: u32) -> WiiMemory {
    let mut memory = WiiMemory::new();
    let mut gecko_registers = [0_u32; 0x10];
    let mut base_address    = 0x80000000;
    let mut pointer_address = 0x80000000;

    // write buffer to memory
    for (i, value) in buffer.iter().enumerate() {
        memory.write_u8(buffer_ram_location as usize + i, *value);
    }

    // TODO: The if statement ast thing will never work properly... How to get to the right line
    // Well hang on when goto does: "The code handler jumps to (next line of code + XXXX lines). XXXX is signed.
    // What does a line even mean. Does it mean exactly 8 bytes every time or does it refer to an individual code?
    // If its 16 bytes, then that even breaks simple enum processing!
    // Do I need to include a line number for each code!!??!?
    //

    let mut line = 0;
    while line < codeset.codes.len() {
        let code = codeset.codes[line].clone();

        match code {
            WiiRDCode::WriteAndFill8 { use_base_address, address, value, length } => {
                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                for i in 0..length {
                    let current_address = mem_address + i;

                    // write to wii ram
                    memory.write_u8(current_address as usize, value);

                    // also write to the provided buffer if it would have been written to on a wii.
                    if current_address >= buffer_ram_location && current_address < buffer_ram_location + buffer.len() as u32 {
                        let buffer_offset = current_address - buffer_ram_location;
                        buffer[buffer_offset as usize] = value;
                    }
                }
            }
            WiiRDCode::WriteAndFill16 { use_base_address, address, value, length } => {
                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                for i in 0..length {
                    let current_address = mem_address + i * 2;

                    // write to wii ram
                    memory.write_u16(current_address as usize, value);

                    // also write to the provided buffer if it would have been written to on a wii.
                    if current_address >= buffer_ram_location && current_address < buffer_ram_location + buffer.len() as u32 {
                        let buffer_offset = current_address - buffer_ram_location;
                        BigEndian::write_u16(&mut buffer[buffer_offset as usize..], value);
                    }
                }
            }
            WiiRDCode::WriteAndFill32 { use_base_address, address, value } => {
                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                if mem_address >= buffer_ram_location && mem_address < buffer_ram_location + buffer.len() as u32 {
                    // write to wii ram
                    memory.write_u32(mem_address as usize, value);

                    // also write to the provided buffer if it would have been written to on a wii.
                    let buffer_offset = mem_address - buffer_ram_location;
                    BigEndian::write_u32(&mut buffer[buffer_offset as usize..], value);
                }
            }
            WiiRDCode::StringWrite { use_base_address, address, values } => {
                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                for (i, value) in values.iter().enumerate() {
                    let current_address = mem_address + i as u32;

                    // write to wii ram
                    memory.write_u8(current_address as usize, *value);

                    // also write to the provided buffer if it would have been written to on a wii.
                    if current_address >= buffer_ram_location && current_address < buffer_ram_location + buffer.len() as u32 {
                        let buffer_offset = current_address - buffer_ram_location;
                        buffer[buffer_offset as usize] = *value;
                    }
                }
            }
            WiiRDCode::LoadBaseAddress { add_result, add_mem_address, add_mem_address_gecko_register, mem_address } => {
                let mut actual_address = mem_address;
                match add_mem_address {
                    AddAddress::BaseAddress    => actual_address += base_address,
                    AddAddress::PointerAddress => actual_address += pointer_address,
                    AddAddress::None => { }
                }

                if let Some(gecko_register) = add_mem_address_gecko_register {
                    actual_address += gecko_registers[gecko_register as usize];
                }

                if add_result {
                    base_address += memory.read_u32(actual_address as usize);
                }
                else {
                    base_address = memory.read_u32(actual_address as usize);
                }
            }
            WiiRDCode::SetBaseAddress { add_result, add, add_gecko_register, value } => {
                let mut value = value;
                match add {
                    AddAddress::BaseAddress    => value += base_address,
                    AddAddress::PointerAddress => value += pointer_address,
                    AddAddress::None => { }
                }

                if let Some(gecko_register) = add_gecko_register {
                    value += gecko_registers[gecko_register as usize];
                }

                if add_result {
                    base_address += value;
                }
                else {
                    base_address = value;
                }
            }
            WiiRDCode::StoreBaseAddress { add_mem_address, add_mem_address_gecko_register, mem_address } => {
                let mut actual_address = mem_address;
                match add_mem_address {
                    AddAddress::BaseAddress    => actual_address += base_address,
                    AddAddress::PointerAddress => actual_address += pointer_address,
                    AddAddress::None => { }
                }

                if let Some(gecko_register) = add_mem_address_gecko_register {
                    actual_address += gecko_registers[gecko_register as usize];
                }

                memory.write_u32(actual_address as usize, base_address);
            }
            WiiRDCode::SetBaseAddressToCodeLocation { .. } => {
                // Mess up the value so writes can be ignored while in this state
                base_address = 0;
            }
            WiiRDCode::LoadPointerAddress { add_result, add_mem_address, add_mem_address_gecko_register, mem_address } => {
                let mut actual_address = mem_address;
                match add_mem_address {
                    AddAddress::BaseAddress    => actual_address += base_address,
                    AddAddress::PointerAddress => actual_address += pointer_address,
                    AddAddress::None => { }
                }

                if let Some(gecko_register) = add_mem_address_gecko_register {
                    actual_address += gecko_registers[gecko_register as usize];
                }

                if add_result {
                    pointer_address += memory.read_u32(actual_address as usize);
                }
                else {
                    pointer_address = memory.read_u32(actual_address as usize);
                }
            }
            WiiRDCode::SetPointerAddress { add_result, add, add_gecko_register, mut value } => {
                match add {
                    AddAddress::BaseAddress    => value += base_address,
                    AddAddress::PointerAddress => value += pointer_address,
                    AddAddress::None => { }
                }

                if let Some(gecko_register) = add_gecko_register {
                    value += gecko_registers[gecko_register as usize];
                }

                if add_result {
                    pointer_address += value;
                }
                else {
                    pointer_address = value;
                }
            }
            WiiRDCode::StorePointerAddress { add_mem_address, add_mem_address_gecko_register, mem_address } => {
                let mut actual_address = mem_address;
                match add_mem_address {
                    AddAddress::BaseAddress    => actual_address += base_address,
                    AddAddress::PointerAddress => actual_address += pointer_address,
                    AddAddress::None => { }
                }

                if let Some(gecko_register) = add_mem_address_gecko_register {
                    actual_address += gecko_registers[gecko_register as usize];
                }

                memory.write_u32(actual_address as usize, pointer_address);
            }
            WiiRDCode::SetPointerAddressToCodeLocation { .. } => {
                // Mess up the value so writes can be ignored while in this state
                pointer_address = 0;
            }
            WiiRDCode::Goto { flag, offset_lines } => {
                match flag {
                    JumpFlag::Always => {
                        line += offset_lines as usize;
                    }
                    _ => { }
                }
            }
            WiiRDCode::ResetAddressHigh { reset_base_address_high, reset_pointer_address_high } => {
                if reset_base_address_high != 0 {
                    base_address = (reset_base_address_high as u32) << 16
                }
                if reset_pointer_address_high != 0 {
                    pointer_address = (reset_pointer_address_high as u32) << 16
                }
            }
            WiiRDCode::SetGeckoRegister { add_result, add, register, mut value } => {
                match add {
                    AddAddress::BaseAddress    => value += base_address,
                    AddAddress::PointerAddress => value += pointer_address,
                    AddAddress::None => { }
                }

                if add_result {
                    gecko_registers[register as usize] += value;
                }
                else {
                    gecko_registers[register as usize] = value;
                }
            }
            _ => { }
        }
        line += 1;
    }

    memory
}
