use spin::Mutex;

fn set_low_32(value: &mut u64, low: u32) {
    *value = (*value & 0xffffffff00000000) | (low as u64);
}

fn set_high_32(value: &mut u64, high: u32) {
    *value = (*value & 0x00000000ffffffff) | ((high as u64) << 32);
}

fn get_low_32(value: u64) -> u32 {
    (value & 0xffffffff) as u32
}

fn get_high_32(value: u64) -> u32 {
    ((value >> 32) & 0xffffffff) as u32
}

pub static VIRTIO_BLK: Mutex<VirtioBlk> = Mutex::new(VirtioBlk::new());

#[derive(Debug)]
pub struct VirtioBlk {
    pub device_features_sel: u32,
    pub driver_features_sel: u32,
    pub driver_features: [u32; 2],
    pub queue_sel: u32,
    pub device_status: u32,
    pub queue_num: u32,
    pub queue_desc: u64,
    pub queue_avail: u64,
    pub queue_used: u64,
    pub queue_ready: u32,
    pub irq_status: u32,
}

impl VirtioBlk {
    pub const fn new() -> Self {
        Self {
            device_features_sel: 0,
            driver_features_sel: 0,
            driver_features: [0, 0],
            queue_sel: 0,
            device_status: 0,
            queue_num: 0,
            queue_desc: 0,
            queue_avail: 0,
            queue_used: 0,
            queue_ready: 0,
            irq_status: 0,
        }
    }

    pub fn handle_mmio_write(
        &mut self,
        guest_addr: u64,
        value: u64,
        width: u64,
    ) {
        assert_eq!(width, 4);
        match guest_addr {
            0x14 => self.device_features_sel = value as u32,
            0x20 => {
                let sel = self.driver_features_sel as usize;
                if sel < 2 {
                    self.driver_features[sel] = value as u32;
                }
            }
            0x24 => self.driver_features_sel = value as u32,
            0x30 => self.queue_sel = value as u32,
            0x38 => self.queue_num = value as u32,
            0x44 => self.queue_ready = value as u32,
            0x50 => self.process_virtqueue(),
            0x70 => self.device_status = value as u32,
            0x80 => set_low_32(&mut self.queue_desc, value as u32),
            0x84 => set_high_32(&mut self.queue_desc, value as u32),
            0x90 => set_low_32(&mut self.queue_avail, value as u32),
            0x94 => set_high_32(&mut self.queue_avail, value as u32),
            0xa0 => set_low_32(&mut self.queue_used, value as u32),
            0xa4 => set_high_32(&mut self.queue_used, value as u32),
            0x60 => self.irq_status &= !(value as u32),
            0x64 => {},
            _ => panic!("unknown virtio-blk mmio write: guest_addr={:#x}", guest_addr),
        }
    }

    pub fn handle_mmio_read(&self, guest_addr: u64, _width: u64) -> u64 {
        match guest_addr {
            0x00 => 0x74726976,  // Magic value "virt"
            0x04 => 0x2,         // Version
            0x08 => 0x2,         // Device ID (block device)
            0x0c => 0x554d4551,  // Vendor ID "QEMU"
            0x10 => {
                match self.device_features_sel {
                    0 => 0,
                    1 => 1 << 0,  // VIRTIO_F_VERSION_1
                    _ => 0,
                }
            }
            0x20 => {
                let sel = self.driver_features_sel as usize;
                if sel < 2 { self.driver_features[sel] as u64 } else { 0 }
            }
            0x34 => 256,  // QUEUE_NUM_MAX
            0x38 => self.queue_num as u64,
            0x44 => self.queue_ready as u64,
            0x70 => self.device_status as u64,
            0x80 => get_low_32(self.queue_desc) as u64,
            0x84 => get_high_32(self.queue_desc) as u64,
            0x90 => get_low_32(self.queue_avail) as u64,
            0x94 => get_high_32(self.queue_avail) as u64,
            0xa0 => get_low_32(self.queue_used) as u64,
            0xa4 => get_high_32(self.queue_used) as u64,
            0x100 => (128 * 1024) & 0xffffffff,  // Capacity low
            0x104 => ((128 * 1024) >> 32) & 0xffffffff,  // Capacity high
            0x60 => self.irq_status as u64,
            0xfc => 0x0,  // Config generation
            _ => panic!("unknown virtio-blk mmio read: guest_addr={:#x}", guest_addr),
        }
    }

    pub fn process_virtqueue(&mut self) {        
        todo!("guest requested a virtio-blk operation");
    }
}