use cgmath::Vector3;
use fancy_slice::FancySlice;

pub fn mbox(data: FancySlice) -> MBox {
    MBox {
        min: Vector3::<f32>::new(data.f32_be(0x00), data.f32_be(0x04), data.f32_be(0x08)),
        max: Vector3::<f32>::new(data.f32_be(0x0c), data.f32_be(0x10), data.f32_be(0x14)),
    }
}

// named MBox because Box is used in std lib
#[derive(Debug, Clone)]
pub struct MBox {
    min: Vector3<f32>,
    max: Vector3<f32>,
}

impl MBox {
    pub fn compile(&self) -> Vec<u8> {
        let mut output = vec![];

        output.extend(&u32::to_be_bytes(self.min.x.to_bits()));
        output.extend(&u32::to_be_bytes(self.min.y.to_bits()));
        output.extend(&u32::to_be_bytes(self.min.z.to_bits()));
        output.extend(&u32::to_be_bytes(self.max.x.to_bits()));
        output.extend(&u32::to_be_bytes(self.max.y.to_bits()));
        output.extend(&u32::to_be_bytes(self.max.z.to_bits()));

        output
    }
}
