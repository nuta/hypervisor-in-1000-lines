---
title: Supervisor Binary Interface
---

# Supervisor Binary Interface

> [!WARNING]
> This chapter is work in progress.

SBI is a RISC-V's standard interface for operating systems to interact with the firmware and hypervisor. You can think of it as the so-called BIOS/UEFI in x86-64.

## Refactor the SBI handler

Before implementing SBI calls, let's refactor the existing SBI handler, which only handles the `console_putchar` call:

```rs [src/trap.rs]
fn handle_sbi_call(vcpu: &mut VCpu) {
    let eid = vcpu.a7;
    let fid = vcpu.a6;
    let result: Result<i64, i64> = match (eid, fid) {
        // Console Putchar.
        (0x1, 0x0) => {
            let ch = vcpu.a0 as u8;
            println!("[guest] {}", ch as char);
            Ok(0)
        }
        _ => {
            panic!("unknown SBI call: eid={:#x}, fid={:#x}", eid, fid);
        }
    };


    match result {
        Ok(value) => {
            vcpu.a0 = 0;
            vcpu.a1 = value as u64;
        }
        Err(err) => {
            vcpu.a0 = err as u64;
        }
    }
}
```

```rs [src/trap.rs] {2-4,5}
    let vcpu = unsafe { &mut *vcpu };
    match scause {
        10 /* environment call from VS-mode */ => {
            handle_sbi_call(vcpu);
            vcpu.sepc = sepc + 4;
        }
        _ => panic!("trap handler: {} at {:#x} (stval={:#x})", scause_str, sepc, stval),
    }
```

```
Booting hypervisor...
loaded kernel: size=5658KB
panic: panicked at src/trap.rs:121:13:
unknown SBI call: eid=0x10, fid=0x0
```

It's broken, but this is all right. `handle_sbi_call` is now more strict and panics on unknown SBI calls.

## Implement `sbi_get_spec_version`

```rs [src/trap.rs] {2-3}
    let result: Result<i64, i64> = match (eid, fid) {
        // Get SBI specification version
        (0x10, 0x0) => Ok(0),
        // Console Putchar.
        (0x1, 0x0) => {
            let ch = vcpu.a0 as u8;
            println!("[guest] {}", ch as char);
            Ok(0)
        }
    };
```

```
Booting hypervisor...
loaded kernel: size=5658KB
[guest] [
[guest]  
[guest]  
[guest]  
[guest]  
[guest] 0
[guest] .
[guest] 0
[guest] 0
[guest] 0
[guest] 0
[guest] 0
[guest] 0
[guest] ]
[guest]  
[guest] L
[guest] i
[guest] n
[guest] u
[guest] x
```

## Implement `sbi_console_putchar` (again)

We've implemented the `sbi_get_spec_version` call, and got serial output from the guest as before, but it's hard to read.

Let's buffer the console output and print it in a single line.

```rs [src/trap.rs] {1-2,4,15-22}
use alloc::vec::Vec;
use spin::Mutex;

static CONSOLE_BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());

fn handle_sbi_call(vcpu: &mut VCpu) {
    let eid = vcpu.a7;
    let fid = vcpu.a6;
    let result: Result<i64, i64> = match (eid, fid) {
        // Get SBI specification version
        (0x10, 0x0) => Ok(0),
        // Console Putchar.
        (0x1, 0x0) => {
            let ch = vcpu.a0 as u8;
            let mut buffer = CONSOLE_BUFFER.lock();
            if ch == b'\n' {
                let output = core::str::from_utf8(&buffer).unwrap_or("(not utf-8)");
                println!("[guest] {}", output);
                buffer.clear();
            } else {
                buffer.push(ch);
            }
            Ok(0)
        }
        _ => {
            panic!("unknown SBI call: eid={:#x}, fid={:#x}", eid, fid);
        }
    };
```

```
$ ./run.sh
Booting hypervisor...
loaded kernel: size=5658KB
[guest] [    0.000000] Linux version 6.12.34 (root@bef674f92d71) (riscv64-linux-gnu-gcc (Ubuntu 13.3.0-6ubuntu2~24.04) 13.3.0, GNU ld (GNU Binutils for Ubuntu) 2.42) #1 SMP Wed Jul 23 11:12:13 UTC 2025
[guest] [    0.000000] Machine model: riscv-virtio
[guest] [    0.000000] SBI specification v0.1 detected
[guest] [    0.000000] earlycon: sbi0 at I/O port 0x0 (options '')
[guest] [    0.000000] printk: legacy bootconsole [sbi0] enabled
[guest] [    0.000000] OF: reserved mem: Reserved memory: No reserved-memory node in the DT
[guest] [    0.000000] Zone ranges:
[guest] [    0.000000]   DMA32    [mem 0x0000000080000000-0x0000000083ffffff]
[guest] [    0.000000]   Normal   empty
[guest] [    0.000000] Movable zone start for each node
[guest] [    0.000000] Early memory node ranges
[guest] [    0.000000]   node   0: [mem 0x0000000080000000-0x0000000083ffffff]
[guest] [    0.000000] Initmem setup node 0 [mem 0x0000000080000000-0x0000000083ffffff]
panic: panicked at src/trap.rs:123:13:
unknown SBI call: eid=0x10, fid=0x3
```

## Implement `sbi_probe_extension`

```rs [src/trap.rs] {4-5}
    let result: Result<i64, i64> = match (eid, fid) {
        // Get SBI specification version
        (0x10, 0x0) => Ok(0),
        // Probe SBI extension
        (0x10, 0x3) => Err(-1),
        // Console Putchar.
        (0x1, 0x0) => {
```

```
$ ./run.sh
...
panic: panicked at src/trap.rs:125:13:
unknown SBI call: eid=0x10, fid=0x4
```

## Implement `sbi_get_m{vector,arch,imp}id`

```rs [src/trap.rs] {6-7}
    let result: Result<i64, i64> = match (eid, fid) {
        // Get SBI specification version
        (0x10, 0x0) => Ok(0),
        // Probe SBI extension
        (0x10, 0x3) => Err(-1),
        // Get machine vendor/arch/implementation ID
        (0x10, 0x4 | 0x5 | 0x6) => Ok(0),
        // Console Putchar.
        (0x1, 0x0) => {
```

```
$ ./run.sh
...
[guest] [    0.000000] Kernel panic - not syncing: No interrupt controller found.
[guest] [    0.000000] CPU: 0 UID: 0 PID: 0 Comm: swapper/0 Not tainted 6.12.34 #1
[guest] [    0.000000] Hardware name: riscv-virtio (DT)
[guest] [    0.000000] Call Trace:
[guest] [    0.000000] [<ffffffff80004650>] dump_backtrace+0x1c/0x24
[guest] [    0.000000] [<ffffffff8036b2ba>] show_stack+0x2c/0x38
[guest] [    0.000000] [<ffffffff8037234c>] dump_stack_lvl+0x52/0x74
[guest] [    0.000000] [<ffffffff80372382>] dump_stack+0x14/0x1c
[guest] [    0.000000] [<ffffffff8036b44e>] panic+0xf8/0x2e0
[guest] [    0.000000] [<ffffffff8037ce04>] init_IRQ+0x88/0xba
[guest] [    0.000000] [<ffffffff8037b334>] start_kernel+0x360/0x428
```

## Define PLIC

Linux complains that it did not find any interrupt controller. Let's define a PLIC device in the device tree so that the kernel can recognize it. This is off-topic and not related to SBI, but it's a trivial thing to do:

```rs [src/linux_loader.rs]
pub const PLIC_ADDR: u64 = 0x0c00_0000;
pub const PLIC_END: u64 = PLIC_ADDR + 0x400000;
```

```rs [src/linux_loader.rs] {4-9,14-22}
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
    fdt.property_array_u64("reg", &[PLIC_ADDR 0x4000000])?;
    fdt.property_u32("riscv,ndev", 3)?;
    fdt.property_array_u32("interrupts-extended", &[1, 11, 1, 9])?;
    fdt.property_phandle(2)?;
    fdt.end_node(plic_node)?;

    fdt.end_node(root_node)?;
    fdt.finish()
}
```

```
$ ./run.sh
...
panic: panicked at src/trap.rs:188:14:
trap handler: virtual instruction at 0xffffffff8022674e (stval=0xc0102573)
```

## Enable counters

```
$ llvm-addr2line -e linux/vmlinux 0xffffffff8022674e
/kernel/./arch/riscv/include/asm/timex.h:53 (discriminator 1)
```

[](https://elixir.bootlin.com/linux/v6.12.34/source/arch/riscv/include/asm/timex.h#L53)

```c
static inline cycles_t get_cycles(void)
{
	return csr_read(CSR_TIME);
}
```

```rs [src/trap.rs] {3}
                "csrw hgatp, {hgatp}",
                "csrw hedeleg, {hedeleg}",
                "csrw hcounteren, {hcounteren}",
                "csrw sepc, {sepc}",
```

```rs [src/trap.rs] {3}
                hgatp = in(reg) self.hgatp,
                hedeleg = in(reg) self.hedeleg,
                hcounteren = in(reg) 0b11, /* cycle and time */
                sepc = in(reg) self.sepc,
```

```
$ ./run.sh
...
panic: panicked at src/trap.rs:127:13:
unknown SBI call: eid=0x0, fid=0x0
```

## Implement `sbi_set_timer`

```rs [src/trap.rs] {2-6}
    let result: Result<i64, i64> = match (eid, fid) {
        // Set Timer
        (0x00, 0x0) => {
            println!("[sbi] WARN: set_timer is not implemented, ignoring");
            Ok(0)
        }
```

```
$ ./run.sh
...
panic: panicked at src/trap.rs:193:14:
trap handler: load guest-page fault at 0xffffffff801d7d78 (stval=0xff20000000002080)
```
