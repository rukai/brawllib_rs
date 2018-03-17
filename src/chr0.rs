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

        let code = Chr0ChildCode::new((&child_data[4..]).read_u32::<BigEndian>().unwrap());

        let mut data_offset = CHR0_CHILD_SIZE;

        let mut scale_x_keyframe = Keyframe::None;
        let mut scale_y_keyframe = Keyframe::None;
        let mut scale_z_keyframe = Keyframe::None;
        let rot_x_keyframe = Keyframe::None;
        let rot_y_keyframe = Keyframe::None;
        let rot_z_keyframe = Keyframe::None;
        let translation_x_keyframe = Keyframe::None;
        let translation_y_keyframe = Keyframe::None;
        let translation_z_keyframe = Keyframe::None;

        // TODO: AnimationConverter.cs :113
        if code.scale_exists() {
            if code.scale_isotropic() {
                if code.scale_fixed_z() {
                    let value = (&child_data[data_offset..]).read_f32::<BigEndian>().unwrap();
                    scale_x_keyframe = Keyframe::Fixed(value);
                    scale_y_keyframe = Keyframe::Fixed(value);
                    scale_z_keyframe = Keyframe::Fixed(value);
                }
                else {
                    // TODO
                }
                //data_offset += 4;
            }
            else {
                scale_x_keyframe = if code.scale_fixed_x() {
                    Keyframe::Fixed((&child_data[data_offset..]).read_f32::<BigEndian>().unwrap())
                } else {
                    let offset = (&child_data[data_offset..]).read_u32::<BigEndian>().unwrap();
                    keyframe(&child_data[offset as usize ..], code.scale_format())
                };
                data_offset += 4;

                scale_y_keyframe = if code.scale_fixed_y() {
                    Keyframe::Fixed((&child_data[data_offset..]).read_f32::<BigEndian>().unwrap())
                } else {
                    let offset = (&child_data[data_offset..]).read_u32::<BigEndian>().unwrap();
                    keyframe(&child_data[offset as usize ..], code.scale_format())
                };
                data_offset += 4;

                scale_z_keyframe = if code.scale_fixed_z() {
                    Keyframe::Fixed((&child_data[data_offset..]).read_f32::<BigEndian>().unwrap())
                } else {
                    let offset = (&child_data[data_offset..]).read_u32::<BigEndian>().unwrap();
                    keyframe(&child_data[offset as usize ..], code.scale_format())
                };
                //data_offset += 4;
            };
        }

        children.push(Chr0Child {
            string_offset: resource.string_offset,
            name:          resource.string,
            code,
            scale_x_keyframe,
            scale_y_keyframe,
            scale_z_keyframe,
            rot_x_keyframe,
            rot_y_keyframe,
            rot_z_keyframe,
            translation_x_keyframe,
            translation_y_keyframe,
            translation_z_keyframe,
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
    pub children: Vec<Chr0Child>
}

const CHR0_CHILD_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct Chr0Child {
    string_offset: i32,
    pub name: String,
    pub scale_x_keyframe:       Keyframe,
    pub scale_y_keyframe:       Keyframe,
    pub scale_z_keyframe:       Keyframe,
    pub rot_x_keyframe:         Keyframe,
    pub rot_y_keyframe:         Keyframe,
    pub rot_z_keyframe:         Keyframe,
    pub translation_x_keyframe: Keyframe,
    pub translation_y_keyframe: Keyframe,
    pub translation_z_keyframe: Keyframe,
    code: Chr0ChildCode,
}

#[derive(Debug)]
pub struct Chr0ChildCode {
    value: u32,
}

impl Chr0ChildCode {
    fn new(value: u32) -> Chr0ChildCode {
        assert_eq!(value & 1, 1);
        Chr0ChildCode { value }
    }

    pub fn identity                   (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0000_0000_0010 != 0 } // Scale = 1, Rot = 0, Trans = 0
    pub fn rot_translation_zero       (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0000_0000_0100 != 0 } // Rot = 0, Trans = 0 - Must be set if Identity is set
    pub fn scale_one                  (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0000_0000_1000 != 0 } // Scale = 1          - Must be set if Identity is set

    pub fn scale_isotropic            (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0000_0001_0000 != 0 }
    pub fn rot_isotropic              (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0000_0010_0000 != 0 }
    pub fn translation_isotropic      (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0000_0100_0000 != 0 }

    pub fn use_model_scale            (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0000_1000_0000 != 0 }
    pub fn use_model_rot              (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0001_0000_0000 != 0 }
    pub fn use_model_translation      (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0010_0000_0000 != 0 }

    pub fn scale_compensate_apply     (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_0100_0000_0000 != 0 } // Maya calculations
    pub fn scale_compenstate_parent   (&self) -> bool { self.value & 0b0000_0000_0000_0000_0000_1000_0000_0000 != 0 } // Maya calculations
    pub fn classic_scale_off          (&self) -> bool { self.value & 0b0000_0000_0000_0000_0001_0000_0000_0000 != 0 } // SoftImage calculations

    pub fn scale_fixed_x              (&self) -> bool { self.value & 0b0000_0000_0000_0000_0010_0000_0000_0000 != 0 }
    pub fn scale_fixed_y              (&self) -> bool { self.value & 0b0000_0000_0000_0000_0100_0000_0000_0000 != 0 }
    pub fn scale_fixed_z              (&self) -> bool { self.value & 0b0000_0000_0000_0000_1000_0000_0000_0000 != 0 }

    pub fn rot_fixed_x                (&self) -> bool { self.value & 0b0000_0000_0000_0001_0000_0000_0000_0000 != 0 }
    pub fn rot_fixed_y                (&self) -> bool { self.value & 0b0000_0000_0000_0010_0000_0000_0000_0000 != 0 }
    pub fn rot_fixed_z                (&self) -> bool { self.value & 0b0000_0000_0000_0100_0000_0000_0000_0000 != 0 }

    pub fn translation_fixed_x        (&self) -> bool { self.value & 0b0000_0000_0000_1000_0000_0000_0000_0000 != 0 }
    pub fn translation_fixed_y        (&self) -> bool { self.value & 0b0000_0000_0001_0000_0000_0000_0000_0000 != 0 }
    pub fn translation_fixed_z        (&self) -> bool { self.value & 0b0000_0000_0010_0000_0000_0000_0000_0000 != 0 }

    pub fn scale_exists               (&self) -> bool { self.value & 0b0000_0000_0100_0000_0000_0000_0000_0000 != 0 } // Equivalent to Use Model Scale & Scale One set to false
    pub fn rot_exists                 (&self) -> bool { self.value & 0b0000_0000_1000_0000_0000_0000_0000_0000 != 0 } // Equivalent to Use Model Rotation & Rotation Zero set to false
    pub fn translation_exists         (&self) -> bool { self.value & 0b0000_0001_0000_0000_0000_0000_0000_0000 != 0 } // Equivalent to Use Model Translation & Translation Zero set to false

    pub fn scale_format      (&self) -> Chr0Format { f((self.value & 0b0000_0110_0000_0000_0000_0000_0000_0000) >> 25) }
    pub fn rot_format        (&self) -> Chr0Format { f((self.value & 0b0011_1000_0000_0000_0000_0000_0000_0000) >> 27) }
    pub fn translation_format(&self) -> Chr0Format { f((self.value & 0b1100_0000_0000_0000_0000_0000_0000_0000) >> 30) }
}

fn f(value: u32) -> Chr0Format {
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

#[derive(Debug)]
pub enum Keyframe {
    None,
    Fixed (f32),
    Interpolated4 (Interpolated4Header),
    Interpolated6 (Interpolated6Header),
    Interpolated12 (Interpolated12Header),
}

const INTERPOLATED_4_HEADER_SIZE: usize = 0x10;
#[derive(Debug)]
pub struct Interpolated4Header {
    pub entries: u16,
    pub unk: u16,
    pub frame_scale: f32,
    pub step: f32,
    pub base: f32,
    pub children: Vec<Interpolated4Entry>,
}

const INTERPOLATED_4_ENTRY_SIZE: usize = 0x4;
#[derive(Debug)]
pub struct Interpolated4Entry {
    pub data: u32,
}

impl Interpolated4Entry {
    pub fn frame_index(&self) -> u32 { (self.data & 0xFF00_0000) >> 24 }
    pub fn step       (&self) -> u32 { (self.data & 0x00FF_F000) >> 12 }
    pub fn tangent    (&self) -> f32 { (self.data & 0x0000_0FFF) as f32 }
}

const INTERPOLATED_6_HEADER_SIZE: usize = 0x10;
#[derive(Debug)]
pub struct Interpolated6Header {
    pub num_frames: u16,
    pub unk: u16,
    pub frame_scale: f32,
    pub step: f32,
    pub base: f32,
    pub children: Vec<Interpolated6Entry>,
}

const INTERPOLATED_6_ENTRY_SIZE: usize = 0x6;
#[derive(Debug)]
pub struct Interpolated6Entry {
    frame_index: u16,
    pub step: u16,
    pub tangent: u8, // TODO: Order might be swapped with unk
    pub unk: u8,
}

impl Interpolated6Entry {
    pub fn frame_index(&self) -> u16 { (self.frame_index >> 5) as u16 }
}

const INTERPOLATED_12_HEADER_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct Interpolated12Header {
    pub num_frames: u16,
    pub unk: u16,
    pub frame_scale: f32,
    pub children: Vec<Interpolated12Entry>,
}

const INTERPOLATED_12_ENTRY_SIZE: usize = 0xc;
#[derive(Debug)]
pub struct Interpolated12Entry {
    pub frame_index: f32,
    pub value: f32,
    pub tangent: f32,
}

fn keyframe(data: &[u8], format: Chr0Format) -> Keyframe {
    match format {
        Chr0Format::Interpolated4 => {
            let entries     = (&data[0x0..]).read_u16::<BigEndian>().unwrap();
            let unk         = (&data[0x2..]).read_u16::<BigEndian>().unwrap();
            let frame_scale = (&data[0x4..]).read_f32::<BigEndian>().unwrap();
            let step        = (&data[0x8..]).read_f32::<BigEndian>().unwrap();
            let base        = (&data[0xc..]).read_f32::<BigEndian>().unwrap();
            let mut children = vec!();
            for i in 0..entries as usize {
                let child_data = &data[INTERPOLATED_4_HEADER_SIZE + INTERPOLATED_4_ENTRY_SIZE * i ..];
                let data = (&child_data[..]).read_u32::<BigEndian>().unwrap();
                children.push(Interpolated4Entry { data });
            }
            Keyframe::Interpolated4 (Interpolated4Header { entries, unk, frame_scale, step, base, children })
        }
        Chr0Format::Interpolated6 => {
            let num_frames  = (&data[0x0..]).read_u16::<BigEndian>().unwrap();
            let unk         = (&data[0x2..]).read_u16::<BigEndian>().unwrap();
            let frame_scale = (&data[0x4..]).read_f32::<BigEndian>().unwrap();
            let step        = (&data[0x8..]).read_f32::<BigEndian>().unwrap();
            let base        = (&data[0xc..]).read_f32::<BigEndian>().unwrap();
            let mut children = vec!();
            for i in 0..num_frames as usize {
                let child_data = &data[INTERPOLATED_6_HEADER_SIZE + INTERPOLATED_6_ENTRY_SIZE * i ..];
                let frame_index = (&child_data[0x0..]).read_u16::<BigEndian>().unwrap();
                let step        = (&child_data[0x2..]).read_u16::<BigEndian>().unwrap();
                let tangent     = (&child_data[0x4..]).read_u8().unwrap();
                let unk         = (&child_data[0x5..]).read_u8().unwrap();
                children.push(Interpolated6Entry { frame_index, step, tangent, unk });
            }
            Keyframe::Interpolated6 (Interpolated6Header { num_frames, unk, frame_scale, step, base, children })
        }
        Chr0Format::Interpolated12 => {
            let num_frames  = (&data[0x0..]).read_u16::<BigEndian>().unwrap();
            let unk         = (&data[0x2..]).read_u16::<BigEndian>().unwrap();
            let frame_scale = (&data[0x4..]).read_f32::<BigEndian>().unwrap();

            let mut children = vec!();
            for i in 0..num_frames as usize {
                let child_data = &data[INTERPOLATED_12_HEADER_SIZE + INTERPOLATED_12_ENTRY_SIZE * i ..];
                let frame_index = (&child_data[0x0..]).read_f32::<BigEndian>().unwrap();
                let value       = (&child_data[0x4..]).read_f32::<BigEndian>().unwrap();
                let tangent     = (&child_data[0x8..]).read_f32::<BigEndian>().unwrap();
                children.push(Interpolated12Entry { frame_index, value, tangent });
            }
            Keyframe::Interpolated12 (Interpolated12Header { num_frames, unk, frame_scale, children })
        }
        _ => Keyframe::None,
    }
}
