use core::alloc::{GlobalAlloc, Layout};

struct Mutable {
    next: usize,
    end: usize,
}

pub struct BumpAllocator {
    mutable: spin::Mutex<Option<Mutable>>,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            mutable: spin::Mutex::new(None),
        }
    }

    pub fn init(&self, start: *mut u8, end: *mut u8) {
        self.mutable.lock().replace(Mutable {
            next: start as usize,
            end: end as usize,
        });
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut mutable_lock = self.mutable.lock();
        let mutable = mutable_lock.as_mut().expect("allocator not initialized");

        let addr = mutable.next.next_multiple_of(layout.align());
        assert!(addr.saturating_add(layout.size()) <= mutable.end, "out of memory");

        mutable.next = addr + layout.size();
        addr as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
pub static GLOBAL_ALLOCATOR: BumpAllocator = BumpAllocator::new();

pub fn alloc_pages(len: usize) -> *mut u8 {
    let layout = Layout::from_size_align(len, 0x1000).unwrap();
    unsafe { GLOBAL_ALLOCATOR.alloc_zeroed(layout) as *mut u8 }
}
