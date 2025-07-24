use alloc::collections::BTreeSet;
use spin::Mutex;


pub static PLIC: Mutex<Plic> = Mutex::new(Plic::new());

#[derive(Debug)]
pub struct Plic {
    irq_pending: BTreeSet<u16>,
}

impl Plic {
    pub const fn new() -> Self {
        Self { irq_pending: BTreeSet::new() }
    }

    pub fn handle_write(&mut self, offset: u64, value: u64, width: u64) {
        match (offset, width) {
            (0x0000..0x1000, 4) => {},  // Priority
            (0x2000..0x200000, 4) => {},  // Enable bits
            (0x200000..0x4000000, 4) if (offset & 0xfff) == 0 => {},  // Priority threshold
            (0x200000..0x4000000, 4) if (offset & 0xfff) == 4 => {
                // IRQ claim/completion
                if value == 1 {
                    self.irq_pending.remove(&1);
                }
            }
            _ => println!("[PLIC]: unknown write offset={:#x} (value={:#x}, width={})", offset, value, width),
        }
    }

    pub fn handle_read(&self, offset: u64, width: u64) -> u64 {
        match (offset, width) {
            (0x2000..0x200000, 4) => 0,  // Enable bits
            (0x200000..0x4000000, 4) if (offset & 0xfff) == 4 => {
                // IRQ claim - return the pending IRQ number
                self.irq_pending.iter().next().copied().unwrap_or(0) as u64
            }
            _ => {
                println!("[PLIC]: unknown read offset={:#x} (width={})", offset, width);
                0
            }
        }
    }

    pub fn add_pending_irq(&mut self, irq: u16) {
        self.irq_pending.insert(irq);
    }

    pub fn has_pending_irqs(&self) -> bool {
        !self.irq_pending.is_empty()
    }
}
