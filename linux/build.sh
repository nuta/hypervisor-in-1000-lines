#!/bin/bash
cd "$(dirname "$0")"

docker build -t guest-linux-builder -f Dockerfile .

# Build Linux kernel, and copy the Image to this directory.
docker run -v $PWD:/linux -it guest-linux-builder \
    bash -c 'make -j$(nproc) Image && cp arch/riscv/boot/Image /linux/Image && cp vmlinux /linux/vmlinux'