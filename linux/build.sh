#!/bin/bash
cd "$(dirname "$0")"

docker build -t guest-linux-builder -f Dockerfile .

docker run -v $PWD:/linux -it guest-linux-builder \
    bash -c 'make -j$(nproc) Image && cp arch/riscv/boot/Image /linux/Image && cp vmlinux /linux/vmlinux'

# Build rootfs with catsay
rm -rf rootfs rootfs.squashfs
mkdir -p rootfs/dev # auto mounted by CONFIG_DEVTMPFS_MOUNT
mkdir -p rootfs/bin
GOOS=linux GOARCH=riscv64 go build -o rootfs/bin/catsay catsay.go
mksquashfs rootfs/ rootfs.squashfs -comp xz -b 1M -no-xattrs -noappend
