use fancy_slice::FancySlice;

use crate::resources::Resource;

#[rustfmt::skip]
pub(crate) fn objects(data: FancySlice, resources: Vec<Resource>) -> Vec<Object> {
    let mut objects = vec!();
    for resource in resources {
        let data = data.relative_fancy_slice(resource.data_offset as usize ..);
        let _total_length           = data.i32_be(0x00);
        let _mdl0_offset            = data.i32_be(0x04);
        let single_bind_node_id     = data.i32_be(0x08);
        let vertex_format1          = data.u32_be(0x0c);
        let vertex_format2          = data.u32_be(0x10);
        let vertex_specs            = data.u32_be(0x14);
        let definitions_buffer_size = data.i32_be(0x18);
        let definitions_size        = data.i32_be(0x1c); // amount of the buffer_size that is actually used
        let definitions_offset      = data.i32_be(0x20); // relative to this struct
        let primitives_buffer_size  = data.i32_be(0x24);
        let primitives_size         = data.i32_be(0x28); // amount of the buffer_size that is actually used
        let primitives_offset       = data.i32_be(0x2c); // relative to this struct
        let array_flags             = data.u32_be(0x30);
        let modifier                = data.u32_be(0x34);
        let string_offset           = data.u32_be(0x38);
        let index                   = data.u32_be(0x3c);
        let num_vertices            = data.u32_be(0x40);
        let num_faces               = data.u32_be(0x44);
        let vertex_id               = data.i16_be(0x48);
        let normal_id               = data.i16_be(0x4A);

        let mut color_ids = [0; 2];
        color_ids[0] = data.i16_be(0x4C);
        color_ids[1] = data.i16_be(0x4E);

        let mut uv_ids = [0; 8];
        // Not sure if I can actually mutate like clippy is suggesting
        #[allow(clippy::needless_range_loop)]
        for i in 0..uv_ids.len() {
            uv_ids[i] = data.i16_be(0x50 + i * 2);
        }

        let single_bind_node_id = if single_bind_node_id < 0 {
            None
        } else {
            Some(single_bind_node_id as u32)
        };

        let modifier = Modifier::new(modifier);

        let name = if string_offset == 0 {
            None
        } else {
            Some(data.str(string_offset as usize).unwrap().to_string())
        };

        objects.push(Object {
            single_bind_node_id,
            vertex_format1,
            vertex_format2,
            vertex_specs,
            definitions_buffer_size,
            definitions_size,
            definitions_offset,
            primitives_buffer_size,
            primitives_size,
            primitives_offset,
            array_flags,
            modifier,
            name,
            index,
            num_vertices,
            num_faces,
            vertex_id,
            normal_id,
            color_ids,
            uv_ids,
        });
    }

    objects
}

const _OBJECT_SIZE: usize = 0x64;
#[derive(Debug, Clone)]
#[rustfmt::skip]
pub struct Object {
    pub single_bind_node_id: Option<u32>,

    // TODO: I should really split these flags into individual fields,
    // that way code can naturally create an Object by creating a struct

    /// 0000 0000 0000 0000 0000 0000 0000 0001 - Vertex/Normal matrix index
    /// 0000 0000 0000 0000 0000 0000 0000 0010 - Texture Matrix 0
    /// 0000 0000 0000 0000 0000 0000 0000 0100 - Texture Matrix 1
    /// 0000 0000 0000 0000 0000 0000 0000 1000 - Texture Matrix 2
    /// 0000 0000 0000 0000 0000 0000 0001 0000 - Texture Matrix 3
    /// 0000 0000 0000 0000 0000 0000 0010 0000 - Texture Matrix 4
    /// 0000 0000 0000 0000 0000 0000 0100 0000 - Texture Matrix 5
    /// 0000 0000 0000 0000 0000 0000 1000 0000 - Texture Matrix 6
    /// 0000 0000 0000 0000 0000 0001 0000 0000 - Texture Matrix 7
    /// 0000 0000 0000 0000 0000 0110 0000 0000 - Vertex format
    /// 0000 0000 0000 0000 0001 1000 0000 0000 - Normal format
    /// 0000 0000 0000 0000 0110 0000 0000 0000 - Color0 format
    /// 0000 0000 0000 0001 1000 0000 0000 0000 - Color1 format
    vertex_format1: u32,

    /// 0000 0000 0000 0000 0000 0000 0000 0011 - Tex0 format
    /// 0000 0000 0000 0000 0000 0000 0000 1100 - Tex1 format
    /// 0000 0000 0000 0000 0000 0000 0011 0000 - Tex2 format
    /// 0000 0000 0000 0000 0000 0000 1100 0000 - Tex3 format
    /// 0000 0000 0000 0000 0000 0011 0000 0000 - Tex4 format
    /// 0000 0000 0000 0000 0000 1100 0000 0000 - Tex5 format
    /// 0000 0000 0000 0000 0011 0000 0000 0000 - Tex6 format
    /// 0000 0000 0000 0000 1100 0000 0000 0000 - Tex7 format
    vertex_format2: u32,

    /// 0000 0000 0000 0000 0000 0000 0000 0011 - Num colors
    /// 0000 0000 0000 0000 0000 0000 0000 1100 - Normal type (0 = none, 1 = normals, 2 = normals + binormals)
    /// 0000 0000 0000 0000 0000 0000 1111 0000 - Num textures
    vertex_specs: u32,

    // TODO: Use these fields to put the data into a vec
    definitions_buffer_size: i32,
    definitions_size: i32,
    definitions_offset: i32,

    // TODO: Use these fields to put the data into a vec
    primitives_buffer_size: i32,
    primitives_size: i32,
    primitives_offset: i32,

    // TODO: havent implemented getters for this field as I want to rewrite to split into seperate fields instead

    /// 0000 0000 0000 0000 0000 0001 Pos Matrix
    /// 0000 0000 0000 0000 0000 0010 TexMtx0
    /// 0000 0000 0000 0000 0000 0100 TexMtx1
    /// 0000 0000 0000 0000 0000 1000 TexMtx2
    /// 0000 0000 0000 0000 0001 0000 TexMtx3
    /// 0000 0000 0000 0000 0010 0000 TexMtx4
    /// 0000 0000 0000 0000 0100 0000 TexMtx5
    /// 0000 0000 0000 0000 1000 0000 TexMtx6
    /// 0000 0000 0000 0001 0000 0000 TexMtx7
    /// 0000 0000 0000 0010 0000 0000 Positions
    /// 0000 0000 0000 0100 0000 0000 Normals
    /// 0000 0000 0000 1000 0000 0000 Color0
    /// 0000 0000 0001 0000 0000 0000 Color1
    /// 0000 0000 0010 0000 0000 0000 Tex0
    /// 0000 0000 0100 0000 0000 0000 Tex1
    /// 0000 0000 1000 0000 0000 0000 Tex2
    /// 0000 0001 0000 0000 0000 0000 Tex3
    /// 0000 0010 0000 0000 0000 0000 Tex4
    /// 0000 0100 0000 0000 0000 0000 Tex5
    /// 0000 1000 0000 0000 0000 0000 Tex6
    /// 0001 0000 0000 0000 0000 0000 Tex7
    array_flags: u32,

    pub modifier: Modifier,
    pub name: Option<String>,
    pub index: u32,
    pub num_vertices: u32,
    pub num_faces: u32,

    pub vertex_id: i16,
    pub normal_id: i16,

    pub color_ids: [i16; 2],
    pub uv_ids: [i16; 8],
}

impl Object {
    pub fn has_vertex_matrix(&self) -> bool {
        self.vertex_format1 & 1 != 0
    }

    pub fn has_tex_matrix(&self, index: usize) -> bool {
        if index > 7 {
            false
        } else {
            (self.vertex_format1 >> (index + 1)) & 1 != 0
        }
    }

    pub fn vertex_format(&self) -> XFDataFormat {
        XFDataFormat::new((self.vertex_format1 >> 9) & 0b11)
    }

    pub fn normal_format(&self) -> XFDataFormat {
        XFDataFormat::new((self.vertex_format1 >> 11) & 0b11)
    }

    pub fn color_format(&self, index: usize) -> Option<XFDataFormat> {
        if index > 1 {
            None
        } else {
            Some(XFDataFormat::new(
                (self.vertex_format1 >> (index * 2 + 13)) & 0b11,
            ))
        }
    }

    pub fn tex_format(&self, index: usize) -> Option<XFDataFormat> {
        if index > 7 {
            None
        } else {
            Some(XFDataFormat::new(
                (self.vertex_format2 >> (index * 2)) & 0b11,
            ))
        }
    }

    pub fn num_colors(&self) -> usize {
        (self.vertex_specs & 0b11) as usize
    }

    pub fn normal_type(&self) -> XFNormalType {
        XFNormalType::new((self.vertex_specs >> 2) & 0b11)
    }

    pub fn num_textures(&self) -> usize {
        ((self.vertex_specs >> 4) & 0xb1111) as usize
    }
}

#[derive(Debug, Clone)]
pub enum XFDataFormat {
    None,
    Direct,
    Index8,
    Index16,
}

impl XFDataFormat {
    fn new(value: u32) -> XFDataFormat {
        match value {
            0 => XFDataFormat::None,
            1 => XFDataFormat::Direct,
            2 => XFDataFormat::Index8,
            3 => XFDataFormat::Index16,
            _ => panic!("Unknown XFDataFormat."),
        }
    }
}

#[derive(Debug, Clone)]
pub enum XFNormalType {
    None,
    XYZ,
    NBT,
}

impl XFNormalType {
    fn new(value: u32) -> XFNormalType {
        match value {
            0 => XFNormalType::None,
            1 => XFNormalType::XYZ,
            2 => XFNormalType::NBT,
            _ => panic!("Unknown XFNormalType."),
        }
    }
}

// TODO: This needs a better name but not sure how its used.
// Brawlbox calls it flag which is even worse
#[derive(Debug, Clone)]
pub enum Modifier {
    None,
    ChangeCurrentMatrix,
    Invisible,
}

impl Modifier {
    fn new(value: u32) -> Self {
        match value {
            0 => Modifier::None,
            1 => Modifier::ChangeCurrentMatrix,
            2 => Modifier::Invisible,
            _ => panic!("Unknown Modifier."),
        }
    }
}
