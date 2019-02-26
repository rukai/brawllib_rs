pub mod bones;
pub mod palettes;
pub mod textures;
pub mod vertices;
pub mod objects;
pub mod definitions;

use byteorder::{BigEndian, ReadBytesExt};

use crate::resources::Resource;
use crate::resources;
use crate::mbox::MBox;
use crate::mbox;
use crate::util;
use palettes::PaletteRef;
use textures::TextureRef;
use vertices::Vertices;
use bones::Bone;
use objects::Object;
use definitions::Definition;

pub(crate) fn mdl0(data: &[u8]) -> Mdl0 {
    let _size        = (&data[0x4..]).read_i32::<BigEndian>().unwrap();
    let version      = (&data[0x8..]).read_i32::<BigEndian>().unwrap();
    let _bres_offset = (&data[0xc..]).read_i32::<BigEndian>().unwrap();

    let string_offset_offset = match version {
        0xA => 0x44,
        0xB => 0x48,
        _   => 0x3C
    };
    let string_offset = (&data[string_offset_offset..]).read_i32::<BigEndian>().unwrap();
    let name = util::parse_str(&data[string_offset as usize .. ]).unwrap().to_string();

    //let data_offset = match version {
    //    0xA => 0x40,
    //    0xB => 0x44,
    //    _   => 0 // no data
    //};

    let props_offset = match version {
        0x08 => 0x40,
        0x09 => 0x40,
        0x0A => 0x48,
        0x0B => 0x4C,
        _    => 0 // no data
    };

    let props = if props_offset == 0 {
        None
    } else {
        Some(Mdl0Props {
            header_len:         (&data[props_offset + 0x00 ..]).read_u32::<BigEndian>().unwrap(),
            mdl0offset:         (&data[props_offset + 0x04 ..]).read_i32::<BigEndian>().unwrap(),
            scaling_rule:       (&data[props_offset + 0x08 ..]).read_i32::<BigEndian>().unwrap(),
            tex_matrix_mode:    (&data[props_offset + 0x0c ..]).read_i32::<BigEndian>().unwrap(),
            num_vertices:       (&data[props_offset + 0x10 ..]).read_i32::<BigEndian>().unwrap(),
            num_triangles:      (&data[props_offset + 0x14 ..]).read_i32::<BigEndian>().unwrap(),
            orig_path_offset:   (&data[props_offset + 0x18 ..]).read_i32::<BigEndian>().unwrap(),
            num_nodes:          (&data[props_offset + 0x1c ..]).read_i32::<BigEndian>().unwrap(),
            need_nrm_mtx_array: (&data[props_offset + 0x20 ..]).read_u8().unwrap(),
            need_tex_mtx_array: (&data[props_offset + 0x21 ..]).read_u8().unwrap(),
            enable_extents:     (&data[props_offset + 0x22 ..]).read_u8().unwrap(),
            env_mtx_mode:       (&data[props_offset + 0x23 ..]).read_u8().unwrap(),
            data_offset:        (&data[props_offset + 0x24 ..]).read_i32::<BigEndian>().unwrap(),
            extents:  mbox::mbox(&data[props_offset + 0x28 ..]),
        })
    };

    let mut definitions = None;
    let mut bones = None;
    let mut vertices = None;
    let mut normals = None;
    let mut colors = None;
    let mut uv = None;
    let mut fur_vectors = None;
    let mut fur_layer_coords = None;
    let mut materials = None;
    let mut shaders = None;
    let mut objects = None;
    let mut texture_refs = None;
    let mut palette_refs = None;

    let fur_version = version >= 10;
    let num_children = if fur_version { 13 } else { 11 };
    for i in 0..num_children {
        let offset = 0x10 + i * 0x4;

        let resources_offset = (&data[offset..]).read_i32::<BigEndian>().unwrap();
        if resources_offset != 0 {
            let resources = resources::resources(&data[resources_offset as usize .. ]);
            match i {
                6  if fur_version => { fur_vectors = Some(resources) }
                7  if fur_version => { fur_layer_coords = Some(resources) }
                8  if fur_version => { materials = Some(resources) }
                9  if fur_version => { shaders = Some(resources) }
                10 if fur_version => { objects = Some(objects::objects(&data[resources_offset as usize ..], resources)) }
                11 if fur_version => { texture_refs = Some(textures::textures(&data[resources_offset as usize ..], resources)) }
                12 if fur_version => { palette_refs = Some(palettes::palettes(&data[resources_offset as usize ..], resources)) }
                0 => { definitions = Some(definitions::definitions(&data[resources_offset as usize..], resources)) }
                1 => { bones = Some(bones::bones(&data[resources_offset as usize ..], resources)) }
                2 => { vertices = Some(vertices::vertices(&data[resources_offset as usize ..], resources)) }
                3 => { normals = Some(resources) }
                4 => { colors = Some(resources) }
                5 => { uv = Some(resources) }
                6 => { materials = Some(resources) }
                7 => { shaders = Some(resources) }
                8 => { objects = Some(objects::objects(&data[resources_offset as usize ..], resources)) }
                9 => { texture_refs = Some(textures::textures(&data[resources_offset as usize ..], resources)) }
                10 => { palette_refs = Some(palettes::palettes(&data[resources_offset as usize ..], resources)) }
                _ => { unreachable!() }
            }
        }
    }

    Mdl0 {
        name,
        version,
        props,
        definitions,
        bones,
        vertices,
        normals,
        colors,
        uv,
        fur_vectors,
        fur_layer_coords,
        materials,
        shaders,
        objects,
        texture_refs,
        palette_refs,
    }
}

#[derive(Debug)]
pub struct Mdl0 {
    pub name: String,
    version: i32,
    pub props: Option<Mdl0Props>,
    pub definitions: Option<Vec<Definition>>,
    pub bones: Option<Bone>,
    pub vertices: Option<Vec<Vertices>>,
    normals: Option<Vec<Resource>>,
    colors: Option<Vec<Resource>>,
    uv: Option<Vec<Resource>>,
    fur_vectors: Option<Vec<Resource>>,
    fur_layer_coords: Option<Vec<Resource>>,
    materials: Option<Vec<Resource>>,
    shaders: Option<Vec<Resource>>,
    pub objects: Option<Vec<Object>>,
    pub texture_refs: Option<Vec<Vec<TextureRef>>>,
    pub palette_refs: Option<Vec<Vec<PaletteRef>>>,
}

#[derive(Debug)]
pub struct Mdl0Props {
    header_len: u32,
    mdl0offset: i32,
    scaling_rule: i32,
    tex_matrix_mode: i32,
    num_vertices: i32,
    num_triangles: i32,
    orig_path_offset: i32,
    num_nodes: i32,
    need_nrm_mtx_array: u8,
    need_tex_mtx_array: u8,
    enable_extents: u8,
    env_mtx_mode: u8,
    data_offset: i32,
    extents: MBox,
}
