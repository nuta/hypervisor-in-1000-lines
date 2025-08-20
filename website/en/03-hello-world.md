---
title: Hello World
---

# Hello World

> [!WARNING]
> This chapter is work in progress.

In the previous chapter, we wrote a minimal boot code in Rust. In this chapter, we'll implement `println!` macro to make it easier to print debug messages. It will be the key debugging tool for us.

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

You should see the very first Hello World message from the hypervisor!

```
$ ./run.sh
Boot HART MHPM Info       : 16 (0x0007fff8)
Boot HART Debug Triggers  : 2 triggers
Boot HART MIDELEG         : 0x0000000000001666
Boot HART MEDELEG         : 0x0000000000f0b509
Hi!
```

## `println!`

Printing a character is a giant step for our productivity, but it's still not enough. In normal Rust programs, we use `println!` macro to print not just a single character, but a string, variables, and more in a human-readable format.

However, the macro is not available in `no_std` environment. That said, implementing it is simple:

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

- Our own `println!` macro is defined. It uses `writeln!` macro ([documentation](https://doc.rust-lang.org/std/macro.writeln.html)) internally, which takes a writer object and format arguments.
- `Printer` implements the writer object required by `writeln!`, and it just prints each character to the serial port.

That's it! Let's try it out:

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

You'll see this charming boot message:

```
$ ./run.sh
...
Booting hypervisor...
```

## Print on a panic

We've finished the title of this chapter "Hello World", but let's do some more things for better debugging experience.

The first thing is to print something on a panic. Currently, our panic handler is just a loop. Since it does not output anything, it's not obvious why it panicked.

To dump the details of a panic, we can just use `println!` macro:

```rust [src/main.rs] {3}
#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic: {}", info);
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
```

## Trap handler

> [!TIP]
>
> For more details about traps, read [OS in 1,000 Lines](https://operating-system-in-1000-lines.vercel.app/en/08-exception).

Another nice-to-have thing is to print something on a trap, which happens when CPU encounters an event that OS needs to handle, such as invalid memory access.

Let's implement a trap handler that prints the details of a trap:

```rust [src/main.rs] {1}
mod trap;
```

```rust [src/trap.rs]
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

    panic!("trap handler: {} at {:#x} (stval={:#x})", scause_str, sepc, stval);
}
```

```txt [hypervisor.ld] {4-5}
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

- `stvec` CSR holds the base address of the trap handler.
- `read_csr!` is a macro that reads a CSR. You can use `asm!` directly, but it's a bit verbose.
- `scause` CSR holds the cause of a trap.
- `sepc` CSR holds the program counter at the time of a trap.
- `stval` CSR holds the trap-specific value, such as the virtual address of a page fault.
- It uses the linker script to place the trap handler at an aligned address. This is because lower bits of `stvec` are used for the trap handler mode.

Let's test the implementation by causing a trap intentionally by adding `unimp` pseudo instruction after setting `stvec`:

```rust [src/main.rs] {8}
fn main() -> ! {
    unsafe {
        let bss_start = &raw mut __bss;
        let bss_size = (&raw mut __bss_end as usize) - (&raw mut __bss as usize);
        core::ptr::write_bytes(bss_start, 0, bss_size);

        asm!("csrw stvec, {}", in(reg) trap::trap_handler as usize);
        asm!("unimp"); // Illegal instruction here!
    }
```

Let's try it:

```
Booting hypervisor...
trap handler: illegal instruction at 0x80201964 (stval=0x0)
```

The trap handler is called and we can see the details of the trap. By using `llvm-addr2line`, we can find the line number of the panic in the source code.

```
$ llvm-addr2line -e hypervisor.elf 0x80201964
/Users/seiya/dev/hypervisor-in-1000-lines/src/main.rs:54
```

Looks correct!
