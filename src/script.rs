use byteorder::{BigEndian, ReadBytesExt};

pub(crate) fn scripts(parent_data: &[u8], offset_data: &[u8], num: usize) -> Vec<Script> {
    let mut result = vec!();
    for i in 0..num {
        let offset = (&offset_data[i * 4..]).read_i32::<BigEndian>().unwrap() as usize;

        if offset > 0 {
            let mut events = vec!();
            let mut event_offset = offset;
            loop {
                let namespace       =   parent_data[event_offset];
                let code            =   parent_data[event_offset + 1];
                let num_arguments   =   parent_data[event_offset + 2];
                let unk1            =   parent_data[event_offset + 3];
                let argument_offset = (&parent_data[event_offset + 4 ..]).read_u32::<BigEndian>().unwrap();

                if code == 0 && namespace == 0 { // seems hacky but its what brawlbox does
                    break
                }

                let event_id = (&parent_data[event_offset ..]).read_u32::<BigEndian>().unwrap() as usize & 0xFFFFFF00;
                let arguments = arguments(parent_data, event_id, argument_offset as usize, num_arguments as usize);
                events.push(Event {
                    namespace,
                    code,
                    unk1,
                    arguments,
                });

                event_offset += EVENT_SIZE;
            }

            result.push(Script {
                events
            });
        }
    }
    println!("{:#?}", result);
    result
}

fn arguments(parent_data: &[u8], event_id: usize, argument_offset: usize, num_arguments: usize) -> Vec<Argument> {
    let mut arguments = vec!();
    for i in 0..num_arguments as usize {
        let argument_offset = argument_offset as usize + i * ARGUMENT_SIZE;
        let ty   = (&parent_data[argument_offset     ..]).read_i32::<BigEndian>().unwrap();
        let data = (&parent_data[argument_offset + 4 ..]).read_i32::<BigEndian>().unwrap();

        let argument = if (event_id == 0x06000D00 || event_id == 0x06150F00 || event_id == 0x062B0D00) && i == 12 {
            Argument::HitBoxFlags (data)
        } else if (event_id == 0x06000D00 || event_id == 0x05150F00 || event_id == 0x062B0D00) && (i == 0 || i == 3 || i == 4) {
            Argument::Value2Half (data)
        } else if (event_id == 0x11150300 || event_id == 0x11001000 || event_id == 0x11020A00) && i == 0 {
            Argument::ValueGFX (data)
        } else if event_id == 0x06150F00 && i == 14 {
            Argument::SpecialHitboxFlags (data)
        } else {
            match ty {
                0 => Argument::Value (data),
                1 => Argument::Scalar (data),
                2 => Argument::Offset (data),
                3 => Argument::Bool (data),
                4 => Argument::File (data),
                5 => Argument::Variable (data),
                6 => Argument::Requirement (data),
                _ => Argument::Unknown (ty, data),
            }
        };
        arguments.push(argument);
    }

    arguments
}

#[derive(Debug)]
pub struct Script {
    events: Vec<Event>
}

// Events are like lines of code in a script
const EVENT_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct Event {
    namespace: u8,
    code: u8,
    unk1: u8,
    arguments: Vec<Argument>,
}

impl Event {
    pub fn id(&self) -> u32 {
        let num_args = self.arguments.len();
        assert!(num_args < 0x100);
        (self.namespace as u32) << 24 | (self.code as u32) << 16 | num_args as u32
    }
}

const ARGUMENT_SIZE: usize = 0x8;
#[derive(Debug)]
pub enum Argument {
    HitBoxFlags (i32),
    SpecialHitboxFlags (i32),
    Value2Half (i32),
    ValueGFX (i32),
    Scalar (i32),
    Bool (i32),
    File (i32),
    Variable (i32),
    Requirement (i32),
    EnumValue (i32),
    Value (i32),
    Offset (i32),
    Unknown (i32, i32)
}
