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
        for level in (1..=3).rev() {
            let entry = table.entry_by_addr(guest_paddr, level);
            if !entry.is_valid() {
                let new_table_ptr = Table::alloc();
                *entry = Entry::new(new_table_ptr as u64, PTE_V);
            }

            table = unsafe { &mut *(entry.paddr() as *mut Table) };
        }

        let entry = table.entry_by_addr(guest_paddr, 0);
        assert!(!entry.is_valid(), "already mapped");
        *entry = Entry::new(host_paddr, flags | PTE_V | PTE_U);
    }
}