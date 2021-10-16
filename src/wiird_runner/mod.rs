use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

use crate::wii_memory::WiiMemory;

mod wiird;

use wiird::{AddAddress, GeckoOperation, JumpFlag};

// My initial attempt at this, parsed the codeset into an AST.
// But that was a TERRIBLE idea because its impossible to tell if you are currently parsing data or instructions.
// So dont ever try to do that again :)

pub fn process(codeset: &[u8], buffer: &mut [u8], buffer_ram_location: u32) -> WiiMemory {
    let mut memory = WiiMemory::new();
    let mut gecko_registers = [0_u32; 0x10];
    let mut base_address = 0x80000000;
    let mut pointer_address = 0x80000000;

    let mut execution_stack: Vec<bool> = vec![];

    // write buffer to memory
    for (i, value) in buffer.iter().enumerate() {
        memory.write_u8(buffer_ram_location as usize + i, *value);
    }

    let mut offset = 0;
    while offset < codeset.len() {
        // Not every code type uses this, but its safe to just create these for if we need them.
        let use_base_address = codeset[offset] & 0b00010000 == 0;
        let address = (&codeset[offset..]).read_u32::<BigEndian>().unwrap() & 0x1FFFFFF;

        let code = codeset[offset] & 0b11101110;
        let execute = execution_stack.last().cloned().unwrap_or(true);
        match code {
            0x00 => {
                let value = codeset[offset + 7];
                let length = (&codeset[offset + 4..]).read_u16::<BigEndian>().unwrap() as u32 + 1;

                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                if execute {
                    for i in 0..length {
                        let current_address = mem_address + i;

                        // write to wii ram
                        memory.write_u8(current_address as usize, value);

                        // also write to the provided buffer if it would have been written to on a wii.
                        if current_address >= buffer_ram_location
                            && current_address < buffer_ram_location + buffer.len() as u32
                        {
                            let buffer_offset = current_address - buffer_ram_location;
                            buffer[buffer_offset as usize] = value;
                        }
                    }
                }

                offset += 8;
            }
            0x02 => {
                let value = (&codeset[offset + 6..]).read_u16::<BigEndian>().unwrap();
                let length = (&codeset[offset + 4..]).read_u16::<BigEndian>().unwrap() as u32 + 1;

                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                if execute {
                    for i in 0..length {
                        let current_address = mem_address + i * 2;

                        // write to wii ram
                        memory.write_u16(current_address as usize, value);

                        // also write to the provided buffer if it would have been written to on a wii.
                        if current_address >= buffer_ram_location
                            && current_address < buffer_ram_location + buffer.len() as u32
                        {
                            let buffer_offset = current_address - buffer_ram_location;
                            BigEndian::write_u16(&mut buffer[buffer_offset as usize..], value);
                        }
                    }
                }

                offset += 8;
            }
            0x04 => {
                let value = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                if execute {
                    if mem_address >= buffer_ram_location
                        && mem_address < buffer_ram_location + buffer.len() as u32
                    {
                        // write to wii ram
                        memory.write_u32(mem_address as usize, value);

                        // also write to the provided buffer if it would have been written to on a wii.
                        let buffer_offset = mem_address - buffer_ram_location;
                        BigEndian::write_u32(&mut buffer[buffer_offset as usize..], value);
                    }
                }

                offset += 8;
            }
            0x06 => {
                let mut values = vec![];
                let count = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap() as usize;
                for i in 0..count {
                    values.push(codeset[offset + 8 + i]);
                }

                offset += 8 + count;

                // align the offset to 8 bytes
                let count_mod = count % 8;
                if count_mod != 0 {
                    offset += 8 - count_mod;
                }

                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                if execute {
                    for (i, value) in values.iter().enumerate() {
                        let current_address = mem_address + i as u32;

                        // write to wii ram
                        memory.write_u8(current_address as usize, *value);

                        // also write to the provided buffer if it would have been written to on a wii.
                        if current_address >= buffer_ram_location
                            && current_address < buffer_ram_location + buffer.len() as u32
                        {
                            let buffer_offset = current_address - buffer_ram_location;
                            buffer[buffer_offset as usize] = *value;
                        }
                    }
                }
            }
            0x08 => {
                let _initial_value = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();
                let _value_size = codeset[offset + 8];
                let _count =
                    ((&codeset[offset + 8..]).read_u16::<BigEndian>().unwrap() & 0x0FFF) + 1;
                let _address_increment = (&codeset[offset + 10..]).read_u16::<BigEndian>().unwrap();
                let _value_increment = (&codeset[offset + 12..]).read_u32::<BigEndian>().unwrap();

                offset += 16;
            }
            0x20 | 0x22 | 0x24 | 0x26 | 0x28 | 0x2A | 0x2C | 0x2E => {
                let value = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();
                let _lhs_mask = (&codeset[offset + 4..]).read_u16::<BigEndian>().unwrap();
                let _rhs_value = (&codeset[offset + 6..]).read_u16::<BigEndian>().unwrap();

                let insert_endif = address & 1 != 0;
                let address = address & 0xFFFFFFFE;

                if insert_endif {
                    execution_stack.pop();
                }

                let mem_address = if use_base_address {
                    (base_address & 0xFE000000) + address
                } else {
                    pointer_address + address
                };

                if execute {
                    match code {
                        0x20 => {
                            // Is equal
                            execution_stack.push(value == memory.read_u32(mem_address as usize));
                        }
                        0x22 => {
                            // Is not equal
                            execution_stack.push(value != memory.read_u32(mem_address as usize));
                        }
                        0x24 => {
                            // Is greater than
                            execution_stack.push(memory.read_u32(mem_address as usize) > value);
                        }
                        0x26 => {
                            // Is less than
                            execution_stack.push(memory.read_u32(mem_address as usize) < value);
                        }
                        0x28 => {
                            // Is equal mask
                            execution_stack.push(false); // TODO
                        }
                        0x2A => {
                            // Is not equal mask
                            execution_stack.push(false); // TODO
                        }
                        0x2C => {
                            // Is greater than mask
                            execution_stack.push(false); // TODO
                        }
                        0x2E => {
                            // Is less than mask
                            execution_stack.push(false); // TODO
                        }
                        _ => unreachable!(),
                    }
                } else {
                    // TODO: Probably need this!?!?!
                    execution_stack.push(execution_stack.last().cloned().unwrap_or(true));
                }

                offset += 8;
            }
            0x40 => {
                let add_result = codeset[offset + 1] & 0b00010000 != 0;
                let add_mem_address_bool = codeset[offset + 1] & 1 != 0;
                let register_bool = codeset[offset + 2] & 0b00010000 != 0;
                let register = codeset[offset + 3] & 0xF;
                let mem_address = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                let add_mem_address = match (add_mem_address_bool, use_base_address) {
                    (true, true) => AddAddress::BaseAddress,
                    (true, false) => AddAddress::PointerAddress,
                    (false, _) => AddAddress::None,
                };

                let add_mem_address_gecko_register =
                    if register_bool { Some(register) } else { None };

                if execute {
                    let mut actual_address = mem_address;
                    match add_mem_address {
                        AddAddress::BaseAddress => actual_address += base_address,
                        AddAddress::PointerAddress => actual_address += pointer_address,
                        AddAddress::None => {}
                    }

                    if let Some(gecko_register) = add_mem_address_gecko_register {
                        actual_address += gecko_registers[gecko_register as usize];
                    }

                    if add_result {
                        base_address += memory.read_u32(actual_address as usize);
                    } else {
                        base_address = memory.read_u32(actual_address as usize);
                    }
                }

                offset += 8;
            }
            0x42 => {
                let add_result = codeset[offset + 1] & 0b00010000 != 0;
                let add_bool = codeset[offset + 1] & 1 != 0;
                let register_bool = codeset[offset + 2] & 0b00010000 != 0;
                let register = codeset[offset + 3] & 0xF;
                let value = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                let add = match (add_bool, use_base_address) {
                    (true, true) => AddAddress::BaseAddress,
                    (true, false) => AddAddress::PointerAddress,
                    (false, _) => AddAddress::None,
                };

                let add_gecko_register = if register_bool { Some(register) } else { None };

                if execute {
                    let mut value = value;
                    match add {
                        AddAddress::BaseAddress => value += base_address,
                        AddAddress::PointerAddress => value += pointer_address,
                        AddAddress::None => {}
                    }

                    if let Some(gecko_register) = add_gecko_register {
                        value += gecko_registers[gecko_register as usize];
                    }

                    if add_result {
                        base_address += value;
                    } else {
                        base_address = value;
                    }
                }

                offset += 8;
            }
            0x44 => {
                let add_mem_address_bool = codeset[offset + 1] & 1 != 0;
                let register_bool = codeset[offset + 2] & 0b00010000 != 0;
                let register = codeset[offset + 3] & 0xF;
                let mem_address = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                let add_mem_address = match (add_mem_address_bool, use_base_address) {
                    (true, true) => AddAddress::BaseAddress,
                    (true, false) => AddAddress::PointerAddress,
                    (false, _) => AddAddress::None,
                };

                let add_mem_address_gecko_register =
                    if register_bool { Some(register) } else { None };

                if execute {
                    let mut actual_address = mem_address;
                    match add_mem_address {
                        AddAddress::BaseAddress => actual_address += base_address,
                        AddAddress::PointerAddress => actual_address += pointer_address,
                        AddAddress::None => {}
                    }

                    if let Some(gecko_register) = add_mem_address_gecko_register {
                        actual_address += gecko_registers[gecko_register as usize];
                    }

                    memory.write_u32(actual_address as usize, base_address);
                }

                offset += 8;
            }
            0x46 => {
                let _address_offset = (&codeset[offset + 2..]).read_i16::<BigEndian>().unwrap();

                if execute {
                    // Mess up the value so writes can be ignored while in this state
                    base_address = 0;
                }

                offset += 8;
            }
            0x48 => {
                let add_result = codeset[offset + 1] & 0b00010000 != 0;
                let add_mem_address_bool = codeset[offset + 1] & 1 != 0;
                let register_bool = codeset[offset + 2] & 0b00010000 != 0;
                let register = codeset[offset + 3] & 0xF;
                let mem_address = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                let add_mem_address = match (add_mem_address_bool, use_base_address) {
                    (true, true) => AddAddress::BaseAddress,
                    (true, false) => AddAddress::PointerAddress,
                    (false, _) => AddAddress::None,
                };

                let add_mem_address_gecko_register =
                    if register_bool { Some(register) } else { None };

                if execute {
                    let mut actual_address = mem_address;
                    match add_mem_address {
                        AddAddress::BaseAddress => actual_address += base_address,
                        AddAddress::PointerAddress => actual_address += pointer_address,
                        AddAddress::None => {}
                    }

                    if let Some(gecko_register) = add_mem_address_gecko_register {
                        actual_address += gecko_registers[gecko_register as usize];
                    }

                    if add_result {
                        pointer_address += memory.read_u32(actual_address as usize);
                    } else {
                        pointer_address = memory.read_u32(actual_address as usize);
                    }
                }

                offset += 8;
            }
            0x4A => {
                let add_result = codeset[offset + 1] & 0b00010000 != 0;
                let add_bool = codeset[offset + 1] & 1 != 0;
                let register_bool = codeset[offset + 2] & 0b00010000 != 0;
                let register = codeset[offset + 3] & 0xF;
                let new_address = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                let add = match (add_bool, use_base_address) {
                    (true, true) => AddAddress::BaseAddress,
                    (true, false) => AddAddress::PointerAddress,
                    (false, _) => AddAddress::None,
                };

                let add_gecko_register = if register_bool { Some(register) } else { None };

                if execute {
                    let mut new_address = new_address;
                    match add {
                        AddAddress::BaseAddress => new_address += base_address,
                        AddAddress::PointerAddress => new_address += pointer_address,
                        AddAddress::None => {}
                    }

                    if let Some(gecko_register) = add_gecko_register {
                        new_address += gecko_registers[gecko_register as usize];
                    }

                    if add_result {
                        pointer_address += new_address;
                    } else {
                        pointer_address = new_address;
                    }
                }

                offset += 8;
            }
            0x4C => {
                let add_mem_address_bool = codeset[offset + 1] & 1 != 0;
                let register_bool = codeset[offset + 2] & 0b00010000 != 0;
                let register = codeset[offset + 3] & 0xF;
                let mem_address = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                let add_mem_address = match (add_mem_address_bool, use_base_address) {
                    (true, true) => AddAddress::BaseAddress,
                    (true, false) => AddAddress::PointerAddress,
                    (false, _) => AddAddress::None,
                };

                let add_mem_address_gecko_register =
                    if register_bool { Some(register) } else { None };

                if execute {
                    let mut actual_address = mem_address;
                    match add_mem_address {
                        AddAddress::BaseAddress => actual_address += base_address,
                        AddAddress::PointerAddress => actual_address += pointer_address,
                        AddAddress::None => {}
                    }

                    if let Some(gecko_register) = add_mem_address_gecko_register {
                        actual_address += gecko_registers[gecko_register as usize];
                    }

                    memory.write_u32(actual_address as usize, pointer_address);
                }

                offset += 8;
            }
            0x4E => {
                let _address_offset = (&codeset[offset + 2..]).read_i16::<BigEndian>().unwrap();

                if execute {
                    // Mess up the value so writes can be ignored while in this state
                    pointer_address = 0;
                }

                offset += 8;
            }
            0x60 => {
                let _count = (&codeset[offset + 2..]).read_u16::<BigEndian>().unwrap();
                let _block_id = codeset[offset + 7];

                offset += 8;
            }
            0x62 => {
                let _block_id = codeset[offset + 7] & 0xF;

                offset += 8;
            }
            0x64 => {
                let _flag = match codeset[offset + 1] {
                    0x00 => JumpFlag::WhenTrue,
                    0x10 => JumpFlag::WhenFalse,
                    0x20 => JumpFlag::Always,
                    flag => {
                        error!("Unknown jump flag '{}' in return", flag);
                        break;
                    }
                };
                let _block_id = codeset[offset + 7] & 0xF;

                offset += 8;
            }
            0x66 => {
                let flag = match codeset[offset + 1] {
                    0x00 => JumpFlag::WhenTrue,
                    0x10 => JumpFlag::WhenFalse,
                    0x20 => JumpFlag::Always,
                    flag => {
                        error!("Unknown jump flag '{}' in goto", flag);
                        break;
                    }
                };

                let offset_lines = (&codeset[offset + 2..]).read_i16::<BigEndian>().unwrap();

                offset += 8;

                match flag {
                    JumpFlag::WhenTrue => {
                        if execute {
                            offset += 8 * offset_lines as usize;
                        }
                    }
                    JumpFlag::WhenFalse => {
                        if !execute {
                            offset += 8 * offset_lines as usize;
                        }
                    }
                    JumpFlag::Always => {
                        offset += 8 * offset_lines as usize;
                    }
                }
            }
            0x68 => {
                let _flag = match codeset[offset + 1] {
                    0x00 => JumpFlag::WhenTrue,
                    0x10 => JumpFlag::WhenFalse,
                    0x20 => JumpFlag::Always,
                    flag => {
                        error!("Unknown jump flag '{}' in subroutine", flag);
                        break;
                    }
                };
                let _offset_lines = (&codeset[offset + 2..]).read_i16::<BigEndian>().unwrap();
                let _block_id = codeset[offset + 7] & 0xF;

                offset += 8;
            }
            0x80 => {
                let add_result = codeset[offset + 1] & 0b00010000 != 0;
                let add_bool = codeset[offset + 1] & 1 != 0;
                let register = codeset[offset + 3] & 0xF;
                let new_value = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                if execute {
                    let mut new_value = new_value;
                    let add = match (add_bool, use_base_address) {
                        (true, true) => AddAddress::BaseAddress,
                        (true, false) => AddAddress::PointerAddress,
                        (false, _) => AddAddress::None,
                    };

                    match add {
                        AddAddress::BaseAddress => new_value += base_address,
                        AddAddress::PointerAddress => new_value += pointer_address,
                        AddAddress::None => {}
                    }

                    if add_result {
                        gecko_registers[register as usize] += new_value;
                    } else {
                        gecko_registers[register as usize] = new_value;
                    }
                }

                offset += 8;
            }
            0x82 => {
                let _register = codeset[offset + 3] & 0xF;
                let _mem_address = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                // TODO

                offset += 8;
            }
            0x84 => {
                let _register = codeset[offset + 3] & 0xF;
                let _mem_address = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                // TODO

                offset += 8;
            }
            0x86 => {
                let operation_byte = codeset[offset + 1] & 0xF0;
                let _load_register = codeset[offset + 1] & 0b00000001 != 0;
                let _load_value = codeset[offset + 1] & 0b00000010 != 0;
                let _register = codeset[offset + 3] & 0x0F;
                let _value = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                let _operation = GeckoOperation::new(operation_byte);

                offset += 8;
            }
            0x88 => {
                let operation_byte = codeset[offset + 1] & 0xF0;
                let _load_register1 = codeset[offset + 1] & 0b00000001 != 0;
                let _load_register2 = codeset[offset + 1] & 0b00000010 != 0;
                let _register1 = codeset[offset + 3] & 0x0F;
                let _register2 = codeset[offset + 7] & 0x0F;

                let _operation = GeckoOperation::new(operation_byte);

                offset += 8;
            }
            0x8A => {
                let _count = (&codeset[offset + 1..]).read_u16::<BigEndian>().unwrap();
                let _source_register = codeset[offset + 3] & 0xF0;
                let _dest_register = codeset[offset + 3] & 0x0F;
                let _dest_offset = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                offset += 8;
            }
            0x8C => {
                let _count = (&codeset[offset + 1..]).read_u16::<BigEndian>().unwrap();
                let _source_register = codeset[offset + 3] & 0xF0;
                let _dest_register = codeset[offset + 3] & 0x0F;
                let _source_offset = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap();

                offset += 8;
            }
            0xC0 => {
                let mut instruction_data = vec![];
                let count = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap() as usize;
                for i in 0..count * 8 {
                    instruction_data.push(codeset[offset + 8 + i]);
                }

                offset += 8 + count * 8;
            }
            0xC2 => {
                let mut instruction_data = vec![];
                let count = (&codeset[offset + 4..]).read_u32::<BigEndian>().unwrap() as usize;
                for i in 0..count * 8 {
                    instruction_data.push(codeset[offset + 8 + i]);
                }

                offset += 8 + count * 8;
            }
            0xE0 => {
                let reset_base_address_high =
                    (&codeset[offset + 4..]).read_u16::<BigEndian>().unwrap();
                let reset_pointer_address_high =
                    (&codeset[offset + 6..]).read_u16::<BigEndian>().unwrap();

                execution_stack.clear();

                if reset_base_address_high != 0 {
                    base_address = (reset_base_address_high as u32) << 16
                }
                if reset_pointer_address_high != 0 {
                    pointer_address = (reset_pointer_address_high as u32) << 16
                }

                offset += 8;
            }
            0xE2 => {
                let else_branch = codeset[offset + 1] & 0x10 != 0;
                let count = codeset[offset + 3];
                let reset_base_address_high =
                    (&codeset[offset + 4..]).read_u16::<BigEndian>().unwrap();
                let reset_pointer_address_high =
                    (&codeset[offset + 6..]).read_u16::<BigEndian>().unwrap();

                for _ in 0..count {
                    execution_stack.pop();
                }

                if else_branch {
                    // TODO: not sure if this should be taken from the stack before it is pop'd
                    let last = execution_stack.last().cloned().unwrap_or(true);

                    execution_stack.push(!last);
                }

                if reset_base_address_high != 0 {
                    base_address = (reset_base_address_high as u32) << 16
                }
                if reset_pointer_address_high != 0 {
                    pointer_address = (reset_pointer_address_high as u32) << 16
                }

                offset += 8;
            }
            0xF0 => {
                // End of codes
            }
            unknown => {
                // Can't really continue processing because we dont know what the correct offset should be.
                // Report an error and return what we have so far.
                error!("Cannot process WiiRD code starting with 0x{:x}", unknown);
                break;
            }
        }
    }

    memory
}
