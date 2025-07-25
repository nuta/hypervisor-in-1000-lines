use crate::{guest_memory::{DTB_MEMORY, GUEST_MEMORY}, guest_page_table::{GuestPageTable, PTE_R, PTE_W, PTE_X}};
use alloc::vec::Vec;
use alloc::format;
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

pub const GUEST_DTB_ADDR: u64 = 0x7000_0000;
pub const GUEST_BASE_ADDR: u64 = 0x8000_0000;
pub const GUEST_PLIC_ADDR: u64 = 0x0c00_0000;
pub const GUEST_VIRTIO_BLK_ADDR: u64 = 0x1000_0000;
pub const MEMORY_SIZE: usize = 64 * 1024 * 1024;

pub fn load_linux_kernel(table: &mut GuestPageTable, image: &[u8]) {
    assert!(image.len() >= size_of::<RiscvImageHeader>());
    
    let mut header_bytes = [0u8; size_of::<RiscvImageHeader>()];
    header_bytes.copy_from_slice(&image[..size_of::<RiscvImageHeader>()]);
    let header = unsafe { &*(header_bytes.as_ptr() as *const RiscvImageHeader) };
    
    assert_eq!(u32::from_le(header.magic2), 0x05435352, "invalid magic");

    let kernel_size = u64::from_le(header.image_size);
    assert!(image.len() <= MEMORY_SIZE);
    GUEST_MEMORY.write(table, image, PTE_R | PTE_W | PTE_X);

    let dtb = build_device_tree().unwrap();
    DTB_MEMORY.write(table, &dtb, PTE_R);

    println!("loaded kernel: size={}KB", kernel_size / 1024);
}

fn build_device_tree() -> Result<Vec<u8>, vm_fdt::Error> {
    let mut fdt = vm_fdt::FdtWriter::new()?;
    let root_node = fdt.begin_node("")?;
    fdt.property_string("compatible", "riscv-virtio")?;
    fdt.property_u32("#address-cells", 0x2)?;
    fdt.property_u32("#size-cells", 0x2)?;

    let chosen_node = fdt.begin_node("chosen")?;
    fdt.property_string("bootargs", "console=hvc earlycon=sbi panic=-1 root=/dev/vda init=/bin/catsay")?;
    fdt.end_node(chosen_node)?;

    let memory_node = fdt.begin_node(&format!("memory@{}", GUEST_BASE_ADDR))?;
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

    let intc_node = fdt.begin_node("interrupt-controller")?;
    fdt.property_u32("#interrupt-cells", 1)?;
    fdt.property_null("interrupt-controller")?;
    fdt.property_string("compatible", "riscv,cpu-intc")?;
    fdt.property_phandle(1)?;
    fdt.end_node(intc_node)?;

    fdt.end_node(cpu_node)?;
    fdt.end_node(cpus_node)?;

    let plic_node = fdt.begin_node("plic@c000000")?;
    fdt.property_string("compatible", "riscv,plic0")?;
    fdt.property_u32("#interrupt-cells", 1)?;
    fdt.property_null("interrupt-controller")?;
    fdt.property_array_u64("reg", &[GUEST_PLIC_ADDR, 0x4000000])?;
    fdt.property_u32("riscv,ndev", 3)?;
    fdt.property_array_u32("interrupts-extended", &[1, 11, 1, 9])?;
    fdt.property_phandle(2)?;
    fdt.end_node(plic_node)?;

    let virtio_node = fdt.begin_node("virtio_mmio@10000000")?;
    fdt.property_string("compatible", "virtio,mmio")?;
    fdt.property_array_u64("reg", &[GUEST_VIRTIO_BLK_ADDR, 0x1000])?;
    fdt.property_u32("interrupt-parent", 2)?;
    fdt.property_array_u32("interrupts", &[1])?;
    fdt.end_node(virtio_node)?;

    fdt.end_node(root_node)?;
    fdt.finish()
}
