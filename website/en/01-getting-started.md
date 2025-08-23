---
title: Getting Started
---

# Getting Started

This book assumes you're using a UNIX or UNIX like OS such as macOS or Ubuntu. If you're on Windows, install Windows Subsystem for Linux (WSL2) and follow the Ubuntu instructions.

## Rust

Install Rust toolchain using [Rustup](https://rustup.rs/).

## QEMU

Another tool you need is QEMU, an emulator which runs your hypervisor:

### macOS

```sh
brew install qemu
```

### Ubuntu

```sh
sudo apt install qemu-system-riscv64
```
