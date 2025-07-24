use crate::{allocator::alloc_pages, guest_page_table::{GuestPageTable, PTE_R, PTE_W, PTE_X}};
use core::mem::size_of;

#[repr(C)]
struct RiscvImageHeader {
    code0: u32,
    code1: u32,
    text_offset: u64,
    image_size: u64,
    flags: u64,
    version: u32,
    reserved1: u32,
    reserved2: u64,
    magic: u64,
    magic2: u32,
    reserved3: u32,
}

pub const GUEST_BASE_ADDR: u64 = 0x8000_0000;
const MEMORY_SIZE: usize = 64 * 1024 * 1024;

fn copy_and_map(table: &mut GuestPageTable, data: &[u8], guest_addr: u64, len: usize, flags: u64) {
    assert!(data.len() <= len, "data is beyond the region");
    let raw_ptr = alloc_pages(len);
    unsafe {
        core::ptr::copy_nonoverlapping(data.as_ptr(), raw_ptr, data.len());
    }
    for off in (0..len).step_by(4096) {
        table.map(guest_addr + off as u64, raw_ptr as u64 + off as u64, flags);
    }
}

pub fn load_linux_kernel(table: &mut GuestPageTable, image: &[u8]) {
    assert!(image.len() >= size_of::<RiscvImageHeader>());
    let header = unsafe { &*(image.as_ptr() as *const RiscvImageHeader) };
    assert_eq!(u32::from_le(header.magic2), 0x05435352, "invalid magic");

    let kernel_size = u64::from_le(header.image_size);
    assert!(image.len() <= MEMORY_SIZE);
    copy_and_map(table, image, GUEST_BASE_ADDR, MEMORY_SIZE, PTE_R | PTE_W | PTE_X);

    println!("loaded kernel: size={}KB", kernel_size / 1024);
}
