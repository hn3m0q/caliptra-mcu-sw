// Licensed under the Apache-2.0 license

#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]
#![feature(impl_trait_in_assoc_type)]
#![allow(static_mut_refs)]

use core::fmt::Write;
use embassy_sync::lazy_lock::LazyLock;
use libtock_console::Console;
use libtock_platform::Syscalls;
use libtockasync::TockExecutor;

// Import defmt-logger. Due to how Rust procedural macros work,
// defmt must be accessible (it's provided transitively by defmt-logger).
use defmt_logger::defmt;

#[cfg(target_arch = "riscv32")]
mod riscv;

pub static EXECUTOR: LazyLock<TockExecutor> = LazyLock::new(TockExecutor::new);

#[cfg(not(target_arch = "riscv32"))]
pub(crate) fn kernel() -> libtock_unittest::fake::Kernel {
    use libtock_unittest::fake;
    let kernel = fake::Kernel::new();
    let console = fake::Console::new();
    kernel.add_driver(&console);
    kernel
}

#[cfg(not(target_arch = "riscv32"))]
fn main() {
    // build a fake kernel so that the app will at least start without Tock
    let _kernel = kernel();
    // call the main function
    libtockasync::start_async(start());
}

#[cfg(target_arch = "riscv32")]
#[embassy_executor::task]
async fn start() {
    async_main::<libtock_runtime::TockSyscalls>().await;
}

#[cfg(not(target_arch = "riscv32"))]
#[embassy_executor::task]
async fn start() {
    async_main::<libtock_unittest::fake::Syscalls>().await;
}

// Simple embassy task that prints a message
#[embassy_executor::task]
async fn simple_task() {
    let mut console_writer = Console::<libtock_runtime::TockSyscalls>::writer();

    writeln!(console_writer, "Simple task started!").unwrap();

    // Simulate some work
    let mut counter = 0;
    for _ in 0..10 {
        counter += 1;
        writeln!(console_writer, "Simple task iteration: {}", counter).unwrap();
    }

    writeln!(console_writer, "Simple task completed!").unwrap();

    // Task finishes here
}

pub(crate) async fn async_main<S: Syscalls>() {
    let mut console_writer = Console::<S>::writer();
    writeln!(console_writer, "Hello, World!").unwrap();
    writeln!(console_writer, "This is the hello_world app").unwrap();

    // Use defmt_logger::error! macro - this will be encoded using defmt, not human-readable
    defmt_logger::error!("This is a defmt error message - it is encoded!");
    defmt_logger::info!("This is a defmt info message");
    defmt_logger::warn!("This is a defmt warning message");

    writeln!(console_writer, "Starting embassy tasks...").unwrap();

    // Spawn the simple task
    EXECUTOR
        .get()
        .spawner()
        .spawn(simple_task())
        .unwrap();

    // Spawn the defmt-logger demo task
    EXECUTOR
        .get()
        .spawner()
        .spawn(defmt_logger::task::logger_demo_task())
        .unwrap();

    writeln!(console_writer, "Tasks spawned successfully").unwrap();

    // Main executor loop
    loop {
        EXECUTOR.get().poll();
    }
}
