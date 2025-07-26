use spin::Mutex;

pub static VIRTIO_BLK: Mutex<VirtioBlk> = Mutex::new(VirtioBlk::new());

pub struct VirtioBlk {
    status: u32,
    features_select: u32,
}

impl VirtioBlk  {
    pub const fn new() -> Self {
        Self {
            status: 0,
            features_select: 0,
        }
    }

    pub fn handle_mmio_write(&mut self, offset: u64, value: u64, width: u64) {
        println!("[virtio-blk] MMIO write at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            0x70 => self.status = value as u32, // Device status
            0x14 => self.features_select = value as u32, // Device features selection
            _ => panic!("unknown virtio-blk mmio write: offs={:#x}", offset),
        }
    }

    pub fn handle_mmio_read(&self, offset: u64, width: u64) -> u64 {
        println!("[virtio-blk] MMIO read at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            0x00 => 0x74726976,  // Magic value "virt"
            0x04 => 0x2,         // Version
            0x08 => 0x2,         // Device ID (block device)
            0x0c => 0x554d4551,  // Vendor ID "QEMU"
            0x10 if self.features_select == 0 => 0, // Device features: 31-0 bits
            0x10 if self.features_select == 1 => 0x0000_0001, // Device features: 63-32 bits (VIRTIO_F_VERSION_1)
            0x70 => self.status as u64,  // Device status
            _ => panic!("unknown virtio-blk mmio read: guest_addr={:#x}", offset),
        }
    }
}
