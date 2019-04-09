use cgmath::Vector3;
use fancy_slice::FancySlice;

use crate::util;

pub fn misc_section(data: FancySlice, parent_data: FancySlice) -> MiscSection {
    let unk0_offset           = data.i32_be(0);
    let final_smash_aura_list = util::list_offset(data.relative_fancy_slice(0x04..));
    let hurt_box_list         = util::list_offset(data.relative_fancy_slice(0x0c..));
    let ledge_grab_list       = util::list_offset(data.relative_fancy_slice(0x14..));
    let unk7_list             = util::list_offset(data.relative_fancy_slice(0x1c..));
    let bone_refs_offset      = data.i32_be(0x24);
    let unk10_offset          = data.i32_be(0x28);
    let sound_data_offset     = data.i32_be(0x2c);
    let unk12_offset          = data.i32_be(0x30);
    let multi_jump_offset     = data.i32_be(0x34);
    let glide_offset          = data.i32_be(0x38);
    let crawl_offset          = data.i32_be(0x3c);
    let collision_data_offset = data.i32_be(0x40);
    let tether_offset         = data.i32_be(0x44);
    let unk18_offset          = data.i32_be(0x48);

    // TODO
    // Located at 0x1E50c in brawl FitPokeZeniGame.pac, there is supposed to be a struct something like this:
    // struct ECBData {
    //    bone_count: i32 // 0x00000006
    //    min_width:  f32 // 0x40800000
    //    min_height: f32 // 0x40800000
    //}
    // However I have no idea how to find that so I'm just going to calculate the length and hardcode the min_width + min_height

    let ecb_offset = unk0_offset as usize + 0x20;
    let mut ecb_bones = vec!();
    if hurt_box_list.start_offset != 0 {
        // TODO: leaving these prints here cause I plan on fixing this next
        //println!("{}", crate::util::hex_dump(&parent_data[unk0_offset as usize ..unk0_offset as usize + 0x20]));
        //println!("{}", crate::util::hex_dump(&parent_data[ecb_offset as usize ..ecb_offset as usize + 0x20]));
        //println!("hurtbox: {:x}", hurt_box_list.start_offset);
        //println!("ecb: {:x}", ecb_offset);
        if hurt_box_list.start_offset as usize > ecb_offset {
            let ecb_total = (hurt_box_list.start_offset as usize - ecb_offset) / 4; // TODO: this underflows
            //println!("ecb_total: {}", ecb_total);
            if ecb_total < 100 {
                for i in 0..ecb_total {
                    ecb_bones.push(parent_data.i32_be(ecb_offset + i * 4));
                }
            }
        }
        else {
            error!("There should always be hurtbox data, hitting this is demonstrating this hack does not always work")
        }
        //println!("ecb_bones: {:?}", ecb_bones);
    }

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

    let mut ledge_grabs = vec!();
    for i in 0..ledge_grab_list.count {
        let offset = ledge_grab_list.start_offset as usize + i as usize * LEDGE_GRAB_SIZE;
        ledge_grabs.push(ledge_grab(parent_data.relative_fancy_slice(offset ..)));
    }

    let mut unk7s = vec!();
    for i in 0..unk7_list.count {
        let offset = unk7_list.start_offset as usize + i as usize * UNK7_SIZE;
        unk7s.push(unk7(parent_data.relative_fancy_slice(offset ..)));
    }

    let mut bone_refs = vec!();
    for i in 0..10 {
        let offset = bone_refs_offset as usize + i as usize * 4;
        bone_refs.push(parent_data.i32_be(offset));
    }

    let crawl = if crawl_offset == 0 {
        None
    } else {
        Some(Crawl {
            forward:  parent_data.f32_be(crawl_offset as usize),
            backward: parent_data.f32_be(crawl_offset as usize + 0x4),
        })
    };

    let tether = if tether_offset == 0 {
        None
    } else {
        Some(Tether {
            num_hang_frame: parent_data.i32_be(tether_offset as usize),
            unk1:           parent_data.f32_be(tether_offset as usize + 0x4),
        })
    };

    MiscSection {
        ecb_bones,
        final_smash_auras,
        hurt_boxes,
        ledge_grabs,
        unk7s,
        bone_refs,
        unk10_offset,
        sound_data_offset,
        unk12_offset,
        multi_jump_offset,
        glide_offset,
        crawl,
        collision_data_offset,
        tether,
        unk18_offset,
    }
}

fn final_smash_aura(data: FancySlice) -> FinalSmashAura {
    let bone_index = data.i32_be(0x00);
    let x          = data.f32_be(0x04);
    let y          = data.f32_be(0x08);
    let width      = data.f32_be(0x0c);
    let height     = data.f32_be(0x10);
    FinalSmashAura { bone_index, x, y, width, height }
}

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
    let bone_index = ((flags & 0b1111_1111_1000_0000) >> 7) as u16;

    let zone = match zone {
        0 => HurtBoxZone::Low,
        1 => HurtBoxZone::Middle,
        2 => HurtBoxZone::High,
        _ => unreachable!()
    };

    HurtBox {
        offset,
        stretch,
        radius,
        enabled,
        zone,
        region,
        bone_index,
    }
}

fn ledge_grab(data: FancySlice) -> LedgeGrab {
    let x      = data.f32_be(0x0);
    let y      = data.f32_be(0x4);
    let width  = data.f32_be(0x8);
    let height = data.f32_be(0xc);
    LedgeGrab { x, y, width, height }
}

fn unk7(data: FancySlice) -> Unk7 {
    let unk1  = data.u8(0x00);
    let unk2  = data.u8(0x01);
    let unk3  = data.u8(0x02);
    let unk4  = data.u8(0x03);
    let unk5  = data.u8(0x04);
    let unk6  = data.u8(0x05);
    let unk7  = data.u8(0x06);
    let unk8  = data.u8(0x07);
    let unk9  = data.f32_be(0x08);
    let unk10 = data.f32_be(0x0c);
    let unk11 = data.f32_be(0x10);
    let unk12 = data.f32_be(0x14);
    let unk13 = data.f32_be(0x18);
    let unk14 = data.f32_be(0x1c);
    Unk7 { unk1, unk2, unk3, unk4, unk5, unk6, unk7, unk8, unk9, unk10, unk11, unk12, unk13, unk14 }
}

#[derive(Clone, Debug)]
pub struct MiscSection {
    pub ecb_bones: Vec<i32>,
    pub final_smash_auras: Vec<FinalSmashAura>,
    pub hurt_boxes: Vec<HurtBox>,
    pub ledge_grabs: Vec<LedgeGrab>,
    pub unk7s: Vec<Unk7>,
    pub bone_refs: Vec<i32>,
    unk10_offset: i32,
    sound_data_offset: i32,
    unk12_offset: i32,
    multi_jump_offset: i32,
    glide_offset: i32,
    pub crawl: Option<Crawl>,
    collision_data_offset: i32,
    pub tether: Option<Tether>,
    unk18_offset: i32,
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
#[derive(Serialize, Clone, Debug)]
pub struct HurtBox {
    pub offset: Vector3<f32>,
    pub stretch: Vector3<f32>,
    pub radius: f32,
    pub enabled: bool,
    pub zone: HurtBoxZone,
    pub region: u8,
    pub bone_index: u16,
}

#[derive(Serialize, Clone, Debug)]
pub enum HurtBoxZone {
    Low,
    Middle,
    High
}

pub const LEDGE_GRAB_SIZE: usize = 0x10;
#[derive(Serialize, Clone, Debug)]
pub struct LedgeGrab {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub const UNK7_SIZE: usize = 0x20;
#[derive(Clone, Debug)]
pub struct Unk7 {
    unk1: u8,
    unk2: u8,
    unk3: u8,
    unk4: u8,

    unk5: u8,
    unk6: u8,
    unk7: u8,
    unk8: u8,

    unk9: f32,
    unk10: f32,
    unk11: f32,
    unk12: f32,

    unk13: f32,
    unk14: f32,
}

#[derive(Clone, Debug)]
pub struct Crawl {
    pub forward: f32,
    pub backward: f32,
}

#[derive(Clone, Debug)]
pub struct Tether {
    pub num_hang_frame: i32,
    unk1: f32,
}
