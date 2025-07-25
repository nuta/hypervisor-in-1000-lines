---
title: Virtual Disk
---

# Virtual Disk

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

## Advertise the virtual disk to the guest

```rust [src/linux_loader.rs] {2}
    let chosen_node = fdt.begin_node("chosen")?;
    fdt.property_string("bootargs", "console=hvc earlycon=sbi panic=-1 root=/dev/vda init=/bin/catsay")?;
    fdt.end_node(chosen_node)?;
```

```rust [src/linux_loader.rs] {4}
pub const GUEST_DTB_ADDR: u64 = 0x7000_0000;
pub const GUEST_BASE_ADDR: u64 = 0x8000_0000;
pub const GUEST_PLIC_ADDR: u64 = 0x0c00_0000;
pub const GUEST_VIRTIO_BLK_ADDR: u64 = 0x0a00_0000;
```

```rust [src/linux_loader.rs] {4-9}
    fdt.property_phandle(2)?;
    fdt.end_node(plic_node)?;

    let virtio_node = fdt.begin_node("virtio_mmio@a000000")?;
    fdt.property_string("compatible", "virtio,mmio")?;
    fdt.property_array_u64("reg", &[GUEST_VIRTIO_BLK_ADDR, 0x1000])?;
    fdt.property_u32("interrupt-parent", 2)?;
    fdt.property_array_u32("interrupts", &[1])?;
    fdt.end_node(virtio_node)?;

    fdt.end_node(root_node)?;
    fdt.finish()
```

```
$ ./run.sh
...
[MMIO]: read 0xa000000 (width=4)
[guest] [    0.184343] virtio-mmio a000000.virtio_mmio: Wrong magic value 0x00000000!
...
```

Great! Now the guest now tries to use the virtio-mmio device, but it fails because the magic value is incorrect.

## Virtio-mmio
