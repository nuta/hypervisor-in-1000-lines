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

```rust [src/linux_loader.rs]
pub const VIRTIO_BLK_ADDR: u64 = 0x0a00_0000;
pub const VIRTIO_BLK_END: u64 = VIRTIO_BLK_ADDR + 0x1000;
```

```rust [src/linux_loader.rs] {4-9}
    fdt.property_phandle(2)?;
    fdt.end_node(plic_node)?;

    let virtio_node = fdt.begin_node("virtio_mmio@a000000")?;
    fdt.property_string("compatible", "virtio,mmio")?;
    fdt.property_array_u64("reg", &[VIRTIO_BLK_ADDR, 0x1000])?;
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

## Scaffold virtio-blk device emulation

```rs [src/main.rs]
mod virtio_blk;
```

```rs [src/virtio_blk.rs]
use spin::Mutex;

pub static VIRTIO_BLK: Mutex<VirtioBlk> = Mutex::new(VirtioBlk::new());

pub struct VirtioBlk {
}

impl VirtioBlk  {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn handle_mmio_write(&mut self, offset: u64, _value: u64, width: u64) {
        println!("[virtio-blk] MMIO write at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            _ => panic!("unknown virtio-blk mmio write: offs={:#x}", offset),
        }
    }

    pub fn handle_mmio_read(&self, offset: u64, _width: u64) -> u64 {
        println!("[virtio-blk] MMIO read at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            0x00 => 0x74726976,  // Magic value "virt"
            0x04 => 0x2,         // Version
            0x08 => 0x2,         // Device ID (block device)
            0x0c => 0x554d4551,  // Vendor ID "QEMU"
            _ => panic!("unknown virtio-blk mmio read: guest_addr={:#x}", offset),
        }
    }
}
```

```rs [src/trap.rs] {6-17}
fn handle_mmio_write(vcpu: &mut VCpu, guest_addr: u64, reg: u64, width: u64) {
    let value = match reg {
        /* ... */
    };

    match guest_addr {
        PLIC_ADDR..PLIC_END => {
            println!("[MMIO]: ignore write to PLIC at {:#x}", guest_addr);
        }
        VIRTIO_BLK_ADDR..VIRTIO_BLK_END => {
            let offset = guest_addr - VIRTIO_BLK_ADDR;
            VIRTIO_BLK.lock().handle_mmio_write(offset, value, width);
        }
        _ => {
            panic!("[MMIO]: invalid write at {:#x} (value={:#x}, width={})", guest_addr, value, width);
        }
    }
}
```

```rs [src/trap.rs] {2-14}
fn handle_mmio_read(vcpu: &mut VCpu, guest_addr: u64, reg: u64, width: u64) {
    let value = match guest_addr {
        PLIC_ADDR..PLIC_END => {
            println!("[MMIO]: ignore read from PLIC at {:#x}", guest_addr);
            0
        }
        VIRTIO_BLK_ADDR..VIRTIO_BLK_END => {
            let offset = guest_addr - VIRTIO_BLK_ADDR;
            VIRTIO_BLK.lock().handle_mmio_read(offset, width)
        }
        _ => {
            panic!("[MMIO]: invalid read at {:#x} (width={})", guest_addr, width);
        }
    };

    /* ... */
}
```

```
$ ./run.sh
...
[virtio-blk] MMIO read at 0x0
[virtio-blk] MMIO read at 0x4
[virtio-blk] MMIO read at 0x8
[virtio-blk] MMIO read at 0xc
panic: panicked at src/virtio_blk.rs:16:18:
unknown virtio-blk mmio write: offs=0x70
```

Great, Linux's device driver does not complain about the magic value, and wants to continue the initialization.

## Virtio initialization

> **3.1.1 Driver Requirements: Device Initialization**
>
> The driver MUST follow this sequence to initialize a device:
>
> 1. Reset the device.
> 2. Set the ACKNOWLEDGE status bit: the guest OS has noticed the device.
> 3. Set the DRIVER status bit: the guest OS knows how to drive the device.
> 4. Read device feature bits, and write the subset of feature bits understood by the OS and driver to the device. During this step the driver MAY read (but MUST NOT write) the device-specific configuration fields to check that it can support the device before accepting it.
> 5. Set the FEATURES_OK status bit. The driver MUST NOT accept new feature bits after this step.
> 6. Re-read device status to ensure the FEATURES_OK bit is still set: otherwise, the device does not support our subset of features and the device is unusable.
> 7. Perform device-specific setup, including discovery of virtqueues for the device, optional per-bus setup, reading and possibly writing the device’s virtio configuration space, and population of virtqueues.
> 8. Set the DRIVER_OK status bit. At this point the device is “live”.
>
> [Virtual I/O Device (VIRTIO) Version 1.3](https://docs.oasis-open.org/virtio/virtio/v1.3/virtio-v1.3.html)

## Emulate device status register

Let's start with `Status` register. According to the spec:

> **Device status**
>
> Reading from this register returns the current device status flags. Writing non-zero values to this register sets the status flags, indicating the driver progress. Writing zero (0x0) to this register triggers a device reset. See also p. 4.2.3.1 Device Initialization.

```rs [src/virtio_blk.rs] {2,8,16,29}
pub struct VirtioBlk {
    status: u32,
}

impl VirtioBlk  {
    pub const fn new() -> Self {
        Self {
            status: 0,
        }
    }

    pub fn handle_mmio_write(&mut self, offset: u64, value: u64, width: u64) {
        println!("[virtio-blk] MMIO write at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            0x70 => self.status = value as u32, // Device status
            _ => panic!("unknown virtio-blk mmio write: offs={:#x}", offset),
        }
    }

    pub fn handle_mmio_read(&self, offset: u64, _width: u64) -> u64 {
        println!("[virtio-blk] MMIO read at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            0x00 => 0x74726976,  // Magic value "virt"
            0x04 => 0x2,         // Version
            0x08 => 0x2,         // Device ID (block device)
            0x0c => 0x554d4551,  // Vendor ID "QEMU"
            0x70 => self.status as u64,  // Device status
            _ => panic!("unknown virtio-blk mmio read: guest_addr={:#x}", offset),
        }
    }
}
```

## Emulate device features register

> **DeviceFeatures (0x010): Flags representing features the device supports**
>
> Reading from this register returns 32 consecutive flag bits, the least significant bit depending on the last value written to `DeviceFeaturesSel`. Access to this register returns bits `DeviceFeaturesSel * 32` to `(DeviceFeaturesSel * 32) + 31`, eg. feature bits 0 to 31 if `DeviceFeaturesSel` is set to 0 and features bits 32 to 63 if DeviceFeaturesSel is set to 1.

> **DeviceFeaturesSel (0x020): Device (host) features word selection**
>
> Writing to this register selects a set of 32 device feature bits accessible by reading from `DeviceFeatures`.


```rs [src/virtio_blk.rs] {3}
pub struct VirtioBlk {
    status: u32,
    device_features_sel: u32,
}
```

```rs [src/virtio_blk.rs] {6}
    pub fn handle_mmio_write(&mut self, offset: u64, value: u64, width: u64) {
        println!("[virtio-blk] MMIO write at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            0x70 => self.status = value as u32, // Device status
            0x14 => self.device_features_sel = value as u32, // Device features selection
            _ => panic!("unknown virtio-blk mmio write: offs={:#x}", offset),
        }
    }
```

```rs [src/virtio_blk.rs] {9-12}
    pub fn handle_mmio_read(&self, offset: u64, width: u64) -> u64 {
        println!("[virtio-blk] MMIO read at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            0x00 => 0x74726976,  // Magic value "virt"
            0x04 => 0x2,         // Version
            0x08 => 0x2,         // Device ID (block device)
            0x0c => 0x554d4551,  // Vendor ID "QEMU"
            // Device features: 31-0 bits
            0x10 if self.device_features_sel == 0 => 0,
            // Device features: 63-32 bits
            0x10 if self.device_features_sel == 1 => 0,
```

```
$ ./run.sh
...
[guest] [    0.194779] virtio_blk virtio0: New virtio-mmio devices (version 2) must provide VIRTIO_F_VERSION_1 feature!
...
```

Oh, the guest complains that the version is not supported. Let's add the feature bit.

```rs [src/virtio_blk.rs] {4}
            // Device features: 31-0 bits
            0x10 if self.device_features_sel == 0 => 0,
            // Device features: 63-32 bits (VIRTIO_F_VERSION_1)
            0x10 if self.device_features_sel == 0x0000_0001,
```

```
$ ./run.sh
...
panic: panicked at src/virtio_blk.rs:23:18:
unknown virtio-blk mmio write: offs=0x24
```

## Emulate driver features register

> **DriverFeatures (0x020): Flags representing features the driver supports**
>
> Reading from this register returns 32 consecutive flag bits, the least significant bit depending on the last value written to `DriverFeaturesSel`. Access to this register returns bits `DriverFeaturesSel * 32` to `(DriverFeaturesSel * 32) + 31`, eg. feature bits 0 to 31 if `DriverFeaturesSel` is set to 0 and features bits 32 to 63 if DriverFeaturesSel is set to 1.

> **DriverFeaturesSel (0x024): Activated (guest) features word selection**
>
> Writing to this register selects a set of 32 activated feature bits accessible by writing to DriverFeatures.


```rs [src/virtio_blk.rs] {4,12}
pub struct VirtioBlk {
    status: u32,
    device_features_sel: u32,
    driver_features_sel: u32,
}

impl VirtioBlk  {
    pub const fn new() -> Self {
        Self {
            status: 0,
            device_features_sel: 0,
            driver_features_sel: 0,
        }
    }
```

```rs [src/virtio_blk.rs] {7-10}
    pub fn handle_mmio_write(&mut self, offset: u64, value: u64, width: u64) {
        println!("[virtio-blk] MMIO write at {:#x}", offset);
        assert_eq!(width, 4);
        match offset {
            0x70 => self.status = value as u32, // Device status
            0x14 => self.device_features_sel = value as u32, // Device features selection
            // Driver features
            0x20 => {}, // ignore writes
            // Driver features selection
            0x24 => self.driver_features_sel = value as u32,
            _ => panic!("unknown virtio-blk mmio write: offs={:#x}", offset),
        }
    }
```

```
$ ./run.sh
...
panic: panicked at src/virtio_blk.rs:28:18:
unknown virtio-blk mmio write: offs=0x30
```
