use fancy_slice::FancySlice;

use crate::wii_texture_formats::WiiPaletteFormat;
use crate::user_data::{UserData, UserDataValue};

pub(crate) fn plt0(data: FancySlice) -> Plt0 {
    let size               = data.i32_be(0x4);
    let version            = data.i32_be(0x8);
    //let bres_offset      = data.i32_be(0xc);
    //let resources_offset = data.i32_be(0x10);
    let string_offset      = data.u32_be(0x14);
    let pixel_format       = data.u32_be(0x18);
    //let num_entries      = data.u16_be(0x1c);
    let orig_path_offset   = data.i32_be(0x20);

    let pixel_format = WiiPaletteFormat::new(pixel_format);

    let user_data = if version == 3 {
        let _user_data_offset = data.i32_be(0x24);
        let mut user_data = vec!();

        // TODO
        user_data.push(UserData {
            name: "TODO".into(),
            value: UserDataValue::Int(42),
        });

        user_data
    } else if version == 1 {
        vec!()
    } else {
        panic!("Unknown PLT0 verison: {}", version)
    };

    let name = data.str(string_offset as usize).unwrap().to_string();

    // TODO: This doesnt necasarily start at PLT0_HEADER_SIZE, maybe the offset is stored in the
    // resources which I havent parsed yet??
    // Brawlcrate seems to just be reading from PLT0_HEADER_SIZE ???
    let color_data: Vec<u16> = data.relative_slice(PLT0_HEADER_SIZE..size as usize)
        .chunks_exact(2)
        .map(|x| u16::from_be_bytes([x[0], x[1]])).collect();

    Plt0 { name, pixel_format, orig_path_offset, user_data, color_data }
}

const PLT0_HEADER_SIZE: usize = 0x40;
#[derive(Clone, Debug)]
pub struct Plt0 {
    pub name:         String,
    pub pixel_format: WiiPaletteFormat,
    pub user_data:    Vec<UserData>,
    pub color_data:   Vec<u16>,
    // TODO: Calculate this, what is it even pointing to?
    orig_path_offset: i32,
}

impl Plt0 {
    pub fn compile(&self, bres_offset: i32) -> Vec<u8> {
        let mut output = vec!();

        let size = PLT0_HEADER_SIZE + self.color_data.len() * 2;
        let version = if self.user_data.len() > 0 { 3 } else { 1 };
        let num_entries = self.color_data.len();

        // create PLT0 header
        output.extend("PLT0".chars().map(|x| x as u8));
        output.extend(&i32::to_be_bytes(size as i32));
        output.extend(&i32::to_be_bytes(version));
        output.extend(&i32::to_be_bytes(bres_offset));
        output.extend(&i32::to_be_bytes(0)); // TODO: resources_offset
        output.extend(&u32::to_be_bytes(0)); // TODO: string_offset
        output.extend(&u32::to_be_bytes(self.pixel_format.value()));
        output.extend(&u16::to_be_bytes(num_entries as u16));
        output.extend(&u16::to_be_bytes(0)); // padding
        output.extend(&i32::to_be_bytes(self.orig_path_offset));
        if self.user_data.len() > 0 {
            output.extend(&i32::to_be_bytes(0x44)); // TODO: I just guessed this is a constant?
        }
        output.extend(&[0; 0x1c]); // padding

        // create user data
        for _user_data in &self.user_data {
            output.push(0x42); // TODO
        }
        if self.user_data.len() > 0 {
            while output.len() % 0x20 != 0 {
                output.push(0x00);
            }
        }

        // create color data
        for color in &self.color_data {
            output.extend(&u16::to_be_bytes(*color));
        }

        output
    }
}
