use clap::{Parser, ValueEnum};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    thread::JoinHandle,
};

/// Defines the operating mode.
#[derive(ValueEnum, Debug, Clone, PartialEq, Eq)]
enum With {
    /// Uses default IO for command spawning.
    Default,
    /// Uses null IO for command spawning.
    Null,
    /// Uses piped IO for command spawning.
    Piped,
    /// Uses piped IO (and reads the result to prevent buffering) for command spawning.
    PipedProcess,
}

impl std::fmt::Display for With {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

/// CLI arguments.
#[derive(Parser)]
struct Args {
    /// Determines the operating mode we'll run the repro with.
    #[clap(long, short, default_value_t = With::Default)]
    with: With,

    /// The command (optionally followed by arguments) to run.
    #[clap(last = true, required = true)]
    command: Vec<String>,
}

/// Piped IO bug repro, that demonstrates the issue with [`Command`] piped IO being buffered.
///
/// To run this example (on windows) try the following:
/// - `cargo run -- -w default -- cmd.exe /c type long_file.txt`: Note that the long file is written to `piped_io_repro.exe`'s stdout, as it's "inherited". No buffer limit is reached.
/// - `cargo run -- -w null -- cmd.exe /c type long_file.txt`: Note that the long file is not written, as it's "null". No buffer limit is reached.
/// - `cargo run -- -w piped -- cmd.exe /c type long_file.txt`: Note that the long file is not written, and the process does not exit. A buffer limit __is reached__.
///
/// You can see from the third case, that there is an issue where-in using piped io, that isn't actually read, will fill an internal buffer and then hang the spawned process, as it
/// tries to further write to the filled stdout.
///
/// Finally, try the following to see the safe-usage of "piped" IO, reading from the pipe to ensure the buffer isn't filled:
/// - `cargo run -- -w piped-process -- cmd.exe /c type long_file.txt`
fn main() {
    let Args { with, command } = Args::parse();

    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .stdout(match with {
            With::Default => Stdio::inherit(),
            With::Null => Stdio::null(),
            With::Piped => Stdio::piped(),
            With::PipedProcess => Stdio::piped(),
        })
        .stderr(match with {
            With::Default => Stdio::inherit(),
            With::Null => Stdio::null(),
            With::Piped => Stdio::piped(),
            With::PipedProcess => Stdio::piped(),
        })
        .spawn()
        .unwrap_or_else(|_| panic!("Failed to spawn process {:?}", command));

    // storage for thread handles if we are implementing the fix
    // otherwise, will be left empty
    let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();

    // to implement the fix, we create threads that process the piped input
    if with == With::PipedProcess {
        let child_stdout = child
            .stdout
            .take()
            .expect("Failed to obtain stdout with PipedProcess");

        let stdout_thread_handle = std::thread::spawn(|| {
            let mut process_stdout = std::io::stdout();
            let mut child_reader = BufReader::new(child_stdout).lines();
            while let Some(Ok(line)) = child_reader.next() {
                process_stdout
                    .write_all(format!("{line}\r\n").as_bytes())
                    .unwrap();
            }
        });

        // store the handle
        thread_handles.push(stdout_thread_handle);

        let child_stderr = child
            .stderr
            .take()
            .expect("Failed to obtain stderr with PipedProcess");

        let stderr_thread_handle = std::thread::spawn(|| {
            let mut process_stderr = std::io::stderr();
            let mut child_reader = BufReader::new(child_stderr).lines();
            while let Some(Ok(line)) = child_reader.next() {
                process_stderr
                    .write_all(format!("{line}\r\n").as_bytes())
                    .unwrap();
            }
        });

        // store the handle
        thread_handles.push(stderr_thread_handle);
    }

    let exit_code = child
        .wait()
        .expect("Command failed to start")
        .code()
        .expect("Command did not have a valid exit code");

    // cleanup thread handles, which will only exist if we're implementing the fix
    if !thread_handles.is_empty() {
        for handle in thread_handles {
            handle.join().unwrap();
        }
    }

    // log what happened
    println!(
        "Executed '{:?}' with '{:?}', got exit code '{:?}'",
        &command[0],
        &command[1..],
        &exit_code
    );
}
