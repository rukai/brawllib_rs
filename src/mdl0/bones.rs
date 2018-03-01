use byteorder::{BigEndian, ReadBytesExt};
use cgmath::{Vector3, Matrix4};

use mbox::MBox;
use mbox;
use resources::Resource;
use util;

pub(crate) fn bones(data: &[u8], resources: Vec<Resource>) -> Bone {
    bone_siblings(&data[resources[0].data_offset as usize ..]).pop().unwrap()
}

fn bone_siblings(data: &[u8]) -> Vec<Bone> {
    let header_len    = (&data[0x00..]).read_i32::<BigEndian>().unwrap();
    let mdl0_offset   = (&data[0x04..]).read_i32::<BigEndian>().unwrap();
    let string_offset = (&data[0x08..]).read_i32::<BigEndian>().unwrap();
    let index         = (&data[0x0c..]).read_i32::<BigEndian>().unwrap();
    let node_id       = (&data[0x10..]).read_i32::<BigEndian>().unwrap();
    let flags_int     = (&data[0x14..]).read_u32::<BigEndian>().unwrap();
    let billboard_int = (&data[0x18..]).read_u32::<BigEndian>().unwrap();
    let bb_index      = (&data[0x1c..]).read_u32::<BigEndian>().unwrap();

    let scale = Vector3::<f32>::new(
        (&data[0x20..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x24..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x28..]).read_f32::<BigEndian>().unwrap(),
    );
    let rot = Vector3::<f32>::new(
        (&data[0x2c..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x30..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x34..]).read_f32::<BigEndian>().unwrap(),
    );
    let trans = Vector3::<f32>::new(
        (&data[0x38..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x3c..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x40..]).read_f32::<BigEndian>().unwrap(),
    );

    let extents  = mbox::mbox(&data[0x44..]);
    let _parent_offset     = (&data[0x5c..]).read_i32::<BigEndian>().unwrap();
    let first_child_offset = (&data[0x60..]).read_i32::<BigEndian>().unwrap();
    let next_offset        = (&data[0x64..]).read_i32::<BigEndian>().unwrap();
    let _prev_offset       = (&data[0x68..]).read_i32::<BigEndian>().unwrap();
    let user_data_offset   = (&data[0x6c..]).read_i32::<BigEndian>().unwrap();

    // BrawlBox uses a Matrix43 for this which I'm assuming is a 4 x 3 matrix.
    // This means there are 4 rows and 3 columns we need to read.
    // We only have 4x4 matrices available, so use [0.0, 0.0 0.0, 1.0] as the last column,
    let transform = Matrix4::new(
        (&data[0x70..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x74..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x78..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x7c..]).read_f32::<BigEndian>().unwrap(),

        (&data[0x80..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x84..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x88..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x8c..]).read_f32::<BigEndian>().unwrap(),

        (&data[0x90..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x94..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x98..]).read_f32::<BigEndian>().unwrap(),
        (&data[0x9c..]).read_f32::<BigEndian>().unwrap(),

        0.0,
        0.0,
        0.0,
        1.0,
    );

    let transform_inv = Matrix4::new(
        (&data[0xa0..]).read_f32::<BigEndian>().unwrap(),
        (&data[0xa4..]).read_f32::<BigEndian>().unwrap(),
        (&data[0xa8..]).read_f32::<BigEndian>().unwrap(),
        (&data[0xac..]).read_f32::<BigEndian>().unwrap(),

        (&data[0xb0..]).read_f32::<BigEndian>().unwrap(),
        (&data[0xb4..]).read_f32::<BigEndian>().unwrap(),
        (&data[0xb8..]).read_f32::<BigEndian>().unwrap(),
        (&data[0xbc..]).read_f32::<BigEndian>().unwrap(),

        (&data[0xc0..]).read_f32::<BigEndian>().unwrap(),
        (&data[0xc4..]).read_f32::<BigEndian>().unwrap(),
        (&data[0xc8..]).read_f32::<BigEndian>().unwrap(),
        (&data[0xcc..]).read_f32::<BigEndian>().unwrap(),

        0.0,
        0.0,
        0.0,
        1.0,
    );

    let name = String::from(util::parse_str(&data[string_offset as usize..]).unwrap());

    let flags = BoneFlags::from_bits(flags_int).unwrap();
    let billboard = match billboard_int {
        0 => BoneBillboard::Off,
        1 => BoneBillboard::Standard,
        2 => BoneBillboard::StandardPerspective,
        3 => BoneBillboard::Rotation,
        4 => BoneBillboard::RotationPerspective,
        5 => BoneBillboard::Y,
        6 => BoneBillboard::YPerspective,
        _ => panic!("invalid bone billboard"),
    };

    let children = if first_child_offset == 0 {
        vec!()
    } else {
        bone_siblings(&data[first_child_offset as usize ..])
    };

    let mut siblings = if next_offset == 0 {
        vec!()
    } else {
        bone_siblings(&data[next_offset as usize ..])
    };

    siblings.push(Bone {
        name,
        header_len,
        mdl0_offset,
        string_offset,
        index,
        node_id,
        flags,
        billboard,
        bb_index,
        scale,
        rot,
        trans,
        extents,
        user_data_offset,
        transform,
        transform_inv,
        children,
    });
    siblings
}

#[derive(Debug)]
pub struct Bone {
    pub name: String,
    header_len: i32,
    mdl0_offset: i32,
    string_offset: i32,
    pub index: i32,
    pub node_id: i32,
    pub flags: BoneFlags,
    pub billboard: BoneBillboard,
    bb_index: u32,
    pub scale: Vector3<f32>,
    pub rot: Vector3<f32>,
    pub trans: Vector3<f32>,
    pub extents: MBox,
    user_data_offset: i32,
    pub transform: Matrix4<f32>,
    pub transform_inv: Matrix4<f32>,
    pub children: Vec<Bone>,
}

bitflags! {
    pub struct BoneFlags: u32 {
        const NO_TRANSFORM          = 0x1;
        const FIXED_TRANSLATION     = 0x2;
        const FIXED_ROTATION        = 0x4;
        const FIXED_SCALE           = 0x8;
        const SCALE_EQUAL           = 0x10;
        const SEG_SCALE_COMP_APPLY  = 0x20;
        const SEG_SCALE_COMP_PARENT = 0x40;
        const CLASSIC_SCALE_OFF     = 0x80;
        const VISIBLE               = 0x100;
        const HAS_GEOMETRY          = 0x200;
        const HAS_BILLBOARD_PARENT  = 0x400;
    }
}

#[derive(Debug)]
pub enum BoneBillboard {
    Off,
    Standard,
    StandardPerspective,
    Rotation,
    RotationPerspective,
    Y,
    YPerspective,
}
