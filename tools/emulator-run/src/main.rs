//! A defmt printer for the Caliptra MCU emulator (RISC-V).
//!
//! This tool runs the emulator and decodes defmt log data from the output.
//! It parses the `.defmt` section from the ELF binary and decodes log frames
//! as they are emitted by the running application.

use std::{
    env, fs,
    io::{BufRead, BufReader},
    process::{self, Command, Stdio},
};

use anyhow::{anyhow, bail};
use defmt_decoder::{DecodeError, StreamDecoder, Table};
use process::Child;

fn main() -> Result<(), anyhow::Error> {
    notmain().map(|opt_code| {
        if let Some(code) = opt_code {
            process::exit(code);
        }
    })
}

fn notmain() -> Result<Option<i32>, anyhow::Error> {
    let args = env::args().skip(1 /* program name */).collect::<Vec<_>>();

    if args.is_empty() {
        bail!("Usage: emulator-run <path-to-hello-world-app-elf>\n\nThis tool will:\n1. Parse the .defmt section from the ELF\n2. Run 'cargo xtask runtime'\n3. Decode and display defmt logs from the emulator output");
    }

    let elf_path = &args[0];
    let bytes = fs::read(elf_path)?;

    let table = if env::var_os("EMULATOR_RUN_IGNORE_VERSION").is_some() {
        Table::parse_ignore_version(&bytes)
    } else {
        Table::parse(&bytes)
    };
    let table = table?.ok_or_else(|| anyhow!("`.defmt` section not found in ELF file"))?;

    eprintln!("Found .defmt section in {}", elf_path);
    eprintln!("Starting emulator with 'cargo xtask runtime'...");

    let mut child = KillOnDrop(
        Command::new("cargo")
            .args(["xtask", "runtime"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to run 'cargo xtask runtime'"),
    );

    let stdout = child
        .0
        .stdout
        .take()
        .ok_or_else(|| anyhow!("failed to acquire child's stdout handle"))?;

    let mut reader = BufReader::new(stdout);
    let mut decoder = table.new_stream_decoder();

    eprintln!("Emulator started, watching for defmt data...\n");

    let mut line = String::new();
    let exit_code = loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // EOF
                if let Some(status) = child.0.try_wait()? {
                    break status.code();
                }
            }
            Ok(_) => {
                // Check if this line contains defmt data
                if line.starts_with("[DEFMT:") && line.contains(']') {
                    // Extract hex bytes between [DEFMT: and ]
                    if let Some(hex_str) = line.strip_prefix("[DEFMT:").and_then(|s| s.strip_suffix("]\n").or_else(|| s.strip_suffix("]"))) {
                        // Convert hex string to bytes
                        let mut bytes = Vec::new();
                        for i in (0..hex_str.len()).step_by(2) {
                            if i + 1 < hex_str.len() {
                                if let Ok(byte) = u8::from_str_radix(&hex_str[i..i+2], 16) {
                                    bytes.push(byte);
                                }
                            }
                        }

                        // Feed bytes to decoder
                        if !bytes.is_empty() {
                            decoder.received(&bytes);
                            let _ = decode(&mut *decoder);
                        }
                    }
                } else {
                    // Normal output line, just print it
                    print!("{}", line);
                }
            }
            Err(e) => {
                eprintln!("Error reading emulator output: {}", e);
                break None;
            }
        }

        if let Some(status) = child.0.try_wait()? {
            break status.code();
        }
    };

    Ok(exit_code)
}

fn decode(decoder: &mut dyn StreamDecoder) -> Result<(), DecodeError> {
    loop {
        match decoder.decode() {
            Ok(frame) => {
                eprintln!("[defmt] {}", frame.display(true));
            }
            Err(DecodeError::UnexpectedEof) => return Ok(()),
            Err(DecodeError::Malformed) => {
                // Don't fail on malformed data, just skip it
                // The emulator output contains non-defmt data too
                return Ok(());
            }
        }
    }
}

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        self.0.kill().ok();
    }
}
