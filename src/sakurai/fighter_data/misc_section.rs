use cgmath::Vector3;
use fancy_slice::FancySlice;

use crate::util;

#[rustfmt::skip]
pub fn misc_section(data: FancySlice, parent_data: FancySlice) -> MiscSection {
    let _unk0_offset          = data.i32_be(0);
    let final_smash_aura_list = util::list_offset(data.relative_fancy_slice(0x04..));
    let hurt_box_list         = util::list_offset(data.relative_fancy_slice(0x0c..));
    let ledge_grab_list       = util::list_offset(data.relative_fancy_slice(0x14..));
    let unk7_list             = util::list_offset(data.relative_fancy_slice(0x1c..));
    let bone_refs_offset      = data.i32_be(0x24);
    let _item_bones           = data.i32_be(0x28);
    let _sound_data_offset    = data.i32_be(0x2c);
    let _unk12_offset         = data.i32_be(0x30);
    let _multi_jump_offset    = data.i32_be(0x34);
    let _glide_offset         = data.i32_be(0x38);
    let crawl_offset          = data.i32_be(0x3c);
    let ecbs_offset           = data.i32_be(0x40);
    let tether_offset         = data.i32_be(0x44);
    let _unk18_offset         = data.i32_be(0x48);

    let mut final_smash_auras = vec!();
    for i in 0..final_smash_aura_list.count {
        let offset = final_smash_aura_list.start_offset as usize + i as usize * FINAL_SMASH_AURA_SIZE;
        final_smash_auras.push(final_smash_aura(parent_data.relative_fancy_slice(offset..)));
    }

    let mut hurt_boxes = vec!();
    for i in 0..hurt_box_list.count {
        let offset = hurt_box_list.start_offset as usize + i as usize * HURTBOX_SIZE;
        hurt_boxes.push(hurtbox(parent_data.relative_fancy_slice(offset..)));
    }

    let mut ledge_grab_boxes = vec!();
    for i in 0..ledge_grab_list.count {
        let offset = ledge_grab_list.start_offset as usize + i as usize * LEDGE_GRAB_SIZE;
        ledge_grab_boxes.push(ledge_grab_box(parent_data.relative_fancy_slice(offset ..)));
    }

    let mut unk7s = vec!();
    for i in 0..unk7_list.count {
        let offset = unk7_list.start_offset as usize + i as usize * UNK7_SIZE;
        unk7s.push(unk7(parent_data.relative_fancy_slice(offset ..)));
    }

    let bone_refs = BoneRefs {
        unk0:    parent_data.i32_be(bone_refs_offset as usize),
        unk1:    parent_data.i32_be(bone_refs_offset as usize + 0x04),
        unk2:    parent_data.i32_be(bone_refs_offset as usize + 0x08),
        unk3:    parent_data.i32_be(bone_refs_offset as usize + 0x0c),
        trans_n: parent_data.i32_be(bone_refs_offset as usize + 0x10),
        unk5:    parent_data.i32_be(bone_refs_offset as usize + 0x14),
        unk6:    parent_data.i32_be(bone_refs_offset as usize + 0x18),
        unk7:    parent_data.i32_be(bone_refs_offset as usize + 0x1c),
        unk8:    parent_data.i32_be(bone_refs_offset as usize + 0x20),
        unk9:    parent_data.i32_be(bone_refs_offset as usize + 0x24),
    };

    let crawl = if crawl_offset == 0 {
        None
    } else {
        Some(Crawl {
            forward:  parent_data.f32_be(crawl_offset as usize),
            backward: parent_data.f32_be(crawl_offset as usize + 0x4),
        })
    };

    let ecbs_list = util::list_offset(parent_data.relative_fancy_slice(ecbs_offset as usize..));

    // it looks like this same structure is used elsewhere as well. Check the DataSection.cs and ExtraDataOffsets.cs files in brawlbox.
    let mut ecbs = vec!();
    for i in 0..ecbs_list.count {
        let pointer  = parent_data.i32_be(ecbs_list.start_offset as usize + i as usize * ECB_SIZE); // TODO: Is this indirection for anything? Maybe the list is supposed to occur here instead?
        let ecb_type = parent_data.i32_be(pointer as usize);
        if ecb_type == 0 {
            let ecb_bones_offset = parent_data.i32_be(pointer as usize + 0x04);
            let ecb_bones_count  = parent_data.i32_be(pointer as usize + 0x08);
            let min_height       = parent_data.f32_be(pointer as usize + 0x0C);
            let min_width        = parent_data.f32_be(pointer as usize + 0x10);
            let unk              = parent_data.f32_be(pointer as usize + 0x14);

            let mut bones = vec!();
            for i in 0..ecb_bones_count {
                bones.push(parent_data.i32_be(ecb_bones_offset as usize + i as usize * 4));
            }

            ecbs.push(ECB { bones, min_height, min_width, unk });
        } else {
            error!("ECB type unimplemented")
        }
    }

    let tether = if tether_offset == 0 {
        None
    } else {
        Some(Tether {
            num_hang_frame: parent_data.i32_be(tether_offset as usize),
            _unk1:          parent_data.f32_be(tether_offset as usize + 0x4),
        })
    };

    MiscSection {
        final_smash_auras,
        hurt_boxes,
        ledge_grab_boxes,
        unk7s,
        bone_refs,
        _item_bones,
        _sound_data_offset,
        _unk12_offset,
        _multi_jump_offset,
        _glide_offset,
        crawl,
        ecbs,
        tether,
        _unk18_offset,
    }
}

#[rustfmt::skip]
fn final_smash_aura(data: FancySlice) -> FinalSmashAura {
    let bone_index = data.i32_be(0x00);
    let x          = data.f32_be(0x04);
    let y          = data.f32_be(0x08);
    let width      = data.f32_be(0x0c);
    let height     = data.f32_be(0x10);
    FinalSmashAura { bone_index, x, y, width, height }
}

#[rustfmt::skip]
fn hurtbox(data: FancySlice) -> HurtBox {
    let offset = Vector3::<f32>::new(
        data.f32_be(0x0),
        data.f32_be(0x4),
        data.f32_be(0x8),
    );

    let stretch = Vector3::<f32>::new(
        data.f32_be(0x0c),
        data.f32_be(0x10),
        data.f32_be(0x14),
    );

    let radius = data.f32_be(0x18);
    let flags  = data.u16_be(0x1c);

    let enabled    =   flags & 0b0000_0000_0000_0001 == 1;
  //let padding    = ((flags & 0b0000_0000_0000_0110) >> 1) as u8; // Always 0
    let zone       = ((flags & 0b0000_0000_0001_1000) >> 3) as u8;
    let region     = ((flags & 0b0000_0000_0110_0000) >> 5) as u8;
    let bone_index = (flags & 0b1111_1111_1000_0000) >> 7;

    let grabbable = region == 0 || region == 3;
    let trap_item_hittable = region == 2 || region == 3;

    let zone = match zone {
        0 => HurtBoxZone::Low,
        1 => HurtBoxZone::Middle,
        2 => HurtBoxZone::High,
        x => HurtBoxZone::Unknown(x),
    };

    HurtBox {
        offset,
        stretch,
        radius,
        enabled,
        zone,
        grabbable,
        trap_item_hittable,
        bone_index,
    }
}

#[rustfmt::skip]
fn ledge_grab_box(data: FancySlice) -> LedgeGrabBox {
    let x_left     = data.f32_be(0x0);
    let y          = data.f32_be(0x4);
    let x_padding  = data.f32_be(0x8);
    let height     = data.f32_be(0xc);
    LedgeGrabBox { x_left, y, x_padding, height }
}

#[rustfmt::skip]
fn unk7(data: FancySlice) -> Unk7 {
    let _unk1  = data.u8(0x00);
    let _unk2  = data.u8(0x01);
    let _unk3  = data.u8(0x02);
    let _unk4  = data.u8(0x03);
    let _unk5  = data.u8(0x04);
    let _unk6  = data.u8(0x05);
    let _unk7  = data.u8(0x06);
    let _unk8  = data.u8(0x07);
    let _unk9  = data.f32_be(0x08);
    let _unk10 = data.f32_be(0x0c);
    let _unk11 = data.f32_be(0x10);
    let _unk12 = data.f32_be(0x14);
    let _unk13 = data.f32_be(0x18);
    let _unk14 = data.f32_be(0x1c);
    Unk7 { _unk1, _unk2, _unk3, _unk4, _unk5, _unk6, _unk7, _unk8, _unk9, _unk10, _unk11, _unk12, _unk13, _unk14 }
}

#[derive(Clone, Debug)]
pub struct MiscSection {
    pub final_smash_auras: Vec<FinalSmashAura>,
    pub hurt_boxes: Vec<HurtBox>,
    pub ledge_grab_boxes: Vec<LedgeGrabBox>,
    pub unk7s: Vec<Unk7>,
    pub bone_refs: BoneRefs,
    _item_bones: i32,
    _sound_data_offset: i32,
    _unk12_offset: i32,
    _multi_jump_offset: i32,
    _glide_offset: i32,
    pub crawl: Option<Crawl>,
    pub ecbs: Vec<ECB>,
    pub tether: Option<Tether>,
    _unk18_offset: i32,
}

pub const FINAL_SMASH_AURA_SIZE: usize = 0x14;
#[derive(Clone, Debug)]
pub struct FinalSmashAura {
    pub bone_index: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub const HURTBOX_SIZE: usize = 0x20;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HurtBox {
    pub offset: Vector3<f32>,
    pub stretch: Vector3<f32>,
    pub radius: f32,
    pub enabled: bool,
    pub zone: HurtBoxZone,
    pub grabbable: bool,
    pub trap_item_hittable: bool,
    pub bone_index: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HurtBoxZone {
    Low,
    Middle,
    High,
    Unknown(u8),
}

/// The up most y value of the box = y + height
/// The down most y value of the box = y
///
/// When `LedgeGrabEnable::EnableInFront`:
/// *   The fighter can only grab ledge when facing towards it
/// *   The left x value of the box = x_left
/// *   The right x value of the box = ecb right x value + x_padding
///
/// When `LedgeGrabEnable::EnableInFrontAndBehind`:
/// *   The fighter can grab ledge no matter the direction they are facing
/// *   The left x value of the box = ecb left x value - x_padding
/// *   The right x value of the box = ecb right x value + x_padding
///
/// Note: left is behind the fighter and right is in front of the fighter
pub const LEDGE_GRAB_SIZE: usize = 0x10;
#[derive(Serialize, Clone, Debug)]
pub struct LedgeGrabBox {
    pub x_left: f32,
    pub y: f32,
    pub x_padding: f32,
    pub height: f32,
}

pub const UNK7_SIZE: usize = 0x20;
#[derive(Clone, Debug)]
pub struct Unk7 {
    _unk1: u8,
    _unk2: u8,
    _unk3: u8,
    _unk4: u8,

    _unk5: u8,
    _unk6: u8,
    _unk7: u8,
    _unk8: u8,

    _unk9: f32,
    _unk10: f32,
    _unk11: f32,
    _unk12: f32,

    _unk13: f32,
    _unk14: f32,
}

#[derive(Clone, Debug)]
pub struct BoneRefs {
    pub unk0: i32,
    pub unk1: i32,
    pub unk2: i32,
    pub unk3: i32,
    pub trans_n: i32,
    pub unk5: i32,
    pub unk6: i32,
    pub unk7: i32,
    pub unk8: i32,
    pub unk9: i32,
}

#[derive(Clone, Debug)]
pub struct Crawl {
    pub forward: f32,
    pub backward: f32,
}

pub const ECB_SIZE: usize = 0x4; // TODO
#[derive(Clone, Debug)]
/// TODO: Currently just ECB type 0, maybe change to enum or maybe change the fields to Options
pub struct ECB {
    pub bones: Vec<i32>,
    pub min_height: f32,
    pub min_width: f32,
    pub unk: f32, // Is this even part of the ecb, might just be padding...? always 0 and changing doesnt seem to do anything
}

#[derive(Clone, Debug)]
pub struct Tether {
    pub num_hang_frame: i32,
    _unk1: f32,
}
