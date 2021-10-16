use cgmath::{Matrix4, Vector3};
use fancy_slice::FancySlice;

use crate::math;
use crate::mbox;
use crate::mbox::MBox;
use crate::resources::Resource;

pub(crate) fn bones(data: FancySlice, resources: Vec<Resource>) -> Bone {
    bone_siblings(
        data.relative_fancy_slice(resources[0].data_offset as usize..),
        0,
    )
    .pop()
    .unwrap()
}

#[rustfmt::skip]
fn bone_siblings(data_root: FancySlice, offset: i32) -> Vec<Bone> {
    let data = data_root.relative_fancy_slice(offset as usize..);

    let header_len    = data.i32_be(0x00);
    let mdl0_offset   = data.i32_be(0x04);
    let string_offset = data.i32_be(0x08);
    let index         = data.i32_be(0x0c);
    let node_id       = data.i32_be(0x10);
    let flags_int     = data.u32_be(0x14);
    let billboard_int = data.u32_be(0x18);
    let bb_index      = data.u32_be(0x1c);

    let scale = Vector3::<f32>::new(
        data.f32_be(0x20),
        data.f32_be(0x24),
        data.f32_be(0x28),
    );
    let rot = Vector3::<f32>::new(
        data.f32_be(0x2c),
        data.f32_be(0x30),
        data.f32_be(0x34),
    );
    let translate = Vector3::<f32>::new(
        data.f32_be(0x38),
        data.f32_be(0x3c),
        data.f32_be(0x40),
    );

    let extents  = mbox::mbox(data.relative_fancy_slice(0x44..));
    let _parent_offset     = data.i32_be(0x5c);
    let first_child_offset = data.i32_be(0x60);
    let next_offset        = data.i32_be(0x64);
    let _prev_offset       = data.i32_be(0x68);
    let user_data_offset   = data.i32_be(0x6c);

    let transform0         = data.f32_be(0x70);
    let transform1         = data.f32_be(0x74);
    let transform2         = data.f32_be(0x78);
    let transform3         = data.f32_be(0x7c);

    let transform4         = data.f32_be(0x80);
    let transform5         = data.f32_be(0x84);
    let transform6         = data.f32_be(0x88);
    let transform7         = data.f32_be(0x8c);

    let transform8         = data.f32_be(0x90);
    let transform9         = data.f32_be(0x94);
    let transform10        = data.f32_be(0x98);
    let transform11        = data.f32_be(0x9c);

    let transform_inv0     = data.f32_be(0xa0);
    let transform_inv1     = data.f32_be(0xa4);
    let transform_inv2     = data.f32_be(0xa8);
    let transform_inv3     = data.f32_be(0xac);

    let transform_inv4     = data.f32_be(0xb0);
    let transform_inv5     = data.f32_be(0xb4);
    let transform_inv6     = data.f32_be(0xb8);
    let transform_inv7     = data.f32_be(0xbc);

    let transform_inv8     = data.f32_be(0xc0);
    let transform_inv9     = data.f32_be(0xc4);
    let transform_inv10    = data.f32_be(0xc8);
    let transform_inv11    = data.f32_be(0xcc);

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

    let name = data.str(string_offset as usize).unwrap().to_string();

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
        bone_siblings(data_root, offset + first_child_offset)
    };

    let mut siblings = if next_offset == 0 {
        vec!()
    } else {
        bone_siblings(data_root, offset + next_offset)
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
    pub transform_inv: Matrix4<f32>,
    pub children: Vec<Bone>,
}

impl Bone {
    pub fn gen_transform(&self) -> Matrix4<f32> {
        math::gen_transform(self.scale, self.rot, self.translate)
    }

    pub(crate) fn gen_transform_rot_only(&self) -> Matrix4<f32> {
        math::gen_transform(
            Vector3::new(1.0, 1.0, 1.0),
            self.rot,
            Vector3::new(0.0, 0.0, 0.0),
        )
    }
}

bitflags! {
    #[rustfmt::skip]
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
