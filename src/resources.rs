use byteorder::{BigEndian, ReadBytesExt};

use util;

pub(crate) fn resources(data: &[u8]) -> Vec<Resource> {
    let _total_size = (&data[..]).read_i32::<BigEndian>().unwrap();
    let num_children = (&data[4 ..]).read_i32::<BigEndian>().unwrap();

    let mut resources = vec!();
    for i in 1..num_children+1 { // the first child is a dummy so we skip it.
        let child_index = RESOURCE_HEADER_SIZE + RESOURCE_SIZE * i as usize;

        let string_offset = (&data[child_index + 8 .. ]).read_i32::<BigEndian>().unwrap();
        let string_data = &data[string_offset as usize .. ];
        let string = String::from(util::parse_str(string_data).unwrap());
        let data_offset = (&data[child_index + 0xc .. ]).read_i32::<BigEndian>().unwrap();

        resources.push(Resource {
            id:          (&data[child_index as usize       .. ]).read_u16::<BigEndian>().unwrap(),
            flag:        (&data[child_index as usize + 0x2 .. ]).read_u16::<BigEndian>().unwrap(),
            left_index:  (&data[child_index as usize + 0x4 .. ]).read_u16::<BigEndian>().unwrap(),
            right_index: (&data[child_index as usize + 0x6 .. ]).read_u16::<BigEndian>().unwrap(),
            string_offset,
            data_offset,
            string,
        });
    }

    resources
}

const RESOURCE_HEADER_SIZE: usize = 0x8;

const RESOURCE_SIZE: usize = 0x10;
#[derive(Debug)]
pub struct Resource {
    id: u16,
    flag: u16,
    left_index: u16,
    right_index: u16,
    pub string_offset: i32,
    pub data_offset:   i32,
    pub string:        String,
}

