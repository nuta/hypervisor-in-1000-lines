---
title: Hello World
---

# Hello World

In the previous chapter, we wrote a minimal boot code in Rust.

## Supervisor Binary Interface (SBI)

OpenSBI, the firmware that we use, is not just a bootloader; it also provides the Supervisor Binary Interface (SBI) to the OS. SBI is an API for OS kernels: it defines what the firmware (OpenSBI) provides to an OS.

The SBI specification is [published on GitHub](https://github.com/riscv-non-isa/riscv-sbi-doc/releases). It defines useful features such as displaying characters on the debug console (e.g., serial port), reboot/shutdown, and timer settings.

## Putchar from SBI

SBI provides many features as you can see in the specification, including a function to print characters to the serial port.

Here's how to use it in Rust. To keep things organized, create a new file `print.rs`:

```rust [src/main.rs]
mod print;
```

```rust [src/print.rs]
use core::arch::asm;

pub fn sbi_putchar(ch: u8) {
    unsafe {
        asm!(
            "ecall",
            in("a6") 0, // SBI function ID
            in("a7") 1, // SBI extension ID (Console Putchar)
            inout("a0") ch as usize => _, // Argument #0
            out("a1") _ // Argument #1 (not used)
        );
    }
}
```

This inline assembly is simple. Fill the following registers and jump into the firmware mode (specifically RISC-V M-mode) using the `ecall` instruction:

- `a6` and `a7` registers specify the SBI feature to call.
- `a0` is the first argument to SBI, which is the character to print. `inout` means `a0` is also set by SBI (the error code).
- `a1` is another value set by SBI (a return value). We ignore it here.

## Say hi!

Now we can use this function in our `boot` function:

```rust [src/main.rs] {3-6}
fn main() -> ! {
    /* ... */
    print::sbi_putchar(b'H');
    print::sbi_putchar(b'i');
    print::sbi_putchar(b'!');
    print::sbi_putchar(b'\n');
    loop {}
}
```

```
$ ./run.sh
Boot HART MHPM Info       : 16 (0x0007fff8)
Boot HART Debug Triggers  : 2 triggers
Boot HART MIDELEG         : 0x0000000000001666
Boot HART MEDELEG         : 0x0000000000f0b509
Hi!
```

## `println!`

```rust [src/print.rs]
pub struct Printer;

impl core::fmt::Write for Printer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            sbi_putchar(byte);
        }
        Ok(())
    }
}

macro_rules! println {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = writeln!($crate::print::Printer, $($arg)*);
    }};
}
```

```rust [src/main.rs] {1}
#[macro_use]
mod print;
```

```rust [src/main.rs] {7}
fn main() -> ! {
    init_bss();
    println!("\nBooting hypervisor...");
    loop {}
}
```

```
$ ./run.sh
...
Booting hypervisor...
```

## Print on a panic

```rust [src/main.rs] {3}
#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic: {}", info);
    loop {}
}
```

## Trap handler

```rust [src/main.rs] {1}
mod trap;
```

```rust [src/trap.rs]
use core::arch::asm;

macro_rules! read_csr {
    ($csr:expr) => {{
        let mut value: u64;
        unsafe {
            ::core::arch::asm!(concat!("csrr {}, ", $csr), out(reg) value);
        }
        value
    }};
}

#[unsafe(link_section = ".text.stvec")]
pub fn trap_handler() -> ! {
    let scause = read_csr!("scause");
    let sepc = read_csr!("sepc");
    let stval = read_csr!("stval");
    let scause_str = match scause {
        0 => "instruction address misaligned",
        1 => "instruction access fault",
        2 => "illegal instruction",
        3 => "breakpoint",
        4 => "load address misaligned",
        5 => "load access fault",
        6 => "store/AMO address misaligned",
        7 => "store/AMO access fault",
        8 => "environment call from U/VU-mode",
        9 => "environment call from HS-mode",
        10 => "environment call from VS-mode",
        11 => "environment call from M-mode",
        12 => "instruction page fault",
        13 => "load page fault",
        15 => "store/AMO page fault",
        20 => "instruction guest-page fault",
        21 => "load guest-page fault",
        22 => "virtual instruction",
        23 => "store/AMO guest-page fault",
        0x8000_0000_0000_0000 => "user software interrupt",
        0x8000_0000_0000_0001 => "supervisor software interrupt",
        0x8000_0000_0000_0002 => "hypervisor software interrupt",
        0x8000_0000_0000_0003 => "machine software interrupt",
        0x8000_0000_0000_0004 => "user timer interrupt",
        0x8000_0000_0000_0005 => "supervisor timer interrupt",
        0x8000_0000_0000_0006 => "hypervisor timer interrupt",
        0x8000_0000_0000_0007 => "machine timer interrupt",
        0x8000_0000_0000_0008 => "user external interrupt",
        0x8000_0000_0000_0009 => "supervisor external interrupt",
        0x8000_0000_0000_000a => "hypervisor external interrupt",
        0x8000_0000_0000_000b => "machine external interrupt",
        _ => panic!("unknown scause: {:#x}", scause),
    };

    println!("trap handler: {} at {:#x} (stval={:#x})", scause_str, sepc, stval);

    // Idle loop: halt the computer.
    loop {
        unsafe {
            asm!("wfi"); // Wait for an interrupt (idle loop)
        }
    }
}
```

```ld [hypervisor.ld] {4-5}
    .text :{
        KEEP(*(.text.boot));
        *(.text .text.*);
        . = ALIGN(4);
        *(.text.stvec);
    }
```

```rust [src/main.rs] {7}
fn main() -> ! {
    unsafe {
        let bss_start = &raw mut __bss;
        let bss_size = (&raw mut __bss_end as usize) - (&raw mut __bss as usize);
        core::ptr::write_bytes(bss_start, 0, bss_size);

        asm!("csrw stvec, {}", in(reg) trap::trap_handler as usize);
    }
```

```

```

```
Booting hypervisor...
v = ['a', 'b', 'c']
trap handler: illegal instruction at 0x80201964 (stval=0x0)
```
