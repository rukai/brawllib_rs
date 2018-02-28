use std::str;

pub(crate) fn parse_str(data: &[u8]) -> Result<&str, String> {
    if let Some(length) = data.iter().position(|x| *x == 0) {
        str::from_utf8(&data[..length]).map_err(|x| format!("{}", x))
    }
    else {
        Err(String::from("String was not terminated"))
    }
}

pub(crate) fn parse_tag(data: &[u8]) -> String {
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
