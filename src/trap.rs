use core::{arch::naked_asm, mem::offset_of};
use alloc::vec::Vec;
use spin::Mutex;

use crate::vcpu::VCpu;

macro_rules! read_csr {
    ($csr:expr) => {{
        let mut value: u64;
        unsafe {
            ::core::arch::asm!(concat!("csrr {}, ", $csr), out(reg) value);
        }
        value
    }};
}

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

static CONSOLE_BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());

fn handle_sbi_call(vcpu: &mut VCpu) {
    let eid = vcpu.a7;
    let fid = vcpu.a6;
    let result: Result<i64, i64> = match (eid, fid) {
        // Get SBI specification version
        (0x10, 0x0) => Ok(0),
        // Probe SBI extension
        (0x10, 0x3) => Err(-1),
        // Get machine vendor/arch/implementation ID
        (0x10, 0x4 | 0x5 | 0x6) => Ok(0),
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

pub fn handle_trap(vcpu: *mut VCpu) -> ! {
    let scause = read_csr!("scause");
    let sepc = read_csr!("sepc");
    let stval = read_csr!("stval");
    let scause_str = match scause {
        0 => "instruction address misaligned",
        1 => "instruction access fault",
        2 => "illegal instruction",
        3 => "breakpoint",
        4 => "load address misaligned",
        5 => "load access fault",
        6 => "store/AMO address misaligned",
        7 => "store/AMO access fault",
        8 => "environment call from U/VU-mode",
        9 => "environment call from HS-mode",
        10 => "environment call from VS-mode",
        11 => "environment call from M-mode",
        12 => "instruction page fault",
        13 => "load page fault",
        15 => "store/AMO page fault",
        20 => "instruction guest-page fault",
        21 => "load guest-page fault",
        22 => "virtual instruction",
        23 => "store/AMO guest-page fault",
        0x8000_0000_0000_0000 => "user software interrupt",
        0x8000_0000_0000_0001 => "supervisor software interrupt",
        0x8000_0000_0000_0002 => "hypervisor software interrupt",
        0x8000_0000_0000_0003 => "machine software interrupt",
        0x8000_0000_0000_0004 => "user timer interrupt",
        0x8000_0000_0000_0005 => "supervisor timer interrupt",
        0x8000_0000_0000_0006 => "hypervisor timer interrupt",
        0x8000_0000_0000_0007 => "machine timer interrupt",
        0x8000_0000_0000_0008 => "user external interrupt",
        0x8000_0000_0000_0009 => "supervisor external interrupt",
        0x8000_0000_0000_000a => "hypervisor external interrupt",
        0x8000_0000_0000_000b => "machine external interrupt",
        _ => panic!("unknown scause: {:#x}", scause),
    };

    let vcpu = unsafe { &mut *vcpu };
    match scause {
        10 /* environment call from VS-mode */ => {
            handle_sbi_call(vcpu);
            vcpu.sepc = sepc + 4;
        }
        _ => panic!("trap handler: {} at {:#x} (stval={:#x})", scause_str, sepc, stval),
    }

    vcpu.run();
}
