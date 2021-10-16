use crate::wii_memory::WiiMemory;

use fancy_slice::FancySlice;

pub(crate) fn arc_item_data(
    _parent_data: FancySlice,
    _data: FancySlice,
    _wii_memory: &WiiMemory,
) -> ArcItemData {
    ArcItemData {}
}

#[derive(Clone, Debug)]
pub struct ArcItemData {}
