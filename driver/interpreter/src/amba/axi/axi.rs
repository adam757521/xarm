// Similar to AXI Channels.
// This should support bursting lol.

#[derive(Copy, Clone, Debug)]
pub struct Size(u8);

impl From<Size> for usize {
    fn from(val: Size) -> usize {
        match val.0 & 0b111 {
            0b000 => 1,
            0b001 => 2,
            0b010 => 4,
            0b011 => 8,
            0b100 => 16,
            0b101 => 32,
            0b110 => 64,
            0b111 => 128,
            _ => unreachable!(),
        }
    }
}

impl From<u8> for Size {
    fn from(val: u8) -> Size {
        assert!(val & 0b111 == val);
        Size(val)
    }
}

#[derive(Clone, Debug)]
pub struct Metadata {
    pub address: u32,
    pub size: Size,
    // A lot of burst information.
}

#[derive(Clone, Debug)]
pub struct Data(Vec<u8>);

impl Data {
    fn from_payload_size(payload: Vec<u8>, size: Size) -> Data {
        let bytes_per_beat: usize = size.into();
        assert!(payload.len() == bytes_per_beat);
        Data(payload)
    }
}

#[derive(Clone, Debug)]
pub struct Beat {
    pub metadata: Metadata,
    pub write_data: Option<Data>
}

impl Beat {
    pub fn new(address: u32, write_data: Option<Vec<u8>>, size: Size) -> Self {
        Beat {
            metadata: Metadata {
                address,
                size,
            },
            write_data: write_data.map(|d| Data::from_payload_size(d, size))
        }
    }
}

pub trait Slave {
    fn process_beat(&mut self, beat: &Beat) -> Option<Data>;
}
