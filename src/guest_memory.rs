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

    pub fn write_bytes(&self, table: &mut GuestPageTable, src: &[u8], flags: u64) {
        let raw_ptr = self.data.as_ptr() as *mut u8;
        let slice = unsafe { core::slice::from_raw_parts_mut(raw_ptr, SIZE) };
        slice[..src.len()].copy_from_slice(src);
        for off in (0..SIZE as u64).step_by(4096) {
			let guest_addr = self.guest_base + off;
			let host_addr = raw_ptr as u64 + off;
            table.map(guest_addr, host_addr, flags);
        }
    }
}
