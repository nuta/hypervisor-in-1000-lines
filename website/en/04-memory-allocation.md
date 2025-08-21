---
title: Memory Allocation
---

# Memory Allocation

In `no_std` environment, your daily drivers such as `String`, `Box`, `Vec` are not available. This is because they require dynamic memory allocation, which is optional in `no_std`.

In this chapter, we'll implement a simple bump allocator to enable those features.

## Bump allocation algorithm

Bump allocator is the simplest allocation algorithm. Imagine you have a Yule Log Cake. As people come to your house "memory allocator", you slice off a piece of the cake, and give it to them. Your knife cuts the cake from the top to the bottom, and you can't take it back.

In bump allocator, your have the next pointer (the knife's position), and the end pointer (the bottom of the cake). As it allocates memory, it moves the next pointer down until it reaches the end.

It's very simple, but the critical drawback is that you can't free memory. That being said, the simplicity makes it very fast, and it's sometimes used in real-world applications.

## Using a `no_std` crate

Before implementing the bump allocator, let me show you the power of Rust.

Rust is very nicely designed to be used in `no_std` environment and ... you can introduce third-party libraries (called "crates") to your project in a single command!

```
$ cargo add spin
```

That's it! `spin` crate implements `std::sync::Mutex` in the spinlock way. This allows us to make our bump allocator thread-safe. Check out [the documentation](https://docs.rs/spin/latest/spin/) to explore more APIs.

## Implement bump allocator

Let's implement the bump allocator.

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

- `BumpAllocator` implements `GlobalAlloc` trait, so it can be used as a global allocator.
- `GLOBAL_ALLOCATOR` is the global allocator instance. `#[global_allocator]` attribute tells Rust to use it as the default allocator.
- `Mutable` is an inner struct that contains the next and the end pointer, wrapped by `spin::Mutex` to make it thread-safe.

## Initialize the allocator

Next, define the heap area in the linker script and give the area to the allocator:

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

## Using `alloc` crate

Now you're ready to use the allocator! `std` crate's features that require dynamic memory allocation are available as `alloc` crate ([documentation](https://doc.rust-lang.org/alloc/)). Add `extern crate` to enable it:


```rust [src/main.rs] {4}
#![no_std]
#![no_main]

extern crate alloc;
```

> [!TIP]
>
> You do *not* need to add the crate to your `Cargo.toml` file. It's already bundled with the standard library.

Let's try it out:

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

> [!NOTE]
>
> You need to `use` objects from `alloc` crate explicitly.

```
$ ./run.sh
Booting hypervisor...
v = ['a', 'b', 'c']
```

It works!