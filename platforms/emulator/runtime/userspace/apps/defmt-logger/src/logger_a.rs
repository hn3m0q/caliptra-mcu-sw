//! Logger A - Standard defmt logger implementation

use core::{
    cell::UnsafeCell,
    fmt::Write,
    sync::atomic::{AtomicBool, Ordering},
};

use libtock_console::Console;

#[defmt::global_logger]
struct LoggerA;

static ENCODER: Encoder = Encoder::new();

struct Encoder {
    /// A boolean lock
    ///
    /// Is `true` when `acquire` has been called and we have exclusive access to
    /// the rest of this structure.
    taken: AtomicBool,
    /// We need to remember this to exit a critical section
    cs_restore: UnsafeCell<critical_section::RestoreState>,
    /// A defmt::Encoder for encoding frames
    encoder: UnsafeCell<defmt::Encoder>,
}

impl Encoder {
    /// Create a new defmt-encoder
    const fn new() -> Encoder {
        Encoder {
            taken: AtomicBool::new(false),
            cs_restore: UnsafeCell::new(critical_section::RestoreState::invalid()),
            encoder: UnsafeCell::new(defmt::Encoder::new()),
        }
    }

    /// Acquire the defmt encoder.
    fn acquire(&self) {
        // safety: Must be paired with corresponding call to release(), see below
        let restore = unsafe { critical_section::acquire() };

        // NB: You can re-enter critical sections but we need to make sure
        // no-one does that.
        if self.taken.load(Ordering::Relaxed) {
            panic!("defmt logger taken reentrantly")
        }

        // no need for CAS because we are in a critical section
        self.taken.store(true, Ordering::Relaxed);

        // safety: accessing the cell is OK because we have acquired a critical
        // section.
        unsafe {
            self.cs_restore.get().write(restore);
            let encoder: &mut defmt::Encoder = &mut *self.encoder.get();
            encoder.start_frame(write_bytes);
        }
    }

    /// Release the defmt encoder.
    unsafe fn release(&self) {
        if !self.taken.load(Ordering::Relaxed) {
            panic!("defmt release out of context")
        }

        // safety: accessing the cell is OK because we have acquired a critical
        // section.
        unsafe {
            let encoder: &mut defmt::Encoder = &mut *self.encoder.get();
            encoder.end_frame(write_bytes);
            let restore = self.cs_restore.get().read();
            self.taken.store(false, Ordering::Relaxed);
            // paired with exactly one acquire call
            critical_section::release(restore);
        }
    }

    /// Write bytes to the defmt encoder.
    unsafe fn write(&self, bytes: &[u8]) {
        if !self.taken.load(Ordering::Relaxed) {
            panic!("defmt write out of context")
        }

        // safety: accessing the cell is OK because we have acquired a critical
        // section.
        unsafe {
            let encoder: &mut defmt::Encoder = &mut *self.encoder.get();
            encoder.write(bytes, write_bytes);
        }
    }
}

/// Write encoded bytes to the console output - Logger A format
///
/// This writes the defmt-encoded bytes to the Tock console, prefixed with [DEFMT-A: marker
fn write_bytes(bytes: &[u8]) {
    #[cfg(target_arch = "riscv32")]
    {
        let mut console = Console::<libtock_runtime::TockSyscalls>::writer();
        // Logger A uses [DEFMT-A: prefix
        let _ = write!(console, "[DEFMT-A:");
        for byte in bytes {
            let _ = write!(console, "{:02X}", byte);
        }
        let _ = writeln!(console, "]");
    }

    #[cfg(not(target_arch = "riscv32"))]
    {
        let mut console = Console::<libtock_unittest::fake::Syscalls>::writer();
        let _ = write!(console, "[DEFMT-A:");
        for byte in bytes {
            let _ = write!(console, "{:02X}", byte);
        }
        let _ = writeln!(console, "]");
    }
}

unsafe impl Sync for Encoder {}

unsafe impl defmt::Logger for LoggerA {
    fn acquire() {
        ENCODER.acquire();
    }

    unsafe fn flush() {
        // No-op for this minimal implementation
    }

    unsafe fn release() {
        unsafe {
            ENCODER.release();
        }
    }

    unsafe fn write(bytes: &[u8]) {
        unsafe {
            ENCODER.write(bytes);
        }
    }
}
