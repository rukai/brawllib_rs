use byteorder::{BigEndian, ReadBytesExt};

pub(crate) fn scripts(parent_data: &[u8], offset_data: &[u8], num: usize) -> Vec<Script> {
    let mut result = vec!();
    for i in 0..num {
        let offset = (&offset_data[i * 4..]).read_i32::<BigEndian>().unwrap() as usize;

        let events = if offset > 0 {
            let mut events = vec!();
            let mut event_offset = offset;
            loop {
                let namespace       =   parent_data[event_offset];
                let code            =   parent_data[event_offset + 1];
                let num_arguments   =   parent_data[event_offset + 2];
                let unk1            =   parent_data[event_offset + 3];
                let argument_offset = (&parent_data[event_offset + 4 ..]).read_u32::<BigEndian>().unwrap();
                let raw_id = (&parent_data[event_offset ..]).read_u32::<BigEndian>().unwrap();

                if code == 0 && namespace == 0 { // seems hacky but its what brawlbox does
                    break
                }

                // Dont really understand what FADEF00D or 0xFADE0D8A means but it's apparently added by PSA
                // and brawlbox just skips arguments on events that have an Id of 0xFADEF00D
                if raw_id != 0xFADEF00D && raw_id != 0xFADE0D8A {
                    let arguments = arguments(parent_data, argument_offset as usize, num_arguments as usize);
                    events.push(Event {
                        namespace,
                        code,
                        unk1,
                        arguments,
                    });
                }

                event_offset += EVENT_SIZE;
            }
            events
        } else {
            vec!()
        };
        result.push(Script { events });
    }
    result
}

fn arguments(parent_data: &[u8], argument_offset: usize, num_arguments: usize) -> Vec<Argument> {
    let mut arguments = vec!();
    for i in 0..num_arguments as usize {
        let argument_offset = argument_offset as usize + i * ARGUMENT_SIZE;
        let ty   = (&parent_data[argument_offset     ..]).read_i32::<BigEndian>().unwrap();
        let data = (&parent_data[argument_offset + 4 ..]).read_i32::<BigEndian>().unwrap();

        let argument = match ty {
            0 => Argument::Value (data),
            1 => Argument::Scalar (data as f32 / 60000.0),
            2 => Argument::Offset (data),
            3 => Argument::Bool (data == 1),
            4 => Argument::File (data),
            5 => Argument::Variable (data),
            6 => Argument::Requirement (data),
            _ => Argument::Unknown (ty, data),
        };
        arguments.push(argument);
    }

    arguments
}

#[derive(Clone, Debug)]
pub struct Script {
    pub events: Vec<Event>
}

// Events are like lines of code in a script
const EVENT_SIZE: usize = 0x8;
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub enum Argument {
    Scalar (f32),
    Bool (bool),
    File (i32),
    Variable (i32),
    Requirement (i32),
    EnumValue (i32),
    Value (i32),
    Offset (i32),
    Unknown (i32, i32)
}
