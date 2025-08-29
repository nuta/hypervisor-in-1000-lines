---
title: Guest Page Table
---

# Guest Page Table

The next step is to map the guest memory space to run some code in the guest.

## Guest-physical vs. Host-physical addresses

With hardware-assisted virtualization enabled, RISC-V CPU (and other modern CPUs) has 4 address spaces:

- Guest-virtual address (new)
- Guest-physical address (new)
- Host-virtual address
- Host-physical address (so-called *physical memory space*)

In the host mode, the memory addresses will be translated from host-virtual to host-physical addresses using the page table in the host.

However, when the CPU is in the guest mode, the address translation will be done in 2 stages:

- **Stage 1**: Guest-virtual → guest-physical (page table in `satp`)
- **Stage 2**: Guest-physical → host-physical (guest page table in `hgatp`)

This isolation is what allows multiple VMs to run simultaneously - each guest thinks it has exclusive access to physical memory, but the hypervisor secretly maps them to different host memory regions.

## Page allocator

Page allocator is a memory allocator that allocates memory in pages, where each *page* is a fixed-size memory region. In most cases, it's 4KiB (4096 bytes).

> [!TIP]
>
> RISC-V and other modern CPUs support bigger page sizes.

In this book, we'll use the global allocator for simplicity. Construct a memory request to the allocator `layout` with the required length and alignment:

```rust [src/allocator.rs]
pub fn alloc_pages(len: usize) -> *mut u8 {
    debug_assert!(len % 4096 == 0, "len must be a multiple of 4096");
    let layout = Layout::from_size_align(len, 4096).unwrap();
    unsafe { GLOBAL_ALLOCATOR.alloc_zeroed(layout) as *mut u8 }
}
```

## Building a guest page table

Let's build a guest page table. While it's a different data structure, it's mostly identical to the page table in the user mode we learned in [OS in 1,000 Lines](https://1000os.seiya.me/en/11-page-table):

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

- `Entry` represents a page table entry. It contains the next-level table or the physical page number (PPN) in host memory.
- `Table` represents a page table in each level. It has 512 entries of `Entry` (8 bytes each), that is `512 * 8 = 4096` bytes. It fits into a 4KiB page nicely!
- `GuestPageTable` is a guest page table. It contains the pointer to the top-level page table.
- `GuestPageTable::map` maps a guest-physical address to a host-physical address. It traverses the page table levels from the top to the bottom, and allocates a new intermediate page table if it's not present, and finally updates the leaf entry.

## Loading a guest kernel

We're ready to fill the guest memory space! But what should we put in the guest memory? Let's start with a minimal boot code which just infinitely loops:

```asm [guest.S]
.section .text
.global guest_boot
guest_boot:
    j guest_boot
```

Since using Rust is a bit tricky, we'll use Clang and LLVM:

```sh [run.sh] {6-11}
#!/bin/sh
set -ev

# macOS users need to install "llvm" in Homebrew:
export PATH="$(brew --prefix llvm)/bin:$PATH"

clang \
    -Wall -Wextra --target=riscv32-unknown-elf -ffreestanding -nostdlib \
    -Wl,-eguest_boot -Wl,-Ttext=0x100000 -Wl,-Map=guest.map \
    guest.S -o guest.elf

llvm-objcopy -O binary guest.elf guest.bin

RUSTFLAGS="-C link-arg=-Thypervisor.ld -C linker=rust-lld" \
  cargo build --bin hypervisor --target riscv64gc-unknown-none-elf
```

`llvm-objcopy` coverts the ELF file to a raw binary file (`guest.bin`) that can be loaded into the guest memory as-is. Load it using `include_bytes!` and map it into the guest page table:

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

Let's try it:

```
$ ./run.sh
Booting hypervisor...
map: 00100000 -> 80305000
```

It doesn't cause a guest page fault anymore! However, it's unclear if it's working because it doesn't print anything.

The quick way to check if it's working is to use QEMU's monitor command:

```
QEMU 10.0.0 monitor - type 'help' for more information
(qemu) info registers

CPU#0
 V      =   1
 pc       0000000000100000
```

As you can see, the program counter is stuck at `0x100000` in the guest mode (`V=1`). Yay!