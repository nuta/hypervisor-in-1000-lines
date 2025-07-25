#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
mod print;
mod allocator;
mod guest_page_table;
mod plic;
mod trap;
mod vcpu;
mod linux_loader;
mod virtio_blk;
mod guest_memory;

use core::arch::asm;
use core::panic::PanicInfo;

use crate::{
    guest_page_table::GuestPageTable, linux_loader::{GUEST_BASE_ADDR, GUEST_DTB_ADDR}, vcpu::VCpu
};

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub extern "C" fn boot() -> ! {
    unsafe {
        asm!(
            "la sp, __stack_top",
            "j {main}",
            main = sym main,
            options(noreturn)
        );
    }
}

unsafe extern "C" {
    static mut __bss: u8;
    static mut __bss_end: u8;
    static mut __heap: u8;
    static mut __heap_end: u8;
}

fn main() -> ! {
    unsafe {
        let bss_start = &raw mut __bss;
        let bss_size = (&raw mut __bss_end as usize) - (&raw mut __bss as usize);
        core::ptr::write_bytes(bss_start, 0, bss_size);

        asm!("csrw stvec, {}", in(reg) trap::trap_handler as usize);
    }

    println!("\nBooting hypervisor...");

    allocator::GLOBAL_ALLOCATOR.init(&raw mut __heap, &raw mut __heap_end);

    let kernel_image = include_bytes!("../linux/Image");
    let mut table = GuestPageTable::new();
    linux_loader::load_linux_kernel(&mut table, kernel_image);
    let mut vcpu = VCpu::new(&table, GUEST_BASE_ADDR);
    vcpu.a0 = 0; // hart ID
    vcpu.a1 = GUEST_DTB_ADDR; // device tree address
    vcpu.run();
}

#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic: {}", info);
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
