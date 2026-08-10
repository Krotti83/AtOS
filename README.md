# AtOS
A basic educational operating system written in baremetal Rust for Raspberry Pi 3+ hardware, accompanied by a [step-by-step textbook](https://zackygamedev.github.io/RustOSGuideforRPi3/) that teaches you how to build your own operating system from scratch.

## Demo
<video src="https://github.com/user-attachments/assets/dc9853fb-4df9-4f4d-8793-552b7a235355" autoplay loop muted playsinline width="100%"></video>

---

## Overview

This is a basic operating system, written in Rust. Designed to run baremetal on the Raspberry Pi 3B+ computer hardware. It includes:

AtOS was built in order to learn about OS development, and low level programming. And to help others learn from it.  

---

## Guide Book
<img width="1869" height="906" alt="image" src="https://github.com/user-attachments/assets/167dcfe5-133c-4fb8-b886-fc49934a4fef" />

There is a fully documented guide book written on AtOS development. It aims to help others to also build a basic OS in Rust for the Raspberry Pi hardware. Which may also help in ARM low level development.

It can be found at: [zackygamedev.github.io/RustOSGuideforRPi3/](https://zackygamedev.github.io/RustOSGuideforRPi3/)
(It is still being written roughly weekly)

---

## Current Feature Set

* Process management with creation, termination, execution, and reaping of children
* Mutex locks and Spinlocks
* Round Robin Scheduler with NSP Timer
* Syscall pipeline for user programs with common syscalls (`wait()`, `sleep()`, `fork()`, etc...) 
* Fully virtualized user space
* 4KB granule paging management with 39 bit VA space
* Mini UART handling for I/O
* ELF parsing and execution
* Full exception handling pipeline


Not supported:

* Dynamic Page Allocation
* Dynamic Memory/Heap Allocation
* Networking
* Multicore execution
* GUI (All IO is through UART)

---

## Running on QEMU

You may test this OS out on QEMU. Makefiles are already included for this purpose. You may simply do:

```make
make clean
make
make run
```

Once the OS starts, you may use `help` command to explore your options.

---

## User space isolation

User space is created as a separate cargo project under `src/user`. All user programs technically are bare metal ARM projects. Programs have a `no_std` and `no_main` environment, entry point being marked through the `entry!` macro. 

You may try creating your own user programs for the OS! Explore `templates/` under `user/src/bin/`.

Theres `entry!` or `entry_args!` macro depending on whether your program needs to use entry arguments or not. The expand into proper entry runtime that parses arguments and setups environment for the user program to run. 

---

## Syscalls

All syscalls work by using the traditional `svc` instruction. Both `src/user/src/stdlib` and  `src/kernel/` contain a file named `syscalls.rs` which facilitates syscall pipeline. For user side, `syscall.rs` executes `svc` instruction to prompt kernel with some data. On kernel side a hardware exception is caused by `svc` which is identified and handled as a syscall in `kernel::syscalls`

So far syscalls available are:

1. `sys_print`: Writes some string to the UART output.
2. `sys_read`: Reads a line of string from te UART input.
3. `sys_exit`: Terminates the process.
4. `sys_fork`: Forks current running process.
5. `sys_exec`: Executes a new program in the place of current running process.
6. `sys_wait`: Waits for some or any child of current process.
7. `sys_poll_char`: Read a byte of character from UART if available, else returns `0`.
8. `sys_sleep`: Wait for given amount of milliseconds.
9. `sys_print_os_info`: Shows a `fastfetch` inspired output with hardware information about OS.
10. `sys_p_info`: Returns meta information about the running process.

---

## Memory Management Unit

The kernel is mapped in higher `ttbr1` space starting at `0xFFFFFF8000000000`. First two GB of that VA space are identity mapped to first two GB of PA space. This is so kernel has a unfiltered view of true RAM state. Effectively for kernel, the true memory is found at some offset in virtual memory. The VA space immediately after that region is used for kernel stacks for different processes.

The user space is allocated whenever a new process is spawned. User programs work in `ttbr0` VA space from `0x0000000000000000` to `0x0000007FFFFFFFFF`. User stack starting at the ending of said region.

All processes are given their own kernel stack. Which the kernel uses when prompted from corresponding process.

Pages and Kernel stacks are allocated and freed accordingly to process management.

---

## Locks

Spinlocks and Mutex locks are available to use. Spinlocks upon acquiring block any IRQs or interrupts to current code. Any hypothetical other cores trying to acquire it would need to wait in spin. Mutex locks do not block IRQ or other interruptions. It is simply that if other process tries to acquire the same mutex, it is put to sleep and woken up when said mutex is available next.

---

### User space argument passing

Before a user program is executed into a process, arguments can optionally be passed to it. Arguments passed are written to the user program's stack. Whereby the user space runtime through the entry macros parses the stack and passes it to the program as `&[&str]`.

The format in which the arguments are written on the stack is as follows:

Let's say arguments are "program hello world"

```
    STACK TOP ADDRESS (initial sp)
    ┌────────────────────┐
    │ "program"          │  ← argv[0]
    ├────────────────────┤
    │ "hello"            │  ← argv[1]
    ├────────────────────┤
    │ "world"            │  ← argv[2]
    ├────────────────────┤
    │ padding            │
    ├────────────────────┤
    │ offset → "world"   │  ← &argv[2] - final sp
    ├────────────────────┤
    │ offset → "hello"   │  ← &argv[1] - final sp
    ├────────────────────┤
    │ offset → "program" │  ← &argv[0] - final sp
    └────────────────────┘
    Final SP
```

Where the runtime is given the stack top, and final `sp` values, along with number of args (argc).


### Build Command

`make` in root directory is equivalent to:

```bash
# Navigate into user directory and compile Rust binaries for user programs
cd src/user
cargo build

# Create output folders for user binaries
mkdir -p build
mkdir -p build/dump

# Loop over every .rs file in src/user/src/bin/ (e.g., shell, init)
# For each program, copy the ELF binary and convert it to raw binary format
for prog in $(ls src/bin/*.rs | xargs -n 1 basename | sed 's/\.rs//'); do \
    cp target/aarch64-unknown-none/debug/$prog build/$prog; \
    aarch64-linux-gnu-objcopy -O binary target/aarch64-unknown-none/debug/$prog build/dump/$prog.bin; \
    echo "Built $prog.bin"; \
done

cd ../.. # Return to root directory
cargo build --release --target aarch64-unknown-none
cargo build --target aarch64-unknown-none
aarch64-linux-gnu-objcopy target/aarch64-unknown-none/release/AtOS -O binary kernel8.img
```

Output image:

```
kernel8.img
```

You can either flash it to hardware, or run it in QEMU using `make run` which is equivalent to:

```bash
qemu-system-aarch64 \
    -M raspi3b \
    -kernel kernel8.img \
    -serial null \
    -serial stdio \
    -display none
```

---

## Project Structure

- All kernel operations are written in `src/kernel/` in an individual file, for each feature.
- All user side operations for syscalls and core features are written in `src/user/src/stdlib/`.
- Main user program runtime is written in `src/user/src/lib.rs` as `user` crate.
- All individual user programs are written in `src/user/src/bin/`

---

## Intro Text

Intro text format alongisde logo is stored in `atos_intro.txt`. `build.rs` generates a valid intro string to print on the fly on building. 

---

## Purpose

This project exists to learn OS development through Rust on an ARM environment. And to help others also learn to achieve a similar task. All features of this OS are being documented in the guide book, in a implementation timeline order. An easy to follow beginner friendly manner to create an easy entry for people new to low level work.

---

## Contribution

Contributions are welcomed. New features or improvement of pre-existing features. Simple bug fixes/quality of life changes. Or even brand new user programs to try on AtOS. All are welcome. 

<small> (merge is subject to review) </small>
