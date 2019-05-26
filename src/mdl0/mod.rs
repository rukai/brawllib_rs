pub mod bones;
pub mod palettes;
pub mod textures;
pub mod vertices;
pub mod objects;
pub mod definitions;

use fancy_slice::FancySlice;

use crate::resources::Resource;
use crate::resources;
use crate::mbox::MBox;
use crate::mbox;
use palettes::Palette;
use textures::Texture;
use vertices::Vertices;
use bones::Bone;
use objects::Object;
use definitions::Definition;

pub(crate) fn mdl0(data: FancySlice) -> Mdl0 {
    let _size        = data.i32_be(0x4);
    let version      = data.i32_be(0x8);
    let _bres_offset = data.i32_be(0xc);

    let string_offset_offset = match version {
        0xA => 0x44,
        0xB => 0x48,
        _   => 0x3C
    };
    let string_offset = data.i32_be(string_offset_offset);
    let name = data.str(string_offset as usize).unwrap().to_string();

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
            header_len:         data.u32_be(props_offset + 0x00),
            mdl0offset:         data.i32_be(props_offset + 0x04),
            scaling_rule:       data.i32_be(props_offset + 0x08),
            tex_matrix_mode:    data.i32_be(props_offset + 0x0c),
            num_vertices:       data.i32_be(props_offset + 0x10),
            num_triangles:      data.i32_be(props_offset + 0x14),
            orig_path_offset:   data.i32_be(props_offset + 0x18),
            num_nodes:          data.i32_be(props_offset + 0x1c),
            need_nrm_mtx_array: data.u8    (props_offset + 0x20),
            need_tex_mtx_array: data.u8    (props_offset + 0x21),
            enable_extents:     data.u8    (props_offset + 0x22),
            env_mtx_mode:       data.u8    (props_offset + 0x23),
            data_offset:        data.i32_be(props_offset + 0x24),
            extents: mbox::mbox(data.relative_fancy_slice(props_offset + 0x28..)),
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
    let mut texture_refs = None; // TODO: Bleh I think the naming of this and children is wrong
    let mut palette_refs = None;

    let fur_version = version >= 10;
    let num_children = if fur_version { 13 } else { 11 };
    for i in 0..num_children {
        let offset = 0x10 + i * 0x4;

        let resources_offset = data.i32_be(offset);
        if resources_offset != 0 {
            let resources = resources::resources(data.relative_fancy_slice(resources_offset as usize .. ));
            match i {
                6  if fur_version => { fur_vectors = Some(resources) }
                7  if fur_version => { fur_layer_coords = Some(resources) }
                8  if fur_version => { materials = Some(resources) }
                9  if fur_version => { shaders = Some(resources) }
                10 if fur_version => { objects = Some(objects::objects(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                11 if fur_version => { texture_refs = Some(textures::textures(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                12 if fur_version => { palette_refs = Some(palettes::palettes(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                0 => { definitions = Some(definitions::definitions(data.relative_fancy_slice(resources_offset as usize..), resources)) }
                1 => { bones = Some(bones::bones(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                2 => { vertices = Some(vertices::vertices(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                3 => { normals = Some(resources) }
                4 => { colors = Some(resources) }
                5 => { uv = Some(resources) }
                6 => { materials = Some(resources) }
                7 => { shaders = Some(resources) }
                8 => { objects = Some(objects::objects(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                9 => { texture_refs = Some(textures::textures(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                10 => { palette_refs = Some(palettes::palettes(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
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

#[derive(Clone, Debug)]
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
    pub texture_refs: Option<Vec<Texture>>,
    pub palette_refs: Option<Vec<Palette>>,
}

impl Mdl0 {
    pub fn compile(&self, bres_offset: i32) -> Vec<u8> {
        let mut output = vec!();

        // create mdl0 header
        output.extend("MDL0".chars().map(|x| x as u8));
        output.extend(&i32::to_be_bytes(0x512e)); // size
        output.extend(&i32::to_be_bytes(self.version));
        output.extend(&i32::to_be_bytes(bres_offset));

        match self.version {
            0xA => { }
            0xB => { }
            _   => { }
        }

        output
    }
}

#[derive(Clone, Debug)]
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
