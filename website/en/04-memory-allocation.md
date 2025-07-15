---
title: Memory allocation
---

# Memory allocation

## Bump allocator

```
$ cargo add spin
```

```rust [src/main.rs]
mod allocator;
```

```rust [src/allocator.rs]
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
```

```rust [src/allocator.rs]
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
```

```txt [hypervisor.ld] {4-6}
    . += 1024 * 1024; /* 1MB */
    __stack_top = .;

    __heap = .;
    . += 100 * 1024 * 1024; /* 100MB */
    __heap_end = .;

    /DISCARD/ : {
        *(.eh_frame);
    }
```


```rust [src/main.rs] {4-5,12}
unsafe extern "C" {
    static mut __bss: u8;
    static mut __bss_end: u8;
    static mut __heap: u8;
    static mut __heap_end: u8;
}

fn main() -> ! {
    init_bss();
    println!("\nBooting hypervisor...");

    allocator::GLOBAL_ALLOCATOR.init(&raw mut __heap, &raw mut __heap_end);
```

```rust [src/main.rs] {4}
#![no_std]
#![no_main]

extern crate alloc;
```

```rust [src/main.rs] {4,6-10}
fn main() -> ! {
    /* ... */

    allocator::GLOBAL_ALLOCATOR.init(&raw mut __heap, &raw mut __heap_end);

    let mut v = alloc::vec::Vec::new();
    v.push('a');
    v.push('b');
    v.push('c');
    println!("v = {:?}", v);

    loop {}
}
```

```
$ ./run.sh
Booting hypervisor...
v = ['a', 'b', 'c']
```