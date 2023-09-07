# piped_io_repro

> Requires a [rust toolchain](https://rustup.rs/).

Piped IO bug repro, that demonstrates the issue with `Command` piped IO being buffered.

To run this example (on windows) try the following:

- `cargo run -- -w default -- cmd.exe /c type long_file.txt`: Note that the long file is written to `piped_io_repro.exe`'s stdout, as it's "inherited". No buffer limit is reached.
- `cargo run -- -w null -- cmd.exe /c type long_file.txt`: Note that the long file is not written, as it's "null". No buffer limit is reached.
- `cargo run -- -w piped -- cmd.exe /c type long_file.txt`: Note that the long file is not written, and the process does not exit. A buffer limit **is reached**.

  You can see from the third case, that there is an issue where-in using piped io, that isn't actually read, will fill an internal buffer and then hang the spawned process, as it
  tries to further write to the filled stdout.

  Finally, try the following to see the safe-usage of "piped" IO, reading from the pipe to ensure the buffer isn't filled:

- `cargo run -- -w piped-process -- cmd.exe /c type long_file.txt`
