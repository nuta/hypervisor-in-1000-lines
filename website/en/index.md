# Hypervisor in 1,000 Lines

> [!WARNING]
> This book is work in progress.

Hey there (maybe again)! In this book, you'll learn how to build a minimal RISC-V hypervisor which can boot Linux-based operating systems.

This is a sequel to the online book [Operating System in 1,000 Lines](https://1000os.seiya.me/en/). In that book, you have learned how to build a minimal operating system from scratch in C, but this time, we'll start from scratch again in your favorite language, Rust!

From scratch means we'll start from the bare-metal programming in Rust, that is *type-1 hypervisor*, in 1000 lines of code like we did for the OS.

However, this time we'll cheat a little bit, by the power of Rust's ecosystem: third-party libraries (*"crates"*) to avoid implementing things that don't really matter for learning hypervisors.

- You can download the implementation examples from [GitHub](https://github.com/nuta/hypervisor-in-1000-lines).
- This book is available under the [CC BY 4.0 license](https://creativecommons.jp/faq). The implementation examples and source code in the text are under the [MIT license](https://opensource.org/licenses/MIT).

Happy hypervisor hacking!
