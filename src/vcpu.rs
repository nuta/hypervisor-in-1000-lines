use core::{arch::asm, mem::offset_of};

use crate::{allocator::alloc_pages, guest_page_table::GuestPageTable};

#[derive(Debug, Default)]
pub struct VCpu {
    pub host_sp: u64,
    pub hstatus: u64,
    pub hgatp: u64,
    pub hedeleg: u64,
    pub sstatus: u64,
    pub sepc: u64,
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

impl VCpu {
    pub fn new(table: &GuestPageTable, guest_entry: u64) -> Self {
        let mut hstatus: u64 = 0;
        hstatus |= 2 << 32; // VSXL: XLEN for VS-mode (64-bit)
        hstatus |= 1 << 7; // SPV: Supervisor Previous Virtualization mode

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

        let sstatus: u64 = 1 << 8; // SPP: Supervisor Previous Privilege mode (VS-mode)

        let stack_size = 512 * 1024;
        let host_sp = alloc_pages(stack_size) as u64 + stack_size as u64;
        Self {
            hstatus,
            hgatp: table.hgatp(),
            hedeleg,
            sstatus,
            sepc: guest_entry,
            host_sp,
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
                "csrw hedeleg, {hedeleg}",
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
                hedeleg = in(reg) self.hedeleg,
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

        unreachable!();
    }
}
