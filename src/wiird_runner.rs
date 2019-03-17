use std::collections::HashMap;

use byteorder::{BigEndian, ByteOrder};

use crate::wiird::{WiiRDCode, AddAddress};

pub fn process(codeset: &[WiiRDCode], buffer: &mut [u8], buffer_ram_location: u32) -> Vec<u8> {
    // TODO: HashMap is completely wrong, needs to be an array or else overlapping reads/writes dont work.
    let mut memory = HashMap::new();
    let mut gecko_registers = [0_u32; 0x10];
    let mut base_address    = 0x80000000;
    let mut pointer_address = 0x80000000;

    for code in codeset {
        match code.clone() {
            WiiRDCode::WriteAndFill32 { base_address: use_base_address, address, value } => {
                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                if mem_address > buffer_ram_location && mem_address < buffer_ram_location + buffer.len() as u32 {
                    let buffer_offset = mem_address - buffer_ram_location;

                    BigEndian::write_u32(&mut buffer[buffer_offset as usize..], value);
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
                    base_address += memory.get(&actual_address).cloned().unwrap_or_default();
                }
                else {
                    base_address = memory.get(&actual_address).cloned().unwrap_or_default();
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

                memory.insert(actual_address, base_address);
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
                    pointer_address += memory.get(&actual_address).cloned().unwrap_or_default();
                }
                else {
                    pointer_address = memory.get(&actual_address).cloned().unwrap_or_default();
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

                memory.insert(actual_address, pointer_address);
            }
            WiiRDCode::SetPointerAddressToCodeLocation { .. } => {
                // Mess up the value so writes can be ignored while in this state
                pointer_address = 0;
            }
            WiiRDCode::FullTerminator { base_address_high, pointer_address_high} => {
                // TODO: clear code execution status

                if base_address_high != 0 {
                    base_address = (base_address_high as u32) << 16
                }
                if pointer_address_high != 0 {
                    pointer_address = (pointer_address_high as u32) << 16
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
    }
    vec!()
}
