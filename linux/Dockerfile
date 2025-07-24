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
