use std::str;

use byteorder::{BigEndian, ReadBytesExt};

pub(crate) fn list_offset(data: &[u8]) -> ListOffset {
    ListOffset {
        start_offset: (&data[0x0..]).read_i32::<BigEndian>().unwrap(),
        count:        (&data[0x4..]).read_i32::<BigEndian>().unwrap(),
    }
}

pub(crate) const LIST_OFFSET_SIZE: usize = 0x8;
#[derive(Debug)]
pub(crate) struct ListOffset {
    pub start_offset: i32,
    pub count: i32,
}

pub fn parse_str(data: &[u8]) -> Result<&str, String> {
    if let Some(length) = data.iter().position(|x| *x == 0) {
        str::from_utf8(&data[..length]).map_err(|x| format!("{}", x))
    }
    else {
        Err(String::from("String was not terminated"))
    }
}

pub fn parse_tag(data: &[u8]) -> String {
    let mut tag = String::new();
    for j in 0..4 {
        let byte = data[j] as char;
        if byte.is_ascii_graphic() {
            tag.push(byte);
        }
        else {
            break;
        }
    }
    tag
}

#[allow(unused)]
pub fn hex_dump(data: &[u8]) -> String {
    let mut string = String::new();
    for (i, byte) in data.iter().enumerate() {
        if i != 0 && i % 2 == 0 {
            string.push_str(" ");
        }
        string.push_str(&format!("{:02x}", byte));
    }
    string
}

#[allow(unused)]
pub fn ascii_dump(data: &[u8]) -> String {
    let mut string = String::new();
    for byte in data {
        let ascii = *byte as char;
        if ascii.is_ascii_graphic() {
            string.push(ascii);
        }
        else {
            string.push('.');
        }
    }
    string
}
