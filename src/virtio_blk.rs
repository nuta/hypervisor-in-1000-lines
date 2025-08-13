use core::num::Wrapping;

use spin::Mutex;

use crate::guest_memory::GUEST_MEMORY;

pub const DISK_IMAGE: &[u8] = include_bytes!("../linux/rootfs.squashfs");
pub const DISK_CAPACITY: u64 = DISK_IMAGE.len() as u64 / 512;

pub static VIRTIO_BLK: Mutex<VirtioBlk> = Mutex::new(VirtioBlk::new());

fn set_low_32(value: &mut u64, low: u64) {
    *value = (*value & 0xffff_ffff_0000_0000) | low;
}

fn set_high_32(value: &mut u64, high: u64) {
    *value = (*value & 0x0000_0000_ffff_ffff) | (high << 32);
}

#[repr(C)]
struct VirtqDesc {
    addr: u64,  // Buffer physical address
    len: u32,   // Buffer length
    flags: u16, // Buffer flags
    next: u16,  // Next descriptor index (if chained)
}

#[repr(C)]
struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; 128], // Ring of descriptor indices
}

#[repr(C)]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; 128],
}

#[repr(C)]
struct VirtqUsedElem {
    id: u32,  // Descriptor index
    len: u32, // Length of data written
}


#[repr(C)]
struct VirtioBlkReq {
    req_type: u32,
    reserved: u32,
    sector: u64,
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
    requestq_next_avail: Wrapping<u16>,
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
            requestq_next_avail: Wrapping(0),
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
            0x50 => self.process_queue(value as usize), // Queue notify
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
            0xfc => 0x0,                        // Config generation
            0x100 => DISK_CAPACITY & 0xffffffff, // Device-specific config: capacity (low 32 bits)
            0x104 => DISK_CAPACITY >> 32, // Device-specific config: capacity (high 32 bits)
            _ => panic!("unknown virtio-blk mmio read: guest_addr={:#x}", offset),
        }
    }

    fn process_queue(&mut self, queue_index: usize) {
        assert_eq!(queue_index, 0, "only queue #0 (requestq) is supported");
		println!("[virtio-blk] processing requestq");
        loop {
            let avail: VirtqAvail = GUEST_MEMORY.read(self.requestq_driver_addr);
            if self.requestq_next_avail.0 == u16::from_le(avail.idx) {
                break;
            }
            
            println!("avail: {:?}", avail.idx);
            let avail_index = self.requestq_next_avail.0 as u64 % self.requestq_size as u64;
            let desc0_index = u16::from_le(avail.ring[avail_index as usize]) as u64;
            let desc0: VirtqDesc = GUEST_MEMORY.read(self.requestq_desc_addr + desc0_index * size_of::<VirtqDesc>() as u64);
            let desc0_addr = u64::from_le(desc0.addr);
            let desc0_len = u32::from_le(desc0.len);
            let desc0_next = u16::from_le(desc0.next);
           
            println!("desc[{}]: addr={:#x}, len={}, next={}", desc0_index, desc0_addr, desc0_len, desc0_next);

            let desc1_index = desc0_next as u64;
            let desc1: VirtqDesc = GUEST_MEMORY.read(self.requestq_desc_addr + desc0_next as u64 * size_of::<VirtqDesc>() as u64);
            let desc1_addr = u64::from_le(desc1.addr);
            let desc1_len = u32::from_le(desc1.len);
            let desc1_next = u16::from_le(desc1.next);
            println!("desc[{}]: addr={:#x}, len={}, next={}", desc1_index, desc1_addr, desc1_len, desc1_next);

            let desc2_index = desc1_next as u64;
            let desc2: VirtqDesc = GUEST_MEMORY.read(self.requestq_desc_addr + desc1_next as u64 * size_of::<VirtqDesc>() as u64);
            let desc2_addr = u64::from_le(desc2.addr);
            let desc2_len = u32::from_le(desc2.len);
            let desc2_next = u16::from_le(desc2.next);
            println!("desc[{}]: addr={:#x}, len={}, next={}", desc2_index, desc2_addr, desc2_len, desc2_next);

            let req: VirtioBlkReq = GUEST_MEMORY.read(desc1_addr);
            let sector = u64::from_le(req.sector);
            let start = (sector * 512) as usize;
            let end = start + desc1_len as usize;
            GUEST_MEMORY.write_bytes(desc2_addr, &DISK_IMAGE[start..end]);

            let used: VirtqUsed = GUEST_MEMORY.read(self.requestq_device_addr);

            self.requestq_next_avail += 1;
        }
    }
}
