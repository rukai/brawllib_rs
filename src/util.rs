use fancy_slice::FancySlice;

pub(crate) fn list_offset(data: FancySlice) -> ListOffset {
    ListOffset {
        start_offset: data.i32_be(0x0),
        count:        data.i32_be(0x4),
    }
}

pub(crate) const LIST_OFFSET_SIZE: usize = 0x8;
#[derive(Debug)]
pub(crate) struct ListOffset {
    pub start_offset: i32,
    pub count: i32,
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
