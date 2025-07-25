use spin::Mutex;

pub static VIRTIO_BLK: Mutex<VirtioBlk> = Mutex::new(VirtioBlk::new());

pub struct VirtioBlk {
    device_status: u32,
}

impl VirtioBlk  {
    pub const fn new() -> Self {
        Self {
            device_status: 0,
        }
    }

    pub fn handle_mmio_write(&mut self, offset: u64, value: u64, width: u64) {
        assert_eq!(width, 4);
        match offset {
            0x70 => self.device_status = value as u32, // Device status
            _ => panic!("unknown virtio-blk mmio write: offs={:#x}", offset),
        }
    }

    pub fn handle_mmio_read(&self, offset: u64, _width: u64) -> u64 {
        println!("[MMIO]: read from virtio-blk at {:#x}", offset);
        match offset {
            0x00 => 0x74726976,  // Magic value "virt"
            0x04 => 0x2,         // Version
            0x08 => 0x2,         // Device ID (block device)
            0x0c => 0x554d4551,  // Vendor ID "QEMU"
            0x70 => self.device_status as u64,  // Device status
            _ => panic!("unknown virtio-blk mmio read: guest_addr={:#x}", offset),
        }
    }
}
