pub mod bones;
pub mod definitions;
pub mod objects;
pub mod palettes;
pub mod textures;
pub mod vertices;

use fancy_slice::FancySlice;

use crate::mbox;
use crate::mbox::MBox;
use crate::resources;
use crate::resources::Resource;
use bones::Bone;
use definitions::Definitions;
use objects::Object;
use palettes::Palette;
use textures::Texture;
use vertices::Vertices;

#[rustfmt::skip]
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
            header_len:         data.u32_be(props_offset),
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
    let mut _normals = None;
    let mut _colors = None;
    let mut _uv = None;
    let mut fur_vectors = None;
    let mut fur_layer_coords = None;
    let mut _materials = None;
    let mut _shaders = None;
    let mut objects = None;
    let mut texture_refs = None; // TODO: Bleh I think the naming of this and children is wrong
    let mut palette_refs = None;

    let fur_version = version >= 0xA;
    let num_children = if fur_version { 0xD } else { 0xB };
    for i in 0..num_children {
        let offset = 0x10 + i * 0x4;

        let resources_offset = data.i32_be(offset);
        if resources_offset != 0 {
            let resources = resources::resources(data.relative_fancy_slice(resources_offset as usize .. ));
            match i {
                0x6 if fur_version => { fur_vectors = Some(resources) }
                0x7 if fur_version => { fur_layer_coords = Some(resources) }
                0x8 if fur_version => { _materials = Some(resources) }
                0x9 if fur_version => { _shaders = Some(resources) }
                0xA if fur_version => { objects = Some(objects::objects(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                0xB if fur_version => { texture_refs = Some(textures::textures(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                0xC if fur_version => { palette_refs = Some(palettes::palettes(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                0x0 => { definitions = Some(definitions::definitions(data.relative_fancy_slice(resources_offset as usize..), resources)) }
                0x1 => { bones = Some(bones::bones(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                0x2 => { vertices = Some(vertices::vertices(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                0x3 => { _normals = Some(resources) }
                0x4 => { _colors = Some(resources) }
                0x5 => { _uv = Some(resources) }
                0x6 => { _materials = Some(resources) }
                0x7 => { _shaders = Some(resources) }
                0x8 => { objects = Some(objects::objects(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                0x9 => { texture_refs = Some(textures::textures(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                0xA => { palette_refs = Some(palettes::palettes(data.relative_fancy_slice(resources_offset as usize ..), resources)) }
                _   => { unreachable!() }
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
        _normals,
        _colors,
        _uv,
        fur_vectors,
        fur_layer_coords,
        _materials,
        _shaders,
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
    pub definitions: Option<Definitions>,
    pub bones: Option<Bone>,
    pub vertices: Option<Vec<Vertices>>,
    _normals: Option<Vec<Resource>>,
    _colors: Option<Vec<Resource>>,
    _uv: Option<Vec<Resource>>,
    fur_vectors: Option<Vec<Resource>>,
    fur_layer_coords: Option<Vec<Resource>>,
    _materials: Option<Vec<Resource>>,
    _shaders: Option<Vec<Resource>>,
    pub objects: Option<Vec<Object>>,
    pub texture_refs: Option<Vec<Texture>>,
    pub palette_refs: Option<Vec<Palette>>,
}

impl Mdl0 {
    pub fn compile(&self, bres_offset: i32) -> Vec<u8> {
        let mut output = vec![];

        // create mdl0 header
        output.extend("MDL0".chars().map(|x| x as u8));
        output.extend(&i32::to_be_bytes(0x512e)); // size
        output.extend(&i32::to_be_bytes(self.version));
        output.extend(&i32::to_be_bytes(bres_offset));

        // TODO: Determine version from the fields used
        let props_offset = match self.version {
            0x08 => 0x40,
            0x09 => 0x40,
            0x0A => 0x48,
            0x0B => 0x4C,
            _ => panic!("Unknown MDL0 version"),
        };

        let num_props = match (&self.fur_vectors, &self.fur_layer_coords) {
            (Some(_), Some(_)) => 0xD,
            (None, None) => 0xB,
            _ => panic!("Can't have just one of the fur fields set to Some(_)"),
        };

        let header_size = props_offset + num_props;

        let definitions = self.definitions.as_ref().unwrap().compile();

        output.extend(&i32::to_be_bytes(header_size)); // definitions_offset
        output.extend(&i32::to_be_bytes(header_size + definitions.len() as i32)); // bones_offset
        output.extend(&i32::to_be_bytes(0)); // TODO: vertices_offset
        output.extend(&i32::to_be_bytes(0)); // TODO: normals_offset
        output.extend(&i32::to_be_bytes(0)); // TODO: colors_offset
        output.extend(&i32::to_be_bytes(0)); // TODO: uv_offset
        output.extend(&i32::to_be_bytes(0)); // TODO: materials_offset
        output.extend(&i32::to_be_bytes(0)); // TODO: shaders_offset
        output.extend(&i32::to_be_bytes(0)); // TODO: objects_offset
        output.extend(&i32::to_be_bytes(0)); // TODO: texture_refs_offset
        output.extend(&i32::to_be_bytes(0)); // TODO: palette_refs_offset

        if self.version >= 0xA {
            output.extend(&i32::to_be_bytes(0)); // TODO: fur_vectors_offset
            output.extend(&i32::to_be_bytes(0)); // TODO: fur_layer_coords_offset
        }
        if self.version >= 0xB {
            output.extend(&i32::to_be_bytes(0)); // TODO: An extra something ... goes here
        }

        output.extend(&i32::to_be_bytes(0)); // TODO: string_offset

        // TODO: Many of these should be generated rather than stored
        if let Some(props) = &self.props {
            output.extend(&u32::to_be_bytes(props.header_len));
            output.extend(&i32::to_be_bytes(props.mdl0offset));
            output.extend(&i32::to_be_bytes(props.scaling_rule));
            output.extend(&i32::to_be_bytes(props.tex_matrix_mode));
            output.extend(&i32::to_be_bytes(props.num_vertices));
            output.extend(&i32::to_be_bytes(props.num_triangles));
            output.extend(&i32::to_be_bytes(props.orig_path_offset));
            output.extend(&i32::to_be_bytes(props.num_nodes));
            output.push(props.need_nrm_mtx_array);
            output.push(props.need_tex_mtx_array);
            output.push(props.enable_extents);
            output.push(props.env_mtx_mode);
            output.extend(&i32::to_be_bytes(props.data_offset));
            output.extend(&props.extents.compile());
        }

        // TODO: What is the data here???
        output.extend(&vec![0x00; 0x8]);

        output.extend(definitions);

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
