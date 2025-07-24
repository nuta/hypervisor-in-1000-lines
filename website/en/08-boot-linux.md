---
title: Boot Linux
---

# Boot Linux

## Loading Linux kernel image

```rs [src/linux_loader.rs]
use crate::allocator::alloc_pages;
use crate::guest_page_table::{GuestPageTable, PTE_R, PTE_W, PTE_X};
use core::mem::size_of;

#[repr(C)]
struct RiscvImageHeader {
    code0: u32,
    code1: u32,
    text_offset: u64,
    image_size: u64,
    flags: u64,
    version: u32,
    reserved1: u32,
    reserved2: u64,
    magic: u64,
    magic2: u32,
    reserved3: u32,
}

pub const GUEST_BASE_ADDR: u64 = 0x8000_0000;
const MEMORY_SIZE: usize = 64 * 1024 * 1024;

fn copy_and_map(
    table: &mut GuestPageTable,
    data: &[u8],
    guest_addr: u64,
    len: usize,
    flags: u64,
) {
    // Allocate a memory region, and copy the data to it.
    assert!(data.len() <= len, "data is beyond the region");
    let raw_ptr = alloc_pages(len);
    unsafe {
        core::ptr::copy_nonoverlapping(data.as_ptr(), raw_ptr, data.len());
    }

    // Map the memory region to the guest's address space.
    let host_addr = raw_ptr as u64;
    for off in (0..len).step_by(4096) {
        table.map(guest_addr + off as u64, host_addr + off as u64, flags);
    }
}

pub fn load_linux_kernel(table: &mut GuestPageTable, image: &[u8]) {
    assert!(image.len() >= size_of::<RiscvImageHeader>());
    let header = unsafe { &*(image.as_ptr() as *const RiscvImageHeader) };
    assert_eq!(u32::from_le(header.magic2), 0x05435352, "invalid magic");

    let kernel_size = u64::from_le(header.image_size);
    assert!(image.len() <= MEMORY_SIZE);

    copy_and_map(
        table,
        image,
        GUEST_BASE_ADDR,
        MEMORY_SIZE,
        PTE_R | PTE_W | PTE_X,
    );

    println!("loaded kernel: size={}KB", kernel_size / 1024);
}
```

```rs [src/main.rs] {5-9}
fn main() -> ! {
    /* ... */

    let mut table = GuestPageTable::new();
    let kernel_image = include_bytes!("../linux/Image");
    linux_loader::load_linux_kernel(&mut table, kernel_image);
    let mut vcpu = VCpu::new(&table, GUEST_BASE_ADDR);
    vcpu.a0 = 0; // hart ID
    vcpu.a1 = 0xdeadbeef_00000000; // device tree address (TODO)
    vcpu.run();
}
```

```
Booting hypervisor...
loaded kernel: size=5658KB
panic: panicked at src/trap.rs:143:9:
trap handler: load guest-page fault at 0x803aa976 (stval=0xdeadbeef00000004)
```

## Device tree

```toml [Cargo.toml] {2}
[dependencies]
vm-fdt = { version = "0.3.0", features = ["alloc"], default-features = false }
```


```rs [src/linux_loader.rs]
use alloc::vec::Vec;
use alloc::format;

fn build_device_tree() -> Result<Vec<u8>, vm_fdt::Error> {
    let mut fdt = vm_fdt::FdtWriter::new()?;
    let root_node = fdt.begin_node("")?;
    fdt.property_string("compatible", "riscv-virtio")?;
    fdt.property_u32("#address-cells", 0x2)?;
    fdt.property_u32("#size-cells", 0x2)?;

    let chosen_node = fdt.begin_node("chosen")?;
    fdt.property_string("bootargs", "console=hvc earlycon=sbi panic=-1")?;
    fdt.end_node(chosen_node)?;

    let memory_node = fdt.begin_node(format!("memory@{}", GUEST_BASE_ADDR))?;
    fdt.property_string("device_type", "memory")?;
    fdt.property_array_u64("reg", &[GUEST_BASE_ADDR, MEMORY_SIZE as u64])?;
    fdt.end_node(memory_node)?;

    let cpus_node = fdt.begin_node("cpus")?;
    fdt.property_u32("#address-cells", 0x1)?;
    fdt.property_u32("#size-cells", 0x0)?;
    fdt.property_u32("timebase-frequency", 10000000)?;

    let cpu_node = fdt.begin_node("cpu@0")?;
    fdt.property_string("device_type", "cpu")?;
    fdt.property_string("compatible", "riscv")?;
    fdt.property_u32("reg", 0)?;
    fdt.property_string("status", "okay")?;
    fdt.property_string("mmu-type", "riscv,sv48")?;
    fdt.property_string("riscv,isa", "rv64imafdc")?;

    fdt.end_node(cpu_node)?;
    fdt.end_node(cpus_node)?;
    fdt.end_node(root_node)?;
    fdt.finish()
}
```

```rs [src/linux_loader.rs] {1,12-14}
pub const GUEST_DTB_ADDR: u64 = 0x7000_0000;

pub fn load_linux_kernel(table: &mut GuestPageTable, image: &[u8]) {
    assert!(image.len() >= size_of::<RiscvImageHeader>());
    let header = unsafe { &*(image.as_ptr() as *const RiscvImageHeader) };
    assert_eq!(u32::from_le(header.magic2), 0x05435352, "invalid magic");

    let kernel_size = u64::from_le(header.image_size);
    assert!(image.len() <= MEMORY_SIZE);
    copy_and_map(table, image, GUEST_BASE_ADDR, MEMORY_SIZE, PTE_R | PTE_W | PTE_X);

    let dtb = build_device_tree().unwrap();
    assert!(dtb.len() <= 0x10000, "DTB is too large");
    copy_and_map(table, &dtb, GUEST_DTB_ADDR, dtb.len(), PTE_R);

    println!("loaded kernel: size={}KB", kernel_size / 1024);
}
```

```rs [src/main.rs] {1,8}
use crate::linux_loader::GUEST_DTB_ADDR;

fn main() -> ! {
    /* ... */
    linux_loader::load_linux_kernel(&mut table, kernel_image);
    let mut vcpu = VCpu::new(&table, GUEST_BASE_ADDR);
    vcpu.a0 = 0; // hart ID
    vcpu.a1 = GUEST_DTB_ADDR; // device tree address
    vcpu.run();
 }
```

```
Booting hypervisor...
loaded kernel: size=5658KB
panic: panicked at src/trap.rs:143:9:
trap handler: instruction page fault at 0x80000088 (stval=0x80000088)
```

## Debugging a page fault in guest

```
$ llvm-objdump -d linux/vmlinux
...
ffffffff80000040 <relocate_enable_mmu>:
...
ffffffff80000084: 18051073      csrw    satp, a0
ffffffff80000088: 00000517      auipc   a0, 0x0      <-- page fault here!
```

The page fault occurs at the instruction right after setting `satp`, that is, updating the page table.

```
$ llvm-addr2line -e linux/vmlinux 0xffffffff80000088    
/kernel/arch/riscv/kernel/head.S:110
```

[elixir.bootlin.com](https://elixir.bootlin.com/linux/v6.12.34/source/arch/riscv/kernel/head.S#L95-L106)

```
	/*
	 * Load trampoline page directory, which will cause us to trap to
	 * stvec if VA != PA, or simply fall through if VA == PA.  We need a
	 * full fence here because setup_vm() just wrote these PTEs and we need
	 * to ensure the new translations are in use.
	 */
	la a0, trampoline_pg_dir
	XIP_FIXUP_OFFSET a0
	srl a0, a0, PAGE_SHIFT
	or a0, a0, a1
	sfence.vma
	csrw CSR_SATP, a0
```

Interesting. This page fault is not a bug, but is caused intentionally by Linux.

## Delegate page faults to guest

```rs [src/vcpu.rs] {3,10-22,26,36,38}
pub struct VCpu {
    /* ... */
    pub hedeleg: u64,
}

impl VCpu {
    pub fn new(table: &GuestPageTable, guest_entry: u64) -> Self {
        /* ... */

        let mut hedeleg: u64 = 0;
        hedeleg |= 1 << 0; // Instruction address misaligned
        hedeleg |= 1 << 1; // Instruction access fault
        hedeleg |= 1 << 2; // Illegal instruction
        hedeleg |= 1 << 3; // Breakpoint
        hedeleg |= 1 << 4; // Load address misaligned
        hedeleg |= 1 << 5; // Load access fault
        hedeleg |= 1 << 6; // Store/AMO address misaligned
        hedeleg |= 1 << 7; // Store/AMO access fault
        hedeleg |= 1 << 8; // Environment call from U-mode
        hedeleg |= 1 << 12; // Instruction page fault
        hedeleg |= 1 << 13; // Load page fault
        hedeleg |= 1 << 15; // Store/AMO page fault

        Self {
            /* ... */
            hedeleg,
        }
    }

    pub fn run(&mut self) -> ! {
        unsafe {
            asm!(
                /* ... */
                "csrw hgatp, {hgatp}",
                "csrw sepc, {sepc}",
                "csrw hedeleg, {hedeleg}",
                /* ... */
                hedeleg = in(reg) self.hedeleg,
            );
        }
    }
}
```

```
$ ./run.sh
Booting hypervisor...
loaded kernel: size=5658KB
SBI call: eid=0x10, fid=0x0, a0=0x0 ('')
SBI call: eid=0x1, fid=0x0, a0=0x5b ('[')
SBI call: eid=0x1, fid=0x0, a0=0x20 (' ')
SBI call: eid=0x1, fid=0x0, a0=0x20 (' ')
SBI call: eid=0x1, fid=0x0, a0=0x20 (' ')
SBI call: eid=0x1, fid=0x0, a0=0x20 (' ')
SBI call: eid=0x1, fid=0x0, a0=0x30 ('0')
SBI call: eid=0x1, fid=0x0, a0=0x2e ('.')
SBI call: eid=0x1, fid=0x0, a0=0x30 ('0')
SBI call: eid=0x1, fid=0x0, a0=0x30 ('0')
SBI call: eid=0x1, fid=0x0, a0=0x30 ('0')
SBI call: eid=0x1, fid=0x0, a0=0x30 ('0')
SBI call: eid=0x1, fid=0x0, a0=0x30 ('0')
SBI call: eid=0x1, fid=0x0, a0=0x30 ('0')
SBI call: eid=0x1, fid=0x0, a0=0x5d (']')
SBI call: eid=0x1, fid=0x0, a0=0x20 (' ')
SBI call: eid=0x1, fid=0x0, a0=0x4c ('L')
SBI call: eid=0x1, fid=0x0, a0=0x69 ('i')
SBI call: eid=0x1, fid=0x0, a0=0x6e ('n')
SBI call: eid=0x1, fid=0x0, a0=0x75 ('u')
SBI call: eid=0x1, fid=0x0, a0=0x78 ('x')
...
```
