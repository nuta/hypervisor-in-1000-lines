---
title: Memory-Mapped I/O
---

# Memory-Mapped I/O

> [!WARNING]
> This chapter is work in progress.

```rs [src/trap.rs] {2-1}
    match scause {
        21 /* load guest-page fault */ | 23 /* store/AMO guest-page fault */ => {
            let htinst = read_csr!("htinst");
            let htval = read_csr!("htval");
            assert!(htinst != 0);

            // "A guest physical address written to htval is shifted right by 2 bits"
            let guest_addr = (htval << 2) | (stval & 0b11);
            let is_write = scause == 23;
            let width  = match (htinst & 0x7f, (htinst >> 12) & 0x7) {
                (0x03, 0x0) | (0x03, 0x4) => 1, // lb, lbu
                (0x03, 0x1) | (0x03, 0x5) => 2, // lh, lhu
                (0x03, 0x2) | (0x03, 0x6) => 4, // lw, lwu
                (0x03, 0x3) => 8, // ld
                (0x23, 0x0) => 1, // sb
                (0x23, 0x1) => 2, // sh
                (0x23, 0x2) => 4, // sw
                (0x23, 0x3) => 8, // sd
                _ => 4,
            };

            if is_write {
                let rs2 = (htinst >> 20) & 0x1f;
                handle_mmio_write(vcpu, guest_addr, rs2, width);
            } else {
                let rd = (htinst >> 7) & 0x1f;
                handle_mmio_read(vcpu, guest_addr, rd, width);
            }

            let is_compressed = htinst & (1 << 1) == 0;
            let inst_len = if is_compressed { 2 } else { 4 };
            vcpu.sepc = sepc + inst_len;
        }
    }
```

```rs [src/trap.rs]
fn handle_mmio_write(vcpu: &mut VCpu, guest_addr: u64, reg: u64, width: u64) {
    let value = match reg {
        0 => 0, // x0 is hardwired to 0
        1 => vcpu.ra,
        2 => vcpu.sp,
        3 => vcpu.gp,
        4 => vcpu.tp,
        5 => vcpu.t0,
        6 => vcpu.t1,
        7 => vcpu.t2,
        8 => vcpu.s0,
        9 => vcpu.s1,
        10 => vcpu.a0,
        11 => vcpu.a1,
        12 => vcpu.a2,
        13 => vcpu.a3,
        14 => vcpu.a4,
        15 => vcpu.a5,
        16 => vcpu.a6,
        17 => vcpu.a7,
        18 => vcpu.s2,
        19 => vcpu.s3,
        20 => vcpu.s4,
        21 => vcpu.s5,
        22 => vcpu.s6,
        23 => vcpu.s7,
        24 => vcpu.s8,
        25 => vcpu.s9,
        26 => vcpu.s10,
        27 => vcpu.s11,
        28 => vcpu.t3,
        29 => vcpu.t4,
        30 => vcpu.t5,
        31 => vcpu.t6,
        _ => unreachable!(),
    };

    println!("[MMIO]: write {:#x} (value={:#x}, width={})", guest_addr, value, width);
}
```

```rs [src/trap.rs]
fn handle_mmio_read(vcpu: &mut VCpu, guest_addr: u64, reg: u64, width: u64) {
    println!("[MMIO]: read {:#x} (width={})", guest_addr, width);
    let value = 0; // dummy value
    match reg {
        0 => {}, // x0 is hardwired to 0, writes are ignored
        1 => vcpu.ra = value,
        2 => vcpu.sp = value,
        3 => vcpu.gp = value,
        4 => vcpu.tp = value,
        5 => vcpu.t0 = value,
        6 => vcpu.t1 = value,
        7 => vcpu.t2 = value,
        8 => vcpu.s0 = value,
        9 => vcpu.s1 = value,
        10 => vcpu.a0 = value,
        11 => vcpu.a1 = value,
        12 => vcpu.a2 = value,
        13 => vcpu.a3 = value,
        14 => vcpu.a4 = value,
        15 => vcpu.a5 = value,
        16 => vcpu.a6 = value,
        17 => vcpu.a7 = value,
        18 => vcpu.s2 = value,
        19 => vcpu.s3 = value,
        20 => vcpu.s4 = value,
        21 => vcpu.s5 = value,
        22 => vcpu.s6 = value,
        23 => vcpu.s7 = value,
        24 => vcpu.s8 = value,
        25 => vcpu.s9 = value,
        26 => vcpu.s10 = value,
        27 => vcpu.s11 = value,
        28 => vcpu.t3 = value,
        29 => vcpu.t4 = value,
        30 => vcpu.t5 = value,
        31 => vcpu.t6 = value,
        _ => unreachable!(),
    }
}
```

```
$ ./run.sh
...
[MMIO]: read 0xc002080 (width=4)
[MMIO]: write 0xc002080 (value=0x0, width=4)
[MMIO]: write 0xc000004 (value=0x1, width=4)
[MMIO]: read 0xc002080 (width=4)
[MMIO]: write 0xc002080 (value=0x0, width=4)
[MMIO]: write 0xc000008 (value=0x1, width=4)
[MMIO]: read 0xc002080 (width=4)
[MMIO]: write 0xc002080 (value=0x0, width=4)
[MMIO]: write 0xc00000c (value=0x1, width=4)
[MMIO]: write 0xc201000 (value=0x0, width=4)
[guest] [    0.165692] riscv-plic: plic@c000000: mapped 3 interrupts with 1 handlers for 2 contexts.
[guest] [    0.171317] printk: legacy console [hvc0] enabled
[guest] [    0.171317] printk: legacy console [hvc0] enabled
[guest] [    0.173931] printk: legacy bootconsole [sbi0] disabled
[guest] [    0.173931] printk: legacy bootconsole [sbi0] disabled
[guest] [    0.185119] loop: module loaded
[guest] [    0.187542] NET: Registered PF_INET6 protocol family
[guest] [    0.195298] Segment Routing with IPv6
[guest] [    0.196536] In-situ OAM (IOAM) with IPv6
[guest] [    0.197967] sit: IPv6, IPv4 and MPLS over IPv4 tunneling driver
[guest] [    0.212801] clk: Disabling unused clocks
panic: panicked at src/trap.rs:132:13:
unknown SBI call: eid=0x2, fid=0x0
```

Fortunately Linux does not care about our broken (or rather, unimplemented) MMIO access to PLIC.

## Implement `sbi_console_getchar`

```rs [src/trap.rs] {3-4}
        // Get machine vendor/arch/implementation ID
        (0x10, 0x4 | 0x5 | 0x6) => Ok(0),
        // Console Getchar.
        (0x2, 0x0) => Err(-1), // Not supported
        // Console Putchar.
        (0x1, 0x0) => {
```


```
$ ./run.sh
[guest] [    0.239914] /dev/root: Can't open blockdev
[guest] [    0.241510] VFS: Cannot open root device "" or unknown-block(0,0): error -6
[guest] [    0.243153] Please append a correct "root=" boot option; here are the available partitions:
[guest] [    0.245138] List of all bdev filesystems:
[guest] [    0.246140]  squashfs
[guest] [    0.246166]  fuseblk
[guest] [    0.246815] 
[guest] [    0.247996] Kernel panic - not syncing: VFS: Unable to mount root fs on unknown-block(0,0)
[guest] [    0.250162] CPU: 0 UID: 0 PID: 1 Comm: swapper/0 Not tainted 6.12.34 #1
[guest] [    0.251802] Hardware name: riscv-virtio (DT)
[guest] [    0.253300] Call Trace:
[guest] [    0.254189] [<ffffffff80004650>] dump_backtrace+0x1c/0x24
[guest] [    0.256018] [<ffffffff8036b2ba>] show_stack+0x2c/0x38
[guest] [    0.257297] [<ffffffff8037234c>] dump_stack_lvl+0x52/0x74
[guest] [    0.258636] [<ffffffff80372382>] dump_stack+0x14/0x1c
[guest] [    0.259885] [<ffffffff8036b44e>] panic+0xf8/0x2e0
[guest] [    0.261119] [<ffffffff8037bbe8>] mount_root_generic+0x1a4/0x21a
[guest] [    0.262592] [<ffffffff8037bd9e>] mount_root+0x140/0x15c
[guest] [    0.263895] [<ffffffff8037bf58>] prepare_namespace+0x19e/0x1d8
[guest] [    0.265308] [<ffffffff8037b70e>] kernel_init_freeable+0x1b6/0x1d0
[guest] [    0.266769] [<ffffffff80373ade>] kernel_init+0x1a/0xe2
[guest] [    0.267996] [<ffffffff8037a82a>] ret_from_exception_end+0xe/0x18
```
