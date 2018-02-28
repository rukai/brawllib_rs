use byteorder::{BigEndian, ReadBytesExt};
use cgmath::{Vector3, Matrix4};

use resources::Resource;
use mbox::MBox;
use mbox;

pub(crate) fn bones(data: &[u8], resources: Vec<Resource>) -> Bones {
    let mut bones = vec!();
    for resource in resources {
        let resource_data = &data[resource.data_offset as usize ..];
        let header_len    = (&resource_data[0x00..]).read_i32::<BigEndian>().unwrap();
        let mdl0_offset   = (&resource_data[0x04..]).read_i32::<BigEndian>().unwrap();
        let string_offset = (&resource_data[0x08..]).read_i32::<BigEndian>().unwrap();
        let index         = (&resource_data[0x0c..]).read_i32::<BigEndian>().unwrap();
        let node_id       = (&resource_data[0x10..]).read_i32::<BigEndian>().unwrap();
        let flags_int     = (&resource_data[0x14..]).read_u32::<BigEndian>().unwrap();
        let billboard_int = (&resource_data[0x18..]).read_u32::<BigEndian>().unwrap();
        let bb_index      = (&resource_data[0x1c..]).read_u32::<BigEndian>().unwrap();

        let scale = Vector3::<f32>::new(
            (&resource_data[0x20..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x24..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x28..]).read_f32::<BigEndian>().unwrap(),
        );
        let rot = Vector3::<f32>::new(
            (&resource_data[0x2c..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x30..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x34..]).read_f32::<BigEndian>().unwrap(),
        );
        let trans = Vector3::<f32>::new(
            (&resource_data[0x38..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x3c..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x40..]).read_f32::<BigEndian>().unwrap(),
        );
        let extents  = mbox::mbox(&resource_data[0x44..]);
        let parent_offset      = (&resource_data[0x5c..]).read_i32::<BigEndian>().unwrap();
        let first_child_offset = (&resource_data[0x60..]).read_i32::<BigEndian>().unwrap();
        let next_offset        = (&resource_data[0x64..]).read_i32::<BigEndian>().unwrap();
        let prev_offset        = (&resource_data[0x68..]).read_i32::<BigEndian>().unwrap();
        let user_data_offset   = (&resource_data[0x6c..]).read_i32::<BigEndian>().unwrap();

        // TODO: Took a random guess at how to read these matrices, probably wrong
        let transform = Matrix4::new(
            (&resource_data[0x70..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x74..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x78..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x7c..]).read_f32::<BigEndian>().unwrap(),

            (&resource_data[0x70..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x74..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x78..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x7c..]).read_f32::<BigEndian>().unwrap(),

            (&resource_data[0x80..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x84..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x88..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x8c..]).read_f32::<BigEndian>().unwrap(),

            1.0,
            1.0,
            1.0,
            1.0,
        );

        let transform_inv = Matrix4::new(
            (&resource_data[0x90..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x94..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x98..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0x9c..]).read_f32::<BigEndian>().unwrap(),

            (&resource_data[0xa0..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0xa4..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0xa8..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0xac..]).read_f32::<BigEndian>().unwrap(),

            (&resource_data[0xb0..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0xb4..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0xb8..]).read_f32::<BigEndian>().unwrap(),
            (&resource_data[0xbc..]).read_f32::<BigEndian>().unwrap(),

            1.0,
            1.0,
            1.0,
            1.0,
        );

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

        bones.push(Bone {
            name: resource.string,
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
            parent_offset,
            first_child_offset,
            next_offset,
            prev_offset,
            user_data_offset,
            transform,
            transform_inv,
        });
    }
    Bones { bones }
}

#[derive(Debug)]
pub struct Bones {
    bones: Vec<Bone>,
}

#[derive(Debug)]
pub struct Bone {
    name: String,
    header_len: i32,
    mdl0_offset: i32,
    string_offset: i32,
    index: i32,
    node_id: i32,
    flags: BoneFlags,
    billboard: BoneBillboard,
    bb_index: u32,
    scale: Vector3<f32>,
    rot: Vector3<f32>,
    trans: Vector3<f32>,
    extents: MBox,
    parent_offset: i32,
    first_child_offset: i32,
    next_offset: i32,
    prev_offset: i32,
    user_data_offset: i32,
    transform: Matrix4<f32>,
    transform_inv: Matrix4<f32>
}

bitflags! {
    struct BoneFlags: u32 {
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
