use spin::Mutex;

pub static VIRTIO_BLK: Mutex<VirtioBlk> = Mutex::new(VirtioBlk::new());

fn set_low_32(value: &mut u64, low: u64) {
    *value = (*value & 0xffff_ffff_0000_0000) | low;
}

fn set_high_32(value: &mut u64, high: u64) {
    *value = (*value & 0x0000_0000_ffff_ffff) | (high << 32);
}

pub struct VirtioBlk {
    status: u32,
    device_features_sel: u32,
    driver_features_sel: u32,
    requestq_ready: u32,
    requestq_size: u32,
    requestq_desc_addr: u64,
    requestq_driver_addr: u64,
    requestq_device_addr: u64,
}

impl VirtioBlk  {
    pub const fn new() -> Self {
        Self {
            status: 0,
            device_features_sel: 0,
            driver_features_sel: 0,
            requestq_ready: 0,
            requestq_size: 0,
            requestq_desc_addr: 0,
            requestq_driver_addr: 0,
            requestq_device_addr: 0,
        }
    }

    pub fn handle_mmio_write(&mut self, offset: u64, value: u64, width: u64) {
        println!("[virtio-blk] MMIO write at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            0x70 => self.status = value as u32, // Device status
            0x14 => self.device_features_sel = value as u32, // Device features selection
            0x20 => {},                                      // Driver features
            0x24 => self.driver_features_sel = value as u32, // Driver features selection
            0x30 => assert_eq!(value, 0), // Queue select (must be requestq)
            0x38 => self.requestq_size = value as u32, // Queue size (# of descriptors)
            0x44 => {}, // Queue ready (ignored)
            0x80 => set_low_32(&mut self.requestq_desc_addr, value),
            0x84 => set_high_32(&mut self.requestq_desc_addr, value),
            0x90 => set_low_32(&mut self.requestq_driver_addr, value),
            0x94 => set_high_32(&mut self.requestq_driver_addr, value),
            0xa0 => set_low_32(&mut self.requestq_device_addr, value),
            0xa4 => set_high_32(&mut self.requestq_device_addr, value),
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
            0x10 if self.device_features_sel == 0 => 0, // Device features: 31-0 bits
            0x10 if self.device_features_sel == 1 => 0x0000_0001, // Device features: 63-32 bits (VIRTIO_F_VERSION_1)
            0x34 => 128, // Max queue size (# of descriptors)
            0x44 => self.requestq_ready as u64, // Queue ready
            0x70 => self.status as u64,  // Device status
            0x80 => self.requestq_desc_addr & 0xffff_ffff,
            0x84 => self.requestq_desc_addr >> 32,
            0x90 => self.requestq_driver_addr & 0xffff_ffff,
            0x94 => self.requestq_driver_addr >> 32,
            0xa0 => self.requestq_device_addr & 0xffff_ffff,
            0xa4 => self.requestq_device_addr >> 32,
            _ => panic!("unknown virtio-blk mmio read: guest_addr={:#x}", offset),
        }
    }
}
