---
title: Hello from Guest
---

# Hello from Guest

Our goal in this chapter is to print a "Hello, world!" message from the guest.

To say hello from the guest, we need to introduce a `putchar` function, or more precisely, hypercall (*"hypervisor call"*), to the guest.

Hypercalls are similar to system calls (both use `ecall` instruction), but hypercalls are called from the guest mode. The hypercall handling flow is as follows:

1. The guest program calls `ecall` instruction with parameters in registers.
2. The CPU switches to the HS-mode, and jumps to the trap handler specified in the `stvec` CSR.
3. The trap handler saves the guest state, and then determines the cause of the trap (`ecall` from the guest) by reading the `scause` CSR.
4. After handling the hypercall, the trap handler restores the guest state, and goes back to the guest.

Doesn't sound mostly identical to the [system call handling flow](https://1000os.seiya.me/en/08-exception)?

## Hypercall interface

We learned that the hypercalls are similar to the system calls, but there is a missing piece: system calls define its interface (e.g. [Linux system calls](https://man7.org/linux/man-pages/man2/syscalls.2.html)). What's the equivalent of that in hypercalls?

The answer is SBI (Supervisor Binary Interface). Our hypervisor already uses it to print characters, and the same applies to the guest OS!

## SBI call from guest

Let's call the SBI's *"Console Putchar"* extension from the guest to print a character `A`:

```asm [guest.S]
.section .text
.global guest_boot
guest_boot:
    li a7, 1        # Extension ID: 1 (legacy console)
    li a6, 0        # Function ID: 0 (putchar)
    li a0, 'A'      # Parameter: 'A'
    ecall           # Call SBI (hypervisor)

halt:
    j halt          # Infinite loop
```

`ecall` triggers a trap to the hypervisor, and our trap handler should panic with `environment call from VS-mode`, that is, `ecall` from the guest kernel mode:

```
$ ./run.sh
Booting hypervisor...
map: 00100000 -> 80305000
panic: panicked at src/trap.rs:51:5:
trap handler: environment call from VS-mode at 0x100008 (stval=0x0)
```

## Introduce `VCpu` struct

Before implementing the hypercall, let's refactor the code to manage the guest state in a separate struct `VCpu`:

```rust [src/vcpu.rs]
use core::arch::asm;

use crate::{allocator::alloc_pages, guest_page_table::GuestPageTable};

#[derive(Debug, Default)]
pub struct VCpu {
    pub hstatus: u64,
    pub hgatp: u64,
    pub sstatus: u64,
    pub sepc: u64,
}

impl VCpu {
    pub fn new(table: &GuestPageTable, guest_entry: u64) -> Self {
        let mut hstatus: u64 = 0;
        hstatus |= 2 << 32; // VSXL: XLEN for VS-mode (64-bit)
        hstatus |= 1 << 7; // SPV: Supervisor Previous Virtualization mode

        let sstatus: u64 = 1 << 8; // SPP: Supervisor Previous Privilege mode (VS-mode)

        Self {
            hstatus,
            hgatp: table.hgatp(),
            sstatus,
            sepc: guest_entry,
            ..Default::default()
        }
    }

    pub fn run(&mut self) -> ! {
        unsafe {
            asm!(
                "csrw hstatus, {hstatus}",
                "csrw sstatus, {sstatus}",
                "csrw sscratch, {sscratch}",
                "csrw hgatp, {hgatp}",
                "csrw sepc, {sepc}",
                "sret",
                hstatus = in(reg) self.hstatus,
                sstatus = in(reg) self.sstatus,
                hgatp = in(reg) self.hgatp,
                sepc = in(reg) self.sepc,
                sscratch = in(reg) (self as *mut VCpu as usize),
            );
        }

        unreachable!();
    }
}
```

```rust [src/main.rs] {5-6}
fn main() -> ! {
    /* ... */
    table.map(guest_entry, kernel_memory as u64, PTE_R | PTE_W | PTE_X);

    let mut vcpu = VCpu::new(&table, guest_entry);
    vcpu.run();
}
```

No functional changes. Just refactoring a bit for the following sections!

## Saving the guest state

We already catch the `ecall` from the guest kernel mode, but before handling the hypercall, we need to implement the guest mode state saving.

Prepare the fields to save the guest mode state:

```rust [src/vcpu.rs] {6-37}
pub struct VCpu {
    pub hstatus: u64,
    pub hgatp: u64,
    pub sstatus: u64,
    pub sepc: u64,
    pub host_sp: u64,
    pub ra: u64,
    pub sp: u64,
    pub gp: u64,
    pub tp: u64,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub s0: u64,
    pub s1: u64,
    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub a7: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64,
}
```

In addition to general-purpose registers, `host_sp` is also added to the `VCpu` struct. As the name suggests, `host_sp` is the stack pointer for the hypervisor's trap handler stack.

When a VM-exit occurs, the stack pointer is still the guest's one, so we need to switch the stack before entering the Rust code.

Allocate the hypervisor's stack in the `VCpu::new` function:

```rust [src/vcpu.rs] {8-9,15}
    pub fn new(table: &GuestPageTable, guest_entry: u64) -> Self {
        let mut hstatus: u64 = 0;
        hstatus |= 2 << 32; // VSXL: XLEN for VS-mode (64-bit)
        hstatus |= 1 << 7; // SPV: Supervisor Previous Virtualization mode

        let sstatus: u64 = 1 << 8; // SPP: Supervisor Previous Privilege mode (VS-mode)

        let stack_size = 512 * 1024;
        let host_sp = alloc_pages(stack_size) as u64 + stack_size as u64;
        Self {
            hstatus,
            hgatp: table.hgatp(),
            sstatus,
            sepc: guest_entry,
            host_sp,
            ..Default::default()
        }
    }
```

Saving the guest state is done in the `trap_handler` function:

```rust [src/trap.rs] {1-2,5,7-48,53-84}
use core::{arch::naked_asm, mem::offset_of};
use crate::vcpu::VCpu;

#[unsafe(link_section = ".text.stvec")]
#[unsafe(naked)]
pub extern "C" fn trap_handler() -> ! {
    naked_asm!(
        // Swap a0 and sscratch.
        "csrrw a0, sscratch, a0",

        // a0 is now a pointer to a VCpu. Save registers except a0.
        "sd ra, {ra_offset}(a0)",
        "sd sp, {sp_offset}(a0)",
        "sd gp, {gp_offset}(a0)",
        "sd tp, {tp_offset}(a0)",
        "sd t0, {t0_offset}(a0)",
        "sd t1, {t1_offset}(a0)",
        "sd t2, {t2_offset}(a0)",
        "sd s0, {s0_offset}(a0)",
        "sd s1, {s1_offset}(a0)",
        "sd a1, {a1_offset}(a0)",
        "sd a2, {a2_offset}(a0)",
        "sd a3, {a3_offset}(a0)",
        "sd a4, {a4_offset}(a0)",
        "sd a5, {a5_offset}(a0)",
        "sd a6, {a6_offset}(a0)",
        "sd a7, {a7_offset}(a0)",
        "sd s2, {s2_offset}(a0)",
        "sd s3, {s3_offset}(a0)",
        "sd s4, {s4_offset}(a0)",
        "sd s5, {s5_offset}(a0)",
        "sd s6, {s6_offset}(a0)",
        "sd s7, {s7_offset}(a0)",
        "sd s8, {s8_offset}(a0)",
        "sd s9, {s9_offset}(a0)",
        "sd s10, {s10_offset}(a0)",
        "sd s11, {s11_offset}(a0)",
        "sd t3, {t3_offset}(a0)",
        "sd t4, {t4_offset}(a0)",
        "sd t5, {t5_offset}(a0)",
        "sd t6, {t6_offset}(a0)",

        // Restore a0 from sscratch, and save in to VCpu.
        "csrr t0, sscratch",
        "sd t0, {a0_offset}(a0)",

        // Switch to the hypervisor's stack.
        "ld sp, {host_sp_offset}(a0)",

        // a0 (first argument) is still the vcpu pointer here.
        "call {handle_trap}",
        handle_trap = sym handle_trap,
        host_sp_offset = const offset_of!(VCpu, host_sp),
        ra_offset = const offset_of!(VCpu, ra),
        sp_offset = const offset_of!(VCpu, sp),
        gp_offset = const offset_of!(VCpu, gp),
        tp_offset = const offset_of!(VCpu, tp),
        t0_offset = const offset_of!(VCpu, t0),
        t1_offset = const offset_of!(VCpu, t1),
        t2_offset = const offset_of!(VCpu, t2),
        s0_offset = const offset_of!(VCpu, s0),
        s1_offset = const offset_of!(VCpu, s1),
        a0_offset = const offset_of!(VCpu, a0),
        a1_offset = const offset_of!(VCpu, a1),
        a2_offset = const offset_of!(VCpu, a2),
        a3_offset = const offset_of!(VCpu, a3),
        a4_offset = const offset_of!(VCpu, a4),
        a5_offset = const offset_of!(VCpu, a5),
        a6_offset = const offset_of!(VCpu, a6),
        a7_offset = const offset_of!(VCpu, a7),
        s2_offset = const offset_of!(VCpu, s2),
        s3_offset = const offset_of!(VCpu, s3),
        s4_offset = const offset_of!(VCpu, s4),
        s5_offset = const offset_of!(VCpu, s5),
        s6_offset = const offset_of!(VCpu, s6),
        s7_offset = const offset_of!(VCpu, s7),
        s8_offset = const offset_of!(VCpu, s8),
        s9_offset = const offset_of!(VCpu, s9),
        s10_offset = const offset_of!(VCpu, s10),
        s11_offset = const offset_of!(VCpu, s11),
        t3_offset = const offset_of!(VCpu, t3),
        t4_offset = const offset_of!(VCpu, t4),
        t5_offset = const offset_of!(VCpu, t5),
        t6_offset = const offset_of!(VCpu, t6),
    );
}
```

TODO: `vsstatus` and its friends?

It's almost the same as [the exception handler in 1,000 lines OS](https://1000os.seiya.me/en/08-exception#exception-handler).

It expects `sscratch` to contain the pointer to the `VCpu` struct (we'll implement it later). `csrrw` is used to do following 2 operations at once:

- Restore the `VCpu` pointer from `sscratch` to `a0`
- Save the `a0` to `sscratch`

After saving the guest state, the stack pointer is switched to the hypervisor's stack by loading `host_sp` to `sp`.

## `Console Putchar` hypercall

Now we can implement the hypercall handler using the registers saved in the `VCpu` struct. According to [the SBI specification](https://github.com/riscv-non-isa/riscv-sbi-doc), the SBI expects the following conventions:

> - An `ECALL` is used as the control transfer instruction between the supervisor and the SEE.
> - `a7` encodes the SBI extension ID (EID),
> - `a6` encodes the SBI function ID (FID) for a given extension ID encoded in `a7` for any SBI extension defined in or after SBI v0.2.
> - All registers except `a0` & `a1` must be preserved across an SBI call by the callee.
> - SBI functions must return a pair of values in `a0` and `a1`, with `a0` returning an error code.

Let's implement the Console Putchar hypercall:

```rust [src/trap.rs] {1,4-7}
pub fn handle_trap(vcpu: *mut VCpu) -> ! {
    /* ... */

    let vcpu = unsafe { &mut *vcpu };
    if scause == 10 /* environment call from VS-mode */ {
        panic!("SBI call: eid={:#x}, fid={:#x}, a0={:#x} ('{}')", vcpu.a7, vcpu.a6, vcpu.a0, vcpu.a0 as u8 as char);
    }

    panic!("trap handler: {} at {:#x} (stval={:#x})", scause_str, sepc, stval);
}
```

You should see the following panic - printing a character `A` from the guest!

```
$ ./run.sh
Booting hypervisor...
map: 00100000 -> 80306000
panic: panicked at src/trap.rs:140:9:
SBI call: eid=0x1, fid=0x0, a0=0x41 ('A')
```

## Restoring the guest state

We've implemented the putchar call, but it doesn't go back to the guest. To do that, we need to restore the guest state after the hypercall.

It's simple. Load more registers from the `VCpu` struct when entering the guest mode:

```rust [src/vcpu.rs] {1,13-45,53-83}
use core::mem::offset_of;

impl VCpu {
    pub fn run(&mut self) -> ! {
        unsafe {
            asm!(
                "csrw hstatus, {hstatus}",
                "csrw sstatus, {sstatus}",
                "csrw sscratch, {sscratch}",
                "csrw hgatp, {hgatp}",
                "csrw sepc, {sepc}",

                // Restore general-purpose registers.
                "mv a0, {sscratch}",
                "ld ra, {ra_offset}(a0)",
                "ld sp, {sp_offset}(a0)",
                "ld gp, {gp_offset}(a0)",
                "ld tp, {tp_offset}(a0)",
                "ld t0, {t0_offset}(a0)",
                "ld t1, {t1_offset}(a0)",
                "ld t2, {t2_offset}(a0)",
                "ld s0, {s0_offset}(a0)",
                "ld s1, {s1_offset}(a0)",
                "ld a1, {a1_offset}(a0)",
                "ld a2, {a2_offset}(a0)",
                "ld a3, {a3_offset}(a0)",
                "ld a4, {a4_offset}(a0)",
                "ld a5, {a5_offset}(a0)",
                "ld a6, {a6_offset}(a0)",
                "ld a7, {a7_offset}(a0)",
                "ld s2, {s2_offset}(a0)",
                "ld s3, {s3_offset}(a0)",
                "ld s4, {s4_offset}(a0)",
                "ld s5, {s5_offset}(a0)",
                "ld s6, {s6_offset}(a0)",
                "ld s7, {s7_offset}(a0)",
                "ld s8, {s8_offset}(a0)",
                "ld s9, {s9_offset}(a0)",
                "ld s10, {s10_offset}(a0)",
                "ld s11, {s11_offset}(a0)",
                "ld t3, {t3_offset}(a0)",
                "ld t4, {t4_offset}(a0)",
                "ld t5, {t5_offset}(a0)",
                "ld t6, {t6_offset}(a0)",
                "ld a0, {a0_offset}(a0)",

                "sret",
                hstatus = in(reg) self.hstatus,
                sstatus = in(reg) self.sstatus,
                hgatp = in(reg) self.hgatp,
                sepc = in(reg) self.sepc,
                sscratch = in(reg) (self as *mut VCpu as usize),
                ra_offset = const offset_of!(VCpu, ra),
                sp_offset = const offset_of!(VCpu, sp),
                gp_offset = const offset_of!(VCpu, gp),
                tp_offset = const offset_of!(VCpu, tp),
                t0_offset = const offset_of!(VCpu, t0),
                t1_offset = const offset_of!(VCpu, t1),
                t2_offset = const offset_of!(VCpu, t2),
                s0_offset = const offset_of!(VCpu, s0),
                s1_offset = const offset_of!(VCpu, s1),
                a0_offset = const offset_of!(VCpu, a0),
                a1_offset = const offset_of!(VCpu, a1),
                a2_offset = const offset_of!(VCpu, a2),
                a3_offset = const offset_of!(VCpu, a3),
                a4_offset = const offset_of!(VCpu, a4),
                a5_offset = const offset_of!(VCpu, a5),
                a6_offset = const offset_of!(VCpu, a6),
                a7_offset = const offset_of!(VCpu, a7),
                s2_offset = const offset_of!(VCpu, s2),
                s3_offset = const offset_of!(VCpu, s3),
                s4_offset = const offset_of!(VCpu, s4),
                s5_offset = const offset_of!(VCpu, s5),
                s6_offset = const offset_of!(VCpu, s6),
                s7_offset = const offset_of!(VCpu, s7),
                s8_offset = const offset_of!(VCpu, s8),
                s9_offset = const offset_of!(VCpu, s9),
                s10_offset = const offset_of!(VCpu, s10),
                s11_offset = const offset_of!(VCpu, s11),
                t3_offset = const offset_of!(VCpu, t3),
                t4_offset = const offset_of!(VCpu, t4),
                t5_offset = const offset_of!(VCpu, t5),
                t6_offset = const offset_of!(VCpu, t6),
            );
        }
    }
}
```

Call the `VCpu::run` function after the hypercall:

```rust [src/trap.rs]
    if scause == 10 {
        println!("SBI call: eid={:#x}, fid={:#x}, a0={:#x} ('{}')", vcpu.a7, vcpu.a6, vcpu.a0, vcpu.a0 as u8 as char);
        vcpu.sepc = sepc + 4; // Resume the guest after ECALL instruction.
    } else {
        panic!("trap handler: {} at {:#x} (stval={:#x})", scause_str, sepc, stval);
    }

    vcpu.run();
```

Lastly, add few more calls in the guest to test the hypercall:

```asm [guest.S] {3-6}
    li a0, 'A'      # Parameter: 'A'
    ecall           # Call SBI (hypervisor)
    li a0, 'B'      # Parameter: 'B'
    ecall           # Call SBI (hypervisor)
    li a0, 'C'      # Parameter: 'C'
    ecall           # Call SBI (hypervisor)
```

If it's implemented correctly, following 3 characters should be printed:

```
$ ./run.sh
Booting hypervisor...
map: 00100000 -> 80306000
SBI call: eid=0x1, fid=0x0, a0=0x41 ('A')
SBI call: eid=0x1, fid=0x0, a0=0x42 ('B')
SBI call: eid=0x1, fid=0x0, a0=0x43 ('C')
```

Good job! We've implemented a hypercall, and also VM-exit handling is working!