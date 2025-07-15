---
title: Guest Mode
---

# Guest Mode

## Entering HS-mode

Add `h=true` to the `-cpu` option.

```sh [run.sh] {1}
    -cpu rv64,h=true \
```


```rust [src/main.rs] {4-19}
fn main() -> ! {
    /* ... */

    let mut hstatus: u64 = 0;
    hstatus |= 2u64 << 32; // VSXL: XLEN for VS-mode (64-bit)
    hstatus |= 1u64 << 7; // SPV: Supervisor Previous Virtualization mode (HS-mode)

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

```
$ ./run.sh
Booting hypervisor...
trap handler: instruction guest-page fault at 0x1234abcd (stval=0x1234abcd)
```

## Page allocator

```rust [src/allocator.rs]
pub fn alloc_pages(len: usize) -> *mut u8 {
    let layout = Layout::from_size_align(len, 0x1000).unwrap();
    unsafe { GLOBAL_ALLOCATOR.alloc_zeroed(layout) as *mut u8 }
}
```

## Guest page table

```rust [src/guest_page_table.rs]
use core::mem::size_of;

pub const PTE_R: u64 = 1 << 1; /* Readable */
pub const PTE_W: u64 = 1 << 2; /* Writable */
pub const PTE_X: u64 = 1 << 3; /* Executable */
const PTE_V: u64 = 1 << 0; /* Valid */
const PTE_U: u64 = 1 << 4; /* User */
const PPN_SHIFT: usize = 12;
const PTE_PPN_SHIFT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct Entry(u64);

impl Entry {
    pub fn new(paddr: u64, flags: u64) -> Self {
        let ppn = (paddr as u64) >> PPN_SHIFT;
        Self(ppn << PTE_PPN_SHIFT | flags)
    }

    pub fn is_valid(&self) -> bool {
        self.0 & PTE_V != 0
    }

    pub fn paddr(&self) -> u64 {
        (self.0 >> PTE_PPN_SHIFT) << PPN_SHIFT
    }
}

#[repr(transparent)]
struct Table([Entry; 512]);

impl Table {
    pub fn alloc() -> *mut Table {
        crate::allocator::alloc_pages(size_of::<Table>()) as *mut Table
    }

    pub fn entry_by_addr(&mut self, guest_paddr: u64, level: usize) -> &mut Entry {
        let index = (guest_paddr >> (12 + 9 * level)) & 0x1ff; // extract 9-bits index
        &mut self.0[index as usize]
    }
}

pub struct GuestPageTable {
    table: *mut Table,
}

impl GuestPageTable {
    pub fn new() -> Self {
        Self {
            table: Table::alloc(),
        }
    }

    pub fn hgatp(&self) -> u64 {
        (9u64 << 60/* Sv48x4 */) | (self.table as u64 >> PPN_SHIFT)
    }

    pub fn map(&mut self, guest_paddr: u64, host_paddr: u64, flags: u64) {
        let mut table = unsafe { &mut *self.table };
        for level in (1..=3).rev() { // level = 3, 2, 1
            let entry = table.entry_by_addr(guest_paddr, level);
            if !entry.is_valid() {
                let new_table_ptr = Table::alloc();
                *entry = Entry::new(new_table_ptr as u64, PTE_V);
            }

            table = unsafe { &mut *(entry.paddr() as *mut Table) };
        }

        let entry = table.entry_by_addr(guest_paddr, 0);
        println!("map: {:08x} -> {:08x}", guest_paddr, host_paddr);
        assert!(!entry.is_valid(), "already mapped");
        *entry = Entry::new(host_paddr, flags | PTE_V | PTE_U);
    }
}
```

## Loading a guest kernel

```asm [guest.S]
.section .text
.global guest_boot
guest_boot:
    j guest_boot
```

```sh [run.sh] {4-7}
#!/bin/sh
set -ev

$(brew --prefix llvm)/bin/clang \
    -Wall -Wextra --target=riscv32-unknown-elf -ffreestanding -nostdlib \
    -Wl,-eguest_boot -Wl,-Ttext=0x100000 -Wl,-Map=guest.map \
    guest.S -o guest.elf

$(brew --prefix llvm)/bin/llvm-objcopy -O binary guest.elf guest.bin

RUSTFLAGS="-C link-arg=-Thypervisor.ld -C linker=rust-lld" \
  cargo build --bin hypervisor --target riscv64gc-unknown-none-elf
```

```rust [src/main.rs] {1-4,9-18,20-22,31,35-36}
use crate::{
    allocator::alloc_pages,
    guest_page_table::{GuestPageTable, PTE_R, PTE_W, PTE_X},
};

fn main() -> ! {
    /* ... */

    let kernel_image = include_bytes!("../guest.bin");
    let guest_entry = 0x100000;

    // Copy guest kernel to a guest memory buffer.
    let kernel_memory = alloc_pages(kernel_image.len());
    unsafe {
        let dst = kernel_memory as *mut u8;
        let src = kernel_image.as_ptr();
        core::ptr::copy_nonoverlapping(src, dst, kernel_image.len());
    }

    // Map the guest memory into the guest page table.
    let mut table = GuestPageTable::new();
    table.map(guest_entry, kernel_memory as u64, PTE_R | PTE_W | PTE_X);

    let mut hstatus = 0;
    hstatus |= 2u64 << 32; // VSXL: XLEN for VS-mode (64-bit)
    hstatus |= 1u64 << 7; // SPV: Supervisor Previous Virtualization mode (HS-mode)

    unsafe {
        asm!(
            "csrw hstatus, {hstatus}",
            "csrw hgatp, {hgatp}",
            "csrw sepc, {sepc}",
            "sret",
            hstatus = in(reg) hstatus,
            hgatp = in(reg) table.hgatp(),
            sepc = in(reg) guest_entry,
        );
    }
```

```
$ ./run.sh
Booting hypervisor...
map: 00100000 -> 80305000
```

```
QEMU 10.0.0 monitor - type 'help' for more information
(qemu) info registers

CPU#0
 V      =   1
 pc       0000000000100000
```
