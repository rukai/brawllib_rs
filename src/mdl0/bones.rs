use byteorder::{BigEndian, ReadBytesExt};
use cgmath::{Vector3, Matrix4, SquareMatrix};

use crate::mbox::MBox;
use crate::mbox;
use crate::resources::Resource;
use crate::util;
use crate::math;

pub(crate) fn bones(data: &[u8], resources: Vec<Resource>) -> Bone {
    bone_siblings(&data[resources[0].data_offset as usize ..], 0).pop().unwrap()
}

fn bone_siblings(data_root: &[u8], offset: i32) -> Vec<Bone> {
    let data = &data_root[offset as usize..];

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
    let translate = Vector3::<f32>::new(
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

    let transform0         = (&data[0x70..]).read_f32::<BigEndian>().unwrap();
    let transform1         = (&data[0x74..]).read_f32::<BigEndian>().unwrap();
    let transform2         = (&data[0x78..]).read_f32::<BigEndian>().unwrap();
    let transform3         = (&data[0x7c..]).read_f32::<BigEndian>().unwrap();

    let transform4         = (&data[0x80..]).read_f32::<BigEndian>().unwrap();
    let transform5         = (&data[0x84..]).read_f32::<BigEndian>().unwrap();
    let transform6         = (&data[0x88..]).read_f32::<BigEndian>().unwrap();
    let transform7         = (&data[0x8c..]).read_f32::<BigEndian>().unwrap();

    let transform8         = (&data[0x90..]).read_f32::<BigEndian>().unwrap();
    let transform9         = (&data[0x94..]).read_f32::<BigEndian>().unwrap();
    let transform10        = (&data[0x98..]).read_f32::<BigEndian>().unwrap();
    let transform11        = (&data[0x9c..]).read_f32::<BigEndian>().unwrap();

    let transform_inv0     = (&data[0xa0..]).read_f32::<BigEndian>().unwrap();
    let transform_inv1     = (&data[0xa4..]).read_f32::<BigEndian>().unwrap();
    let transform_inv2     = (&data[0xa8..]).read_f32::<BigEndian>().unwrap();
    let transform_inv3     = (&data[0xac..]).read_f32::<BigEndian>().unwrap();

    let transform_inv4     = (&data[0xb0..]).read_f32::<BigEndian>().unwrap();
    let transform_inv5     = (&data[0xb4..]).read_f32::<BigEndian>().unwrap();
    let transform_inv6     = (&data[0xb8..]).read_f32::<BigEndian>().unwrap();
    let transform_inv7     = (&data[0xbc..]).read_f32::<BigEndian>().unwrap();

    let transform_inv8     = (&data[0xc0..]).read_f32::<BigEndian>().unwrap();
    let transform_inv9     = (&data[0xc4..]).read_f32::<BigEndian>().unwrap();
    let transform_inv10    = (&data[0xc8..]).read_f32::<BigEndian>().unwrap();
    let transform_inv11    = (&data[0xcc..]).read_f32::<BigEndian>().unwrap();

    // BrawlBox uses a Matrix43, this means there are 4 columns and 3 rows we need to read.
    // We only have 4x4 matrices available, so use [0.0, 0.0, 0.0, 1.0] as the last row,
    let transform = Matrix4::new(
        transform0,
        transform4,
        transform8,
        0.0,

        transform1,
        transform5,
        transform9,
        0.0,

        transform2,
        transform6,
        transform10,
        0.0,

        transform3,
        transform7,
        transform11,
        1.0,
    );

    let transform_inv = Matrix4::new(
        transform_inv0,
        transform_inv4,
        transform_inv8,
        0.0,

        transform_inv1,
        transform_inv5,
        transform_inv9,
        0.0,

        transform_inv2,
        transform_inv6,
        transform_inv10,
        0.0,

        transform_inv3,
        transform_inv7,
        transform_inv11,
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
        bone_siblings(&data_root, offset + first_child_offset)
    };

    let mut siblings = if next_offset == 0 {
        vec!()
    } else {
        bone_siblings(&data_root, offset + next_offset)
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
        translate,
        extents,
        user_data_offset,
        transform,
        transform_hitbox: Matrix4::identity(),
        transform_inv,
        children,
    });
    siblings
}

#[derive(Debug, Clone)]
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
    // these values are depdendent on the parent bone
    pub scale: Vector3<f32>,
    pub rot: Vector3<f32>,
    pub translate: Vector3<f32>,
    pub extents: MBox,
    user_data_offset: i32,
    // these matrices are calculated from scale, rot and translate but are independent of the parent bone
    pub transform: Matrix4<f32>,
    /// This is a terrible hack, remove it ASAP
    pub transform_hitbox: Matrix4<f32>,
    pub transform_inv: Matrix4<f32>,
    pub children: Vec<Bone>,
}

impl Bone {
    pub fn gen_transform(&self) -> Matrix4<f32> {
        math::gen_transform(self.scale, self.rot, self.translate)
    }
}

bitflags! {
    pub struct BoneFlags: u32 {
        /// Needs to match what this bones transformation does. Identity matrix.
        const NO_TRANSFORM          = 0x1;
        /// Needs to match what this bones transformation does. Identity translation.
        const FIXED_TRANSLATION     = 0x2;
        /// Needs to match what this bones transformation does. Identity rotation.
        const FIXED_ROTATION        = 0x4;
        /// Needs to match what this bones transformation does. Identity scale.
        const FIXED_SCALE           = 0x8;
        /// Needs to match what this bones transformation does. Scales equally in all dimensions.
        const SCALE_EQUAL           = 0x10;
        const SEG_SCALE_COMP_APPLY  = 0x20;
        const SEG_SCALE_COMP_PARENT = 0x40;
        const CLASSIC_SCALE_OFF     = 0x80;
        const VISIBLE               = 0x100;
        /// Needs to be true if the bone has geometry.
        const HAS_GEOMETRY          = 0x200;
        const HAS_BILLBOARD_PARENT  = 0x400;
    }
}

#[derive(Debug, Clone)]
pub enum BoneBillboard {
    Off,
    Standard,
    StandardPerspective,
    Rotation,
    RotationPerspective,
    Y,
    YPerspective,
}
