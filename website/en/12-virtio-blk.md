
## Build the root filesystem image

```go [catsay.go]
package main

import (
	"fmt"
	"time"
)

func main() {
	// ASCII art cat saying "Hello World!" with colors.
	fmt.Println()
	fmt.Println("\033[33m     /\\_/\\  \033[0m")
	fmt.Println("\033[33m    ( \033[36mo.o\033[33m ) \033[0m")
	fmt.Println("\033[33m     > ^ <\033[0m")
	fmt.Println()
	fmt.Println("\033[32m   Hello World!\033[0m")
	fmt.Println()

	for {
		time.Sleep(1 * time.Second)
	}
}
```

```bash [linux/build.sh]
# Build rootfs with catsay
rm -rf rootfs rootfs.squashfs
mkdir -p rootfs/dev # auto mounted by CONFIG_DEVTMPFS_MOUNT
mkdir -p rootfs/bin
GOOS=linux GOARCH=riscv64 go build -o rootfs/bin/catsay catsay.go
mksquashfs rootfs/ rootfs.squashfs -comp xz -b 1M -no-xattrs -noappend
```

## Virtio-blk config

```
$ ./run.sh
...
[virtio-blk] MMIO read at 0xfc
panic: panicked at src/virtio_blk.rs:78:18:
unknown virtio-blk mmio read: guest_addr=0xfc
```

```rs [src/virtio_blk.rs] {1-2,9-14}
pub const DISK_IMAGE: &[u8] = include_bytes!("../linux/rootfs.squashfs");
pub const DISK_CAPACITY: u64 = DISK_IMAGE.len() as u64 / 512;

impl VirtioBlk  {
    pub fn handle_mmio_read(&self, offset: u64, width: u64) -> u64 {
        /* ... */
        match offset {
            /* ... */
            // Config generation
            0xfc => 0x0,
            // Device-specific config: capacity (low 32 bits)
            0x100 => DISK_CAPACITY & 0xffffffff,
            // Device-specific config: capacity (high 32 bits)
            0x104 => DISK_CAPACITY >> 32,
            _ => panic!("unknown virtio-blk mmio read: guest_addr={:#x}", offset),
```

```
$ ./run.sh
...
[virtio-blk] MMIO read at 0xfc
[virtio-blk] MMIO read at 0x100
[virtio-blk] MMIO read at 0x104
[virtio-blk] MMIO read at 0xfc
[guest] [    0.212838] virtio_blk virtio0: [vda] 1920 512-byte logical blocks (983 kB/960 KiB)
[virtio-blk] MMIO read at 0x70
[virtio-blk] MMIO write at 0x70
[virtio-blk] MMIO write at 0x50
panic: panicked at src/virtio_blk.rs:58:18:
unknown virtio-blk mmio write: offs=0x50
```

- Linux prints the disk capacity the hypervisor reports.
- Linux reads `ConfigGeneration` register twice: before and after reading the disk capacity (0x100 and 0x104) to check if the config has changed.

Now it fails with offset 0x50 (`QueueNotify`), which notifies the hypervisor that the guest has sent a request - that is, the virtio initialization is complete! Good job!

## Implement queue notify register

```rs [src/virtio_blk.rs] {6,11-14}
impl VirtioBlk {
	fn handle_mmio_write(&self, offset: u64, width: u64, value: u64) {
		match offset {
			/* ... */
            0x44 => {}, // Queue ready (ignored)
            0x50 => self.process_queue(value as usize), // Queue notify
			/* ... */
		}
	}

	fn process_queue(&mut self, queue_index: usize) {
        assert_eq!(queue_index, 0, "only queue #0 (requestq) is supported");
		println!("[virtio-blk] processing requestq");
	}
}
```

```
$ ./run.sh
...
[guest] [    0.212706] virtio_blk virtio0: [vda] 1920 512-byte logical blocks (983 kB/960 KiB)
[virtio-blk] MMIO read at 0x70
[virtio-blk] MMIO write at 0x70
[virtio-blk] MMIO write at 0x50
[virtio-blk] processing requestq
```

## How virtqueue works

```rs [src/virtio_blk.rs]
#[repr(C)]
struct VirtqDesc {
    addr: u64,  // Buffer physical address
    len: u32,   // Buffer length
    flags: u16, // Buffer flags
    next: u16,  // Next descriptor index (if chained)
}

#[repr(C)]
struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; 128], // Ring of descriptor indices
}

#[repr(C)]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; 128],
}

#[repr(C)]
struct VirtqUsedElem {
    id: u32,  // Descriptor index
    len: u32, // Length of data written
}
```

```rs
struct VirtioBlk {
  /// Available Ring (Driver area: requests from the guest).
  avail: &'static VirtqAvail,
  /// Used Ring (Device area: processed requests to the guest).
  used: &'static mut VirtqUsed,
  /// Descriptor Table.
  descs: &'static [VirtqDesc; 128],
  /// The next index to pop from the available ring.
  next_avail_idx: u16,
}

impl VirtioBlk {
	fn process_queue(&mut self) {
		while self.avail.idx != self.next_avail_idx {
			let head_index = self.avail.ring[self.next_avail_idx % QUEUE_SIZE];

			// The first descriptor contains the request metadata.
			let desc1 = self.descs[head_index];
			let req: VirtioBlkReq = read_guest_addr(desc1.addr);
			if req.type != VIRTIO_BLK_T_IN {
				panic!("unsupported request type: {}", req.req_type);
			}

			// The second descriptor points to the data buffer.
			let desc2 = self.descs[desc1.next as usize];
			let copy_len = desc2.len;

			// Write the disk image, and status byte (VIRTIO_BLK_S_OK).
			let offset = req.sector * 512;
			write_guest_addr(desc2.addr, &DISK_IMAGE[offset..offset + copy_len]);
			write_guest_addr(desc2.addr + copy_len, VIRTIO_BLK_S_OK);

			// Update the used queue.
			self.used.ring[self.used.idx % QUEUE_SIZE] = VirtqUsedElem {
				id: head_index,
				len: copy_len + size_of::<u8>(),
			};

			// Update the device's read offsets.
			self.used.idx += 1;
			self.next_avail_idx += 1;
		}

		// Notify the driver that the queue has been processed.
		self.trigger_interrupt();
	}
}
```

1. Check if there are any requests to process.
2. Pop a element from the available ring, which contains the head index of the descriptor chain.
3. Read the descriptor chain. Each descriptor contains the guest address of requests.
4. Update the used ring to tell the driver that we've processed a request.
5. Once it's done, notify the driver that the queue has processed requests.

However, 

- `volatile`
- How can we access the virtqueue from the hypervisor?
- Endianess.

## Guest memory API

```rs [src/main.rs]
mod guest_memory;
```

```rs [src/guest_memory.rs]
use crate::{guest_page_table::GuestPageTable, linux_loader::{GUEST_BASE_ADDR, GUEST_DTB_ADDR, MEMORY_SIZE}};

pub static GUEST_MEMORY: GuestMemory<MEMORY_SIZE> = GuestMemory::new(GUEST_BASE_ADDR);
pub static DTB_MEMORY: GuestMemory<0x10000> = GuestMemory::new(GUEST_DTB_ADDR);

#[repr(C, align(4096))]
pub struct GuestMemory<const SIZE: usize> {
    data: [u8; SIZE],
    guest_base: u64,
}

impl<const SIZE: usize> GuestMemory<SIZE> {
    pub const fn new(guest_base: u64) -> Self {
        Self { data: [0; SIZE], guest_base }
    }

    pub fn write_bytes(&self, table: &mut GuestPageTable, src: &[u8], flags: u64) {
        let raw_ptr = self.data.as_ptr() as *mut u8;
        let slice = unsafe { core::slice::from_raw_parts_mut(raw_ptr, SIZE) };
        slice[..src.len()].copy_from_slice(src);
        for off in (0..SIZE as u64).step_by(4096) {
			let guest_addr = self.guest_base + off;
			let host_addr = raw_ptr as u64 + off;
            table.map(guest_addr, host_addr, flags);
        }
    }
}
```

```rs [src/linux_loader.rs] {1,10,13}
use crate::guest_memory::{GUEST_MEMORY, DTB_MEMORY};

pub fn load_linux_kernel(table: &mut GuestPageTable, image: &[u8]) {
    assert!(image.len() >= size_of::<RiscvImageHeader>());
    let header = unsafe { &*(image.as_ptr() as *const RiscvImageHeader) };
    assert_eq!(u32::from_le(header.magic2), 0x05435352, "invalid magic");

    let kernel_size = u64::from_le(header.image_size);
    assert!(image.len() <= MEMORY_SIZE);
    GUEST_MEMORY.write(table, image, PTE_R | PTE_W | PTE_X);

    let dtb = build_device_tree().unwrap();
    DTB_MEMORY.write(table, &dtb, PTE_R);

    println!("loaded kernel: size={}KB", kernel_size / 1024);
}
```