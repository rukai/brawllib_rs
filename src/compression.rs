use fancy_slice::FancySlice;

#[derive(Debug)]
enum CompressionType {
    None,
    LZ77,
    ExtendedLZ77,
}

impl CompressionType {
    fn new(value: u8) -> Self {
        match value {
            0b00000 => CompressionType::None,
            0b10000 => CompressionType::LZ77,
            0b10001 => CompressionType::ExtendedLZ77,
            _ => panic!("Unknown compression type {value}"),
        }
    }
}

pub fn decompress(bytes: FancySlice) -> Vec<u8> {
    let compression_type = CompressionType::new(bytes.u8(0));
    // TODO: bytes.le_u32(0) & 0xFFFFFF
    let decompressed_len =
        (bytes.u8(1) as u32) | (bytes.u8(2) as u32) << 8 | (bytes.u8(3) as u32) << 16;
    let mut output = if decompressed_len == 0 {
        panic!("extended length unimplemented")
    } else {
        vec![0; decompressed_len as usize]
    };
    match compression_type {
        CompressionType::ExtendedLZ77 => {
            // TODO: not quite sure where to start it
            decompress_lz77_extended(bytes.relative_slice(4..), &mut output);
        }
        other => panic!("Support for compression type {other:?} is not yet implemented"),
    }

    output
}

fn decompress_lz77_extended(data: &[u8], destination: &mut [u8]) {
    let mut source_index = 0;
    let mut dest_index = 0;
    while dest_index < destination.len() {
        let control = data[source_index];
        source_index += 1;
        for bit in (0..8).rev() {
            if dest_index >= destination.len() {
                return;
            }
            if (control & (1 << bit)) == 0 {
                destination[dest_index] = data[source_index];
                dest_index += 1;
                source_index += 1;
            } else {
                let nibble = (data[source_index] as i32) >> 4;
                let num;
                if nibble == 1 {
                    num = (((data[source_index] as i32 & 0x0F) << 12)
                        | ((data[source_index + 1] as i32) << 4)
                        | ((data[source_index + 2] as i32) >> 4))
                        + 0xFF
                        + 0xF
                        + 3;
                    source_index += 2;
                } else if nibble == 0 {
                    num = ((((data[source_index] as i32) & 0x0F) << 4)
                        | ((data[source_index + 1]) as i32 >> 4))
                        + 0xF
                        + 2;
                    source_index += 1;
                } else {
                    num = nibble + 1
                }
                let offset = ((((data[source_index] as i32) & 0xF) << 8)
                    | data[source_index + 1] as i32)
                    + 2;
                source_index += 2;

                for _ in 0..num {
                    if dest_index >= destination.len() {
                        return;
                    }
                    destination[dest_index] = destination[dest_index - offset as usize + 1];
                    dest_index += 1;
                }
            }
        }
    }
}
