---
title: Guest Mode
---

# Guest Mode

Now we're ready to enjoy bare-metal programming in Rust. Let's start building hypervisor things.

The first thing is to enter the guest mode, which is the CPU mode of the guest operating system or *virtual machines*. CPU will keep running the guest program as usual, but when it requires a help from the hypervisor, it exits the guest mode (called *"VM exit"*) and jumps into our trap handler.

## HS-mode and VS-mode

TODO:

## Entering HS-mode

First we need to enable the hypervisor extension in QEMU.  Add `h=true` to the `-cpu` option:

```sh [run.sh] {1}
    -cpu rv64,h=true \
```

Here's the minimal code to enter the guest mode:

```rust [src/main.rs] {4-19}
fn main() -> ! {
    /* ... */

    let mut hstatus: u64 = 0;
    hstatus |= 2 << 32; // VSXL: XLEN for VS-mode (64-bit)
    hstatus |= 1 << 7; // SPV: Supervisor Previous Virtualization mode

    let sepc: u64 = 0x1234abcd;
    unsafe {
        asm!(
            "csrw hstatus, {hstatus}",
            "csrw sepc, {sepc}",
            "sret",
            hstatus = in(reg) hstatus,
            sepc = in(reg) sepc,
        );
    }

    unreachable!();
}
```

Isn't it very similar to what we've seen in [the user mode in OS in 1,000 Lines](https://1000os.seiya.me/en/13-user-mode#transition-to-user-mode)? RISC-V, a latecomer in the CPU world, has consistent design throughout the specification, whereas [x86-64 has more dedicated instructions](https://www.felixcloutier.com/x86/vmlaunch:vmresume) for the hardware-assisted virtualization. `sret` is not just for going back to the user mode, but also to the guest mode too.

In a nutshell:

- `hstatus` is the hypervisor status register: a set of flags and parameters that describe the CPU state.
- `sepc` is the program counter to be set when the CPU enters the guest mode.
- `sret`, a trap-return instruction, transitions the CPU into the guest mode according to the `hstatus`. In this case, the CPU will enter the S-mode (kernel mode) because `SPV` is set.

Since `sepc` points to a random unmapped address (`0x1234abcd`), the CPU will cause a *guest* page fault immediately after entering the guest mode:

```
$ ./run.sh
Booting hypervisor...
trap handler: instruction guest-page fault at 0x1234abcd (stval=0x1234abcd)
```

Congrats! You've just entered the guest mode for the first time.
