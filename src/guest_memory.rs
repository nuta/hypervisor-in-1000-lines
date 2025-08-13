use core::mem::MaybeUninit;

use crate::{guest_page_table::GuestPageTable, linux_loader::{GUEST_BASE_ADDR, GUEST_DTB_ADDR, MEMORY_SIZE}};

pub static GUEST_MEMORY: GuestMemory<MEMORY_SIZE> = GuestMemory::new(GUEST_BASE_ADDR);
pub static DTB_MEMORY: GuestMemory<0x10000> = GuestMemory::new(GUEST_DTB_ADDR);

#[repr(C, align(4096))]
pub struct GuestMemory<const SIZE: usize> {
    data: [u8; SIZE],
    guest_base: u64,
}

impl<const SIZE: usize> GuestMemory<SIZE> {
    pub const fn new(guest_base: u64) -> Self {
        Self { data: [0; SIZE], guest_base }
    }

    pub fn write_and_map(&self, table: &mut GuestPageTable, src: &[u8], flags: u64) {
        self.write_bytes(self.guest_base, src);
        for off in (0..SIZE as u64).step_by(4096) {
			let guest_addr = self.guest_base + off;
			let host_addr = self.data.as_ptr() as u64 + off;
            table.map(guest_addr, host_addr, flags);
        }
    }

    pub fn write_bytes(&self, guest_addr: u64, src: &[u8]) {
        let offset = guest_addr - self.guest_base;
        let raw_ptr = self.data.as_ptr() as *mut u8;
        let slice = unsafe { core::slice::from_raw_parts_mut(raw_ptr.add(offset as usize), src.len()) };
        slice.copy_from_slice(src);
    }

    pub fn read_bytes(&self, guest_addr: u64, dst: &mut [u8]) {
        let offset = guest_addr - self.guest_base;
        let raw_ptr = self.data.as_ptr() as *const u8;
        let slice = unsafe { core::slice::from_raw_parts(raw_ptr.add(offset as usize), dst.len()) };
        dst.copy_from_slice(slice);
    }

    pub fn read<T>(&self, guest_addr: u64) -> T {
        let mut tmp = MaybeUninit::<T>::uninit();
        let dst = unsafe { core::slice::from_raw_parts_mut(tmp.as_mut_ptr() as *mut u8, size_of::<T>()) }; 
        self.read_bytes(guest_addr, dst);
        unsafe { tmp.assume_init() }
    }
}
