---
title: Boot
---

# Boot

In this chapter, we'll write a minimal kernel for RISC-V, to be used as the base for our hypervisor.

The kernel will be written in Rust, but it's still very similar to what you'd write in C (see [OS in 1000 lines](https://operating-system-in-1000-lines.vercel.app/en/04-boot)). Enjoy comparing the two and notice the power of Rust.

## OpenSBI

The QEMU `virt` machine, the virtual computer we use in this book, does not boot our hypervisor directly. Instead, it starts OpenSBI, a firmware similar to BIOS/UEFI.

## Let's boot OpenSBI

First, let's see how OpenSBI starts. Create a shell script named `run.sh` as follows:

```
$ touch run.sh
$ chmod +x run.sh
```

```bash [run.sh]
#!/bin/bash
set -xue

# QEMU file path
QEMU=qemu-system-riscv32

# Start QEMU
$QEMU -machine virt -bios default -nographic -serial mon:stdio --no-reboot
```

QEMU takes various options to start the virtual machine. Here are the options used in the script:

- `-machine virt`: Start a `virt` machine. You can check other supported machines with the `-machine '?'` option.
- `-bios default`: Use the default firmware (OpenSBI in this case).
- `-nographic`: Start QEMU without a GUI window.
- `-serial mon:stdio`: Connect QEMU's standard input/output to the virtual machine's serial port. Specifying `mon:` allows switching to the QEMU monitor by pressing <kbd>Ctrl</kbd>+<kbd>A</kbd> then <kbd>C</kbd>.
- `--no-reboot`: If the virtual machine crashes, stop the emulator without rebooting (useful for debugging).

> [!TIP]
>
> In macOS, you can check the path to Homebrew's QEMU with the following command:
>
> ```
> $ ls $(brew --prefix)/bin/qemu-system-riscv32
> /opt/homebrew/bin/qemu-system-riscv32
> ```

Run the script and you will see the following banner:

```
$ ./run.sh

OpenSBI v1.2
   ____                    _____ ____ _____
  / __ \                  / ____|  _ \_   _|
 | |  | |_ __   ___ _ __ | (___ | |_) || |
 | |  | | '_ \ / _ \ '_ \ \___ \|  _ < | |
 | |__| | |_) |  __/ | | |____) | |_) || |_
  \____/| .__/ \___|_| |_|_____/|____/_____|
        | |
        |_|

Platform Name             : riscv-virtio,qemu
Platform Features         : medeleg
Platform HART Count       : 1
Platform IPI Device       : aclint-mswi
Platform Timer Device     : aclint-mtimer @ 10000000Hz
...
```

OpenSBI displays the OpenSBI version, platform name, features, number of HARTs (CPU cores), and more for debugging purposes.

When you press any key, nothing will happen. This is because QEMU's standard input/output is connected to the virtual machine's serial port, and the characters you type are being sent to the OpenSBI. However, no one reads the input characters.

Press <kbd>Ctrl</kbd>+<kbd>A</kbd> then <kbd>C</kbd> to switch to the QEMU debug console (QEMU monitor). You can exit QEMU by `q` command in the monitor:

```
QEMU 8.0.2 monitor - type 'help' for more information
(qemu) q
```

> [!TIP]
>
> <kbd>Ctrl</kbd>+<kbd>A</kbd> has several features besides switching to the QEMU monitor (<kbd>C</kbd> key). For example, pressing the <kbd>X</kbd> key will immediately exit QEMU.
>
> ```
> C-a h    print this help
> C-a x    exit emulator
> C-a s    save disk data back to file (if -snapshot)
> C-a t    toggle console timestamps
> C-a b    send break (magic sysrq)
> C-a c    switch between console and monitor
> C-a C-a  sends C-a
> ```

## Scaffold a new Rust project

Now, let's create a new Rust project:

```
$ cargo init --bin hypervisor  
    Creating binary (application) package
note: see more `Cargo.toml` keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
```

Create a `rust-toolchain.toml` file to specify the toolchain we need. With this file, Cargo will automatically install the correct toolchain:

```toml [rust-toolchain.toml]
[toolchain]
channel = "stable"
targets = ["riscv64gc-unknown-none-elf"]
```

> [!TIP]
>
> If you're familiar with bare-metal programming in Rust, you might be wondering *"stable? not nightly?"*.
>
> And the answer is ... yes, you can use stable toolchain! Bare-metal Rust used to require nightly unstable features, but thanks to the Rust team's effort, everything we need is already stabilized.

## Minimal boot code

Finally we can write some Rust code. Unfortunately, we don't have a way to print anything yet, so let it be an infinite loop for now:

```rs [src/main.rs]
#![no_std]
#![no_main]

use core::arch::asm;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub extern "C" fn boot() -> ! {
    unsafe {
        asm!(
            "la sp, __stack_top",  // Load __stack_top address into sp
            "j {main}",            // Jump to main
            main = sym main,       // Defines {main} in the assembly code
            options(noreturn)      // No return from this function
        );
    }
}


unsafe extern "C" {
    static mut __bss: u8;
    static mut __bss_end: u8;
}

fn main() -> ! {
    // Fill the BSS section with zeros.
    unsafe {
        let bss_start = &raw mut __bss;
        let bss_size = (&raw mut __bss_end as usize) - (&raw mut __bss as usize);
        core::ptr::write_bytes(bss_start, 0, bss_size);
    }

    // Infinite loop.
    loop {}
}
```

## Linker script

A linker script is a file which defines the memory layout of executable files. Based on the layout, the linker assigns memory addresses to functions and variables.

Let's create a new file named `hypervisor.ld`:

```
ENTRY(boot)

SECTIONS {
    . = 0x80200000;

    .text :{
        KEEP(*(.text.boot)); // Keep `boot` function at the beginning
        *(.text .text.*);
    }

    .rodata : ALIGN(4) {
        *(.rodata .rodata.*);
    }

    .data : ALIGN(4) {
        *(.data .data.*);
    }

    .bss : ALIGN(4) {
        __bss = .;
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }

    . = ALIGN(16);
    . += 1024 * 1024; /* 1MB */
    __stack_top = .;

    /DISCARD/ : {
        *(.eh_frame);
    }
}
```

> [!WARNING]
>
> This script is slightly different from the one in [OS in 1000 lines](https://github.com/nuta/operating-system-in-1000-lines/blob/main/kernel.ld). `.eh_frame` is explicitly discarded.

Here are the key points of the linker script:

- The entry point of the kernel is the `boot` function.
- The base address is `0x80200000`.
- The `.text.boot` section is always placed at the beginning.
- Each section is placed in the order of `.text`, `.rodata`, `.data`, and `.bss`.
- The kernel stack comes after the `.bss` section, and its size is 128KB.

`.text`, `.rodata`, `.data`, and `.bss` sections mentioned here are data areas with specific roles:

| Section   | Description                                                  |
| --------- | ------------------------------------------------------------ |
| `.text`   | This section contains the code of the program.               |
| `.rodata` | This section contains constant data that is read-only.       |
| `.data`   | This section contains read/write data.                       |
| `.bss`    | This section contains read/write data with an initial value of zero. |

Let's take a closer look at the syntax of the linker script. First, `ENTRY(boot)` declares that the `boot` function is the entry point of the program. Then, the placement of each section is defined within the `SECTIONS` block.

The `*(.text .text.*)` directive places the `.text` section and any sections starting with `.text.` from all files (`*`) at that location.

The `.` symbol represents the current address. It automatically increments as data is placed, such as with `*(.text)`. The statement `. += 128 * 1024` means "advance the current address by 128KB". The `ALIGN(4)` directive ensures that the current address is adjusted to a 4-byte boundary.

Finally, `__bss = .` assigns the current address to the symbol `__bss`. In C language, you can refer to a defined symbol using `extern char symbol_name`.

## Build and run

Create a shell script to build and run the hypervisor:

```sh [run.sh]
#!/bin/sh
set -ev

RUSTFLAGS="-C link-arg=-Thypervisor.ld -C linker=rust-lld" \
  cargo build --bin hypervisor --target riscv64gc-unknown-none-elf

cp target/riscv64gc-unknown-none-elf/debug/hypervisor hypervisor.elf

qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -bios default \
    -smp 1 \
    -m 128M \
    -nographic \
    -d cpu_reset,unimp,guest_errors,int -D qemu.log \
    -serial mon:stdio \
    --no-reboot \
    -kernel hypervisor.elf
```

Don't forget to make the script executable:

```
$ chmod +x run.sh
```

Run the script:

```
$ ./run.sh 

RUSTFLAGS="-C link-arg=-Thypervisor.ld -C linker=rust-lld" \
  cargo build --bin hypervisor --target riscv64gc-unknown-none-elf
   Compiling hypervisor v0.1.0 (/Users/seiya/dev/hypervisor-in-1000-lines)
error: `#[panic_handler]` function required, but not found

error: could not compile `hypervisor` (bin "hypervisor") due to 1 previous error
```

Oops, we got a compile error.

## Panic handler

The error message says that we need a `#[panic_handler]` function. A panic handler is called when a panic occurs. For example:

- Explicitly calling `panic!` macro.
- Out-of-bounds access in a slice.
- `unwrap`-ing a `None` value.

Here's the minimal panic handler implementation:

```rs [src/main.rs]
use core::panic::PanicInfo;

#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi"); // Wait for an interrupt (idle loop)
        }
    }
}
```

## Build and run again

Let's build and run the hypervisor again:

```
$ ./run.sh
```

You'll observe ... the silence. This is because `main` function is an infinite loop without doing anything.

```
QEMU 10.0.0 monitor - type 'help' for more information
(qemu) info registers

CPU#0
 V      =   0
 pc       0000000080200010
...
```

`pc` points to 0x80200010. According to `llvm-objdump`, it's running the `main` function. Yay!

```
$ llvm-objdump -C -d hypervisor.elf                 

hypervisor.elf: file format elf64-littleriscv

Disassembly of section .text:

0000000080200000 <boot>:
80200000: 00100117      auipc   sp, 0x100
80200004: 01410113      addi    sp, sp, 0x14
80200008: 0060006f      j       0x8020000e <hypervisor::main::h296982a60bc361ad>
8020000c: 0000          unimp

000000008020000e <hypervisor::main::h296982a60bc361ad>:
8020000e: a009          j       0x80200010 <hypervisor::main::h296982a60bc361ad+0x2>
80200010: a001          j       0x80200010 <hypervisor::main::h296982a60bc361ad+0x2> <-- here!
80200012: 0000          unimp
```
