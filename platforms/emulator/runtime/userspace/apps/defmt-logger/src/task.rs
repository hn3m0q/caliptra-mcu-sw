//! Embassy task for defmt-logger example

use core::fmt::Write;
use libtock_console::Console;

/// Example embassy task that demonstrates logging
#[embassy_executor::task]
pub async fn logger_demo_task() {
    let mut console_writer = Console::<libtock_runtime::TockSyscalls>::writer();

    writeln!(console_writer, "[Logger Task] Starting logger demo task").unwrap();

    // Do some iterations with console output
    for i in 0..5 {
        writeln!(console_writer, "[Logger Task] Iteration {} completed", i).unwrap();
    }

    writeln!(console_writer, "[Logger Task] Demo task completed").unwrap();
}
