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
