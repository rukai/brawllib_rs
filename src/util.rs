use fancy_slice::FancySlice;

pub(crate) fn list_offset(data: FancySlice) -> ListOffset {
    ListOffset {
        start_offset: data.i32_be(0x0),
        count: data.i32_be(0x4),
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
        } else {
            break;
        }
    }
    tag
}
