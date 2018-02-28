use byteorder::{BigEndian, ReadBytesExt};

use util;
use resources;

pub(crate) fn chr0(data: &[u8]) -> Chr0 {
    let size             = (&data[0x4..]).read_i32::<BigEndian>().unwrap();
    let version          = (&data[0x8..]).read_i32::<BigEndian>().unwrap();
    let bres_offset      = (&data[0xc..]).read_i32::<BigEndian>().unwrap();
    let resources_offset = (&data[0x10..]).read_i32::<BigEndian>().unwrap();
    let string_offset    = (&data[0x14..]).read_i32::<BigEndian>().unwrap();
    let orig_path_offset = (&data[0x18..]).read_i32::<BigEndian>().unwrap();
    let num_frames       = (&data[0x1c..]).read_u16::<BigEndian>().unwrap();
    let num_children     = (&data[0x1e..]).read_u16::<BigEndian>().unwrap();
    let loop_value       = (&data[0x20..]).read_i32::<BigEndian>().unwrap();
    let scaling_rule     = (&data[0x24..]).read_i32::<BigEndian>().unwrap();
    assert_eq!(version, 4);

    let name = String::from(util::parse_str(&data[string_offset as usize ..]).unwrap());

    let mut children = vec!();
    for resource in resources::resources(&data[resources_offset as usize ..]) {
        let child_data = &data[resources_offset as usize + resource.data_offset as usize .. ];

        let _string_offset = (&child_data[..]).read_i32::<BigEndian>().unwrap();
        let _string = util::parse_str(&child_data[_string_offset as usize .. ]).unwrap();

        let code = (&child_data[4..]).read_u32::<BigEndian>().unwrap();
        assert_eq!(code & 1, 1);

        children.push(Chr0Child {
            string_offset: resource.string_offset,
            data_offset:   resource.data_offset,
            name:          resource.string,
            code,
        });
    }

    Chr0 {
        name,
        size,
        version,
        bres_offset,
        string_offset,
        orig_path_offset,
        num_frames,
        num_children,
        loop_value: loop_value != 0,
        scaling_rule,
        children,
    }
}

#[derive(Debug)]
pub struct Chr0 {
    pub name: String,
    size: i32,
    version: i32,
    bres_offset: i32,
    string_offset: i32,
    orig_path_offset: i32,
    pub num_frames: u16,
    num_children: u16,
    loop_value: bool,
    scaling_rule: i32,
    children: Vec<Chr0Child>
}

#[derive(Debug)]
pub struct Chr0Child {
    string_offset: i32,
    pub name: String,
    data_offset: i32,
    code: u32,
}

impl Chr0Child {
    pub fn identity                 (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0000_0000_0010 != 0 } // Scale = 1, Rot = 0, Trans = 0
    pub fn rot_trans_zero           (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0000_0000_0100 != 0 } // Rot = 0, Trans = 0 - Must be set if Identity is set
    pub fn scale_one                (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0000_0000_1000 != 0 } // Scale = 1          - Must be set if Identity is set

    pub fn scale_isotropic          (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0000_0001_0000 != 0 }
    pub fn rot_isotropic            (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0000_0010_0000 != 0 }
    pub fn trans_isotropic          (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0000_0100_0000 != 0 }

    pub fn use_model_scale          (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0000_1000_0000 != 0 }
    pub fn use_model_rot            (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0001_0000_0000 != 0 }
    pub fn use_model_trans          (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0010_0000_0000 != 0 }

    pub fn scale_compensate_apply   (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_0100_0000_0000 != 0 } // Maya calculations
    pub fn cale_compenstate_parent  (&self) -> bool { self.code & 0b0000_0000_0000_0000_0000_1000_0000_0000 != 0 } // Maya calculations
    pub fn classic_scale_off        (&self) -> bool { self.code & 0b0000_0000_0000_0000_0001_0000_0000_0000 != 0 } // SoftImage calculations

    pub fn scale_fixed_x            (&self) -> bool { self.code & 0b0000_0000_0000_0000_0010_0000_0000_0000 != 0 }
    pub fn scale_fixed_y            (&self) -> bool { self.code & 0b0000_0000_0000_0000_0100_0000_0000_0000 != 0 }
    pub fn scale_fixed_z            (&self) -> bool { self.code & 0b0000_0000_0000_0000_1000_0000_0000_0000 != 0 }

    pub fn rot_fixed_x              (&self) -> bool { self.code & 0b0000_0000_0000_0001_0000_0000_0000_0000 != 0 }
    pub fn rot_fixed_y              (&self) -> bool { self.code & 0b0000_0000_0000_0010_0000_0000_0000_0000 != 0 }
    pub fn rot_fixed_z              (&self) -> bool { self.code & 0b0000_0000_0000_0100_0000_0000_0000_0000 != 0 }

    pub fn trans_fixed_x            (&self) -> bool { self.code & 0b0000_0000_0000_1000_0000_0000_0000_0000 != 0 }
    pub fn trans_fixed_y            (&self) -> bool { self.code & 0b0000_0000_0001_0000_0000_0000_0000_0000 != 0 }
    pub fn trans_fixed_z            (&self) -> bool { self.code & 0b0000_0000_0010_0000_0000_0000_0000_0000 != 0 }

    pub fn scale_exists             (&self) -> bool { self.code & 0b0000_0000_0100_0000_0000_0000_0000_0000 != 0 } // Equivalent to Use Model Scale & Scale One set to false
    pub fn rot_exists               (&self) -> bool { self.code & 0b0000_0000_1000_0000_0000_0000_0000_0000 != 0 } // Equivalent to Use Model Rotation & Rotation Zero set to false
    pub fn trans_exists             (&self) -> bool { self.code & 0b0000_0001_0000_0000_0000_0000_0000_0000 != 0 } // Equivalent to Use Model Translation & Translation Zero set to false

    pub fn scale_format(&self) -> Chr0Format { format(self.code & 0b0000_0110_0000_0000_0000_0000_0000_0000) }
    pub fn rot_format  (&self) -> Chr0Format { format(self.code & 0b0011_1000_0000_0000_0000_0000_0000_0000) }
    pub fn trans_format(&self) -> Chr0Format { format(self.code & 0b1100_0000_0000_0000_0000_0000_0000_0000) }
}

fn format(value: u32) -> Chr0Format {
    match value {
        0 => Chr0Format::None,
        1 => Chr0Format::Interpolated4,
        2 => Chr0Format::Interpolated6,
        3 => Chr0Format::Interpolated12,
        4 => Chr0Format::Linear1,
        5 => Chr0Format::Linear2,
        6 => Chr0Format::Linear3,
        _ => unreachable!()
    }
}

#[derive(Debug)]
pub enum Chr0Format {
    None,
    Interpolated4,
    Interpolated6,
    Interpolated12,
    Linear1,
    Linear2,
    Linear3,
}
