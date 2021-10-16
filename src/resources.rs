use fancy_slice::FancySlice;

// ResourceGroup in brawlbox
#[rustfmt::skip]
pub(crate) fn resources(data: FancySlice) -> Vec<Resource> {
    let total_size   = data.i32_be(0);
    let num_children = data.i32_be(4);

    assert_eq!(total_size, (num_children + 1) * RESOURCE_SIZE as i32 + RESOURCE_HEADER_SIZE as i32);

    let mut resources = vec!();
    for i in 1..=num_children { // the first child is a dummy so we skip it.
        let child_index = RESOURCE_HEADER_SIZE + RESOURCE_SIZE * i as usize;

        let string_offset = data.i32_be(child_index + 8);

        resources.push(Resource {
            //id:          data.u16_be(child_index as usize),
            flag:          data.u16_be(child_index as usize + 0x2),
            //left_index:  data.u16_be(child_index as usize + 0x4),
            //right_index: data.u16_be(child_index as usize + 0x6),
            data_offset:   data.i32_be(child_index + 0xc),
            string:        data.str(string_offset as usize).unwrap().to_string(),
        });
    }

    resources
}

pub(crate) const RESOURCE_HEADER_SIZE: usize = 0x8;

pub(crate) const RESOURCE_SIZE: usize = 0x10;
#[derive(Clone, Debug)]
pub struct Resource {
    flag: u16,
    pub data_offset: i32,
    pub string: String,
}
