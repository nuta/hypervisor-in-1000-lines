---
title: Build Linux Kernel
---

# Build Linux Kernel

> [!WARNING]
> This chapter is work in progress.

```Dockerfile [linux/Dockerfile]
FROM ubuntu:24.04

RUN apt-get update && apt-get install -y \
    curl \
    tar \
    build-essential \
    gcc-riscv64-linux-gnu \
    binutils-riscv64-linux-gnu \
    libncurses-dev \
    flex \
    bison \
    bc

RUN curl -fSLO https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.12.34.tar.xz
RUN tar xf linux-6.12.34.tar.xz && \
    mv linux-6.12.34 kernel

WORKDIR /kernel

ENV CROSS_COMPILE=riscv64-linux-gnu-
ENV ARCH=riscv

COPY linux.config .config
```

```sh [linux/build.sh]
#!/bin/bash
cd "$(dirname "$0")"

docker build -t guest-linux-builder -f Dockerfile .

# Build Linux kernel, and copy the Image to this directory.
docker run -v $PWD:/linux -it guest-linux-builder \
    bash -c 'make -j$(nproc) Image && cp arch/riscv/boot/Image /linux/Image && cp vmlinux /linux/vmlinux'
```

```
$ ./linux/build.sh
```

```
$ ls -alh linux 
total 99M
drwxr-xr-x  7 seiya staff  224 Jul 24 15:37 .
drwxr-xr-x 24 seiya staff  768 Jul 24 15:37 ..
-rwxr-xr-x  1 seiya staff  304 Jul 24 15:37 build.sh
-rw-r--r--  1 seiya staff  461 Jul 24 15:37 Dockerfile
-rwxr-xr-x  1 seiya staff 5.2M Jul 23 20:12 Image
-rw-r--r--  1 seiya staff  43K Jul 24 15:37 linux.config
-rwxr-xr-x  1 seiya staff  93M Jul 23 20:12 vmlinux
```
