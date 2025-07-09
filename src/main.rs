#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
mod print;
mod allocator;

use core::arch::asm;
use core::panic::PanicInfo;

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
    static mut __heap: u8;
    static mut __heap_end: u8;
}

fn main() -> ! {
    println!("\nBooting hypervisor...");

    allocator::GLOBAL_ALLOCATOR.init(&raw mut __heap, &raw mut __heap_end);

    let mut v = alloc::vec::Vec::new();
    v.push('a');
    v.push('b');
    v.push('c');
    println!("v = {:?}", v);

    loop {}
}

#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic: {}", info);
    loop {}
}
