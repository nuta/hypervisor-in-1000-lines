
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

##
