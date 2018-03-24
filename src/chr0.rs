use byteorder::{BigEndian, ReadBytesExt};
use cgmath::{Vector3, Matrix4};

use util;
use resources;
use math;

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

        let scale = keyframe_holder(child_data, &mut data_offset, code.scale_exists(), code.scale_isotropic(), code.scale_fixed_x(), code.scale_fixed_y(), code.scale_fixed_z(), code.scale_format(), num_frames);
        let rot = keyframe_holder(child_data, &mut data_offset, code.rot_exists(), code.rot_isotropic(), code.rot_fixed_x(), code.rot_fixed_y(), code.rot_fixed_z(), code.rot_format(), num_frames);
        let translation = keyframe_holder(child_data, &mut data_offset, code.translation_exists(), code.translation_isotropic(), code.translation_fixed_x(), code.translation_fixed_y(), code.translation_fixed_z(), code.translation_format(), num_frames);

        children.push(Chr0Child {
            string_offset: resource.string_offset,
            name:          resource.string,
            code,
            scale,
            rot,
            translation,
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
    pub loop_value: bool,
    scaling_rule: i32,
    pub children: Vec<Chr0Child>
}

const CHR0_CHILD_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct Chr0Child {
    string_offset: i32,
    pub name: String,
    pub scale:       KeyframeHolder,
    pub rot:         KeyframeHolder,
    pub translation: KeyframeHolder,
    code: Chr0ChildCode,
}

impl Chr0Child {
    pub fn get_transform(&self, loop_value: bool, frame: i32) -> Matrix4<f32> {
        let scale = self.scale.get_value(loop_value, frame, 1.0);
        let rot = self.rot.get_value(loop_value, frame, 0.0);
        let translation = self.translation.get_value(loop_value, frame, 0.0);
        math::gen_transform(scale, rot, translation)
    }

    pub fn debug_string(&self, loop_value: bool, frames: i32) -> String {
        let mut result = format!("bone name: {}\n", self.name);
        for i in 0..frames {
            if i != 0 {
                result.push('\n');
            }
            result.push_str(&format!("frame: {}\n", i+1));
            result.push_str(&format!("translation: {:?}\n", self.translation.get_value(loop_value, i, 0.0)));
            result.push_str(&format!("rot: {:?}\n", self.rot.get_value(loop_value, i, 0.0)));
            result.push_str(&format!("scale: {:?}\n", self.scale.get_value(loop_value, i, 1.0)));
        }
        result
    }
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
        6 => Chr0Format::Linear4,
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
    Linear4,
}

fn keyframe_holder(child_data: &[u8], data_offset: &mut usize, exists: bool, isotropic: bool, fixed_x: bool, fixed_y: bool, fixed_z: bool, format: Chr0Format, num_frames: u16) -> KeyframeHolder {
    if !exists {
        KeyframeHolder::None
    } else if isotropic {
        let keyframe = if fixed_z {
            Keyframe::Fixed((&child_data[*data_offset..]).read_f32::<BigEndian>().unwrap())
        } else {
            let offset = (&child_data[*data_offset..]).read_u32::<BigEndian>().unwrap();
            keyframe(&child_data[offset as usize ..], &format, num_frames)
        };
        *data_offset += 4;
        KeyframeHolder::Isotropic (keyframe)
    } else {
        let x = if fixed_x {
            Keyframe::Fixed((&child_data[*data_offset..]).read_f32::<BigEndian>().unwrap())
        } else {
            let offset = (&child_data[*data_offset..]).read_u32::<BigEndian>().unwrap();
            keyframe(&child_data[offset as usize ..], &format, num_frames)
        };
        *data_offset += 4;

        let y = if fixed_y {
            Keyframe::Fixed((&child_data[*data_offset..]).read_f32::<BigEndian>().unwrap())
        } else {
            let offset = (&child_data[*data_offset..]).read_u32::<BigEndian>().unwrap();
            keyframe(&child_data[offset as usize ..], &format, num_frames)
        };
        *data_offset += 4;

        let z = if fixed_z {
            Keyframe::Fixed((&child_data[*data_offset..]).read_f32::<BigEndian>().unwrap())
        } else {
            let offset = (&child_data[*data_offset..]).read_u32::<BigEndian>().unwrap();
            keyframe(&child_data[offset as usize ..], &format, num_frames)
        };
        *data_offset += 4;
        KeyframeHolder::Individual { x, y, z }
    }
}

#[derive(Debug)]
pub enum KeyframeHolder {
    Isotropic (Keyframe),
    Individual { x: Keyframe, y: Keyframe, z: Keyframe },
    None,
}

impl KeyframeHolder {
    pub fn get_value(&self, loop_value: bool, frame: i32, default: f32) -> Vector3<f32> {
        match self {
            &KeyframeHolder::Isotropic (ref keyframe) => {
                let value = keyframe.get_value(loop_value, frame);
                Vector3::new(value, value, value)
            }
            &KeyframeHolder::Individual { ref x, ref y, ref z } => {
                Vector3::new(x.get_value(loop_value, frame), y.get_value(loop_value, frame), z.get_value(loop_value, frame))
            }
            &KeyframeHolder::None => Vector3::new(default, default, default)
        }
    }
}

#[derive(Debug)]
pub enum Keyframe {
    Fixed (f32),
    Interpolated4 (Interpolated4Header),
    Interpolated6 (Interpolated6Header),
    Interpolated12 (Interpolated12Header),
    Linear1 (Linear1Header),
    Linear2 (Linear2Header),
    Linear4 (Vec<f32>),
}

impl Keyframe {
    pub fn get_value(&self, loop_value: bool, frame: i32) -> f32 {
        match self {
            &Keyframe::Fixed(value) => {
                value
            }
            &Keyframe::Interpolated4(ref header) => {
                let children: Vec<_> = header.children.iter().map(|child| {
                    InterpolatedNEntry {
                        value: header.base + header.step * child.step() as f32,
                        frame_index: child.frame_index() as i32,
                        tangent: child.tangent() as f32
                    }
                }).collect();
                Keyframe::get_value_interpolated_n_entry(&children, loop_value, frame)
            }
            &Keyframe::Interpolated6(ref header) => {
                let children: Vec<_> = header.children.iter().map(|child| {
                    InterpolatedNEntry {
                        value: header.base + header.step * child.step as f32,
                        frame_index: child.frame_index(),
                        tangent: child.tangent() as f32
                    }
                }).collect();
                Keyframe::get_value_interpolated_n_entry(&children, loop_value, frame)
            }
            &Keyframe::Interpolated12(ref header) => {
                let children: Vec<_> = header.children.iter().map(|child| {
                    InterpolatedNEntry {
                        value:       child.value,
                        frame_index: child.frame_index as i32,
                        tangent:     child.tangent,
                    }
                }).collect();
                Keyframe::get_value_interpolated_n_entry(&children, loop_value, frame)
            }
            &Keyframe::Linear1(ref header) => {
                // TODO: Pretty sure I need to interpolate these still to handle non integer frame indexes (currently frame is a usize though so lol)
                //let mut children = vec!();
                //for (i, child_step) in header.children_steps.iter().enumerate() {
                //    let weight_count = 0;
                //    let tangent = 0.0;
                //    children.push(Interpolated12Entry {
                //        value: header.base + header.step * *child_step as f32,
                //        frame_index: i as f32,
                //        tangent,
                //    });
                //}
                //return Keyframe::get_value_interpolated_n_entry(&children, loop_value, frame);
                header.base + header.step as f32 * header.children_steps[frame as usize] as f32
            }
            &Keyframe::Linear2(ref header) => {
                header.base + header.step * header.children_steps[frame as usize] as f32
            }
            &Keyframe::Linear4(ref values) => {
                values[frame as usize]
            }
        }
    }

    /// to be generic we take InterpolatedNEntry's as we can convert all other formats can be converted to this format
    fn get_value_interpolated_n_entry(children: &[InterpolatedNEntry], loop_value: bool, frame: i32) -> f32 {
        // NOTE: the loop flag is very rarely used (most looping actions such as run or wait dont even use it)
        let mut prev_prev: Option<&InterpolatedNEntry> = None; // the keyframe before the keyframe before the current frame
        let mut prev: Option<&InterpolatedNEntry> = None; // the keyframe before the current frame
        let mut next: Option<&InterpolatedNEntry> = None; // the keyframe after the current frame
        let mut next_next: Option<&InterpolatedNEntry> = None; // the keyframe after the keyframe after the current frame

        // find prev and next
        for child in children {
            if child.frame_index <= frame && prev.map_or(true, |x| child.frame_index > x.frame_index) {
                prev = Some(child);
            }
            if child.frame_index >= frame && next.map_or(true, |x| child.frame_index < x.frame_index) {
                next = Some(child);
            }
        }

        // find prev and next handling looping animations
        // TODO: Might be broken, brawlbox seems more complicated
        if loop_value {
            if prev.is_none() {
                prev = children.iter().max_by_key(|x| x.frame_index);
            }
            if next.is_none() {
                next = children.iter().min_by_key(|x| x.frame_index);
            }
        }

        // find prev_prev and next_next
        for child in children {
            if let Some(prev) = prev {
                if child.frame_index <= prev.frame_index && prev as *const _ != child as *const _ && prev_prev.map_or(true, |x| child.frame_index > x.frame_index) {
                    prev_prev = Some(child);
                }
            }
            if let Some(next) = next {
                if child.frame_index >= next.frame_index && next as *const _ != child as *const _ && next_next.map_or(true, |x| child.frame_index < x.frame_index) {
                    next_next = Some(child);
                }
            }
        }

        match (prev, next) {
            (Some(prev), Some(next)) => {
                if prev as *const _ == next as *const _ { // TODO: Should this be prev.value == next.value
                    prev.value
                } else {
                    let one_apart = next.frame_index == prev.frame_index + 1;
                    let prev_double = if let Some(prev_prev) = prev_prev {
                        prev_prev.frame_index >= 0 && prev_prev.frame_index == prev.frame_index - 1
                    } else {
                        false
                    };
                    let next_double = if let Some(next_next) = next_next {
                        next_next.frame_index >= 0 && next_next.frame_index== next.frame_index + 1
                    } else {
                        false
                    };

                    let double_value = (next.value - prev.value) / (next.frame_index - prev.frame_index) as f32;
                    let prev_tangent = if one_apart || prev_double { double_value } else { prev.tangent };
                    let next_tangent = if one_apart || next_double { double_value } else { next.tangent };

                    // Interpolate using a hermite curve
                    let value_diff = next.value - prev.value;
                    let span = next.frame_index - prev.frame_index;
                    let offset = frame - prev.frame_index;
                    let time = offset as f32 / span as f32;
                    let time_inv = time - 1.0;
                    let result = prev.value
                        + (offset as f32 * time_inv * (time_inv * prev_tangent + time * next_tangent))
                        + ((time * time) * (3.0 - 2.0 * time) * value_diff);

                    result
                }
            }
            (Some(child), None) | (None, Some(child)) => {
                child.value
            }
            (None, None) => unreachable!()
        }
    }
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
    pub fn frame_index(&self) -> u8  { ((self.data & 0xFF00_0000) >> 24) as u8 }
    pub fn step       (&self) -> u16 { ((self.data & 0x00FF_F000) >> 12) as u16 }
    //     tangent                      (self.data & 0x0000_0FFF) -> refer implementation below

    #[allow(exceeding_bitshifts)]
    pub fn tangent    (&self) -> f32 {
        // We need to read the tangent value as a SIGNED integer so we bit shift to the left and then to the right, so that the +/- bit is correct
        let signed_int = ((self.data as i32) << 20) >> 20;

        // Then we map to [0,1]
        signed_int as f32 / 32.0
    }
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
    tangent: i16,
}

impl Interpolated6Entry {
    pub fn frame_index(&self) -> i32 { (self.frame_index >> 5) as i32 }
    pub fn tangent    (&self) -> f32 { self.tangent as f32 / 256.0 }
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

#[derive(Debug)]
pub struct InterpolatedNEntry {
    pub frame_index: i32,
    pub value: f32,
    pub tangent: f32,
}

const LINEAR_1_HEADER_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct Linear1Header {
    pub step: f32,
    pub base: f32,
    pub children_steps: Vec<u8>,
}

const LINEAR_2_HEADER_SIZE: usize = 0x8;
#[derive(Debug)]
pub struct Linear2Header {
    pub step: f32,
    pub base: f32,
    pub children_steps: Vec<u16>,
}

const LINEAR_2_ENTRY_SIZE: usize = 0x2;
const LINEAR_4_ENTRY_SIZE: usize = 0x4;

fn keyframe(data: &[u8], format: &Chr0Format, num_frames: u16) -> Keyframe {
    match format {
        &Chr0Format::Interpolated4 => {
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
        &Chr0Format::Interpolated6 => {
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
                let tangent     = (&child_data[0x4..]).read_i16::<BigEndian>().unwrap();
                children.push(Interpolated6Entry { frame_index, step, tangent });
            }
            Keyframe::Interpolated6 (Interpolated6Header { num_frames, unk, frame_scale, step, base, children })
        }
        &Chr0Format::Interpolated12 => {
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
        &Chr0Format::Linear1 => {
            let step = (&data[0x0..]).read_f32::<BigEndian>().unwrap();
            let base = (&data[0x4..]).read_f32::<BigEndian>().unwrap();
            let children_steps = data[LINEAR_1_HEADER_SIZE .. LINEAR_1_HEADER_SIZE + num_frames as usize].iter().cloned().collect();
            Keyframe::Linear1 (Linear1Header { step, base, children_steps })
        }
        &Chr0Format::Linear2 => {
            let step = (&data[0x0..]).read_f32::<BigEndian>().unwrap();
            let base = (&data[0x4..]).read_f32::<BigEndian>().unwrap();

            let mut children_steps = vec!();
            for i in 0..num_frames as usize {
                children_steps.push((&data[LINEAR_2_HEADER_SIZE + LINEAR_2_ENTRY_SIZE * i ..]).read_u16::<BigEndian>().unwrap());
            }
            Keyframe::Linear2 (Linear2Header { step, base, children_steps })
        }
        &Chr0Format::Linear4 => {
            let mut values = vec!();
            for i in 0..num_frames as usize {
                values.push((&data[LINEAR_4_ENTRY_SIZE * i ..]).read_f32::<BigEndian>().unwrap());
            }
            Keyframe::Linear4 (values)
        }
        &Chr0Format::None => panic!("this function should not be called with a format of None"),
    }
}
