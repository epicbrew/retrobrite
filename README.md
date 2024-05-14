Retrobrite
==========

An NES emulator written in Rust.

This project is in the early stages of development and does not yet support
full NES emulation.

What works:

  - 6502 CPU emulation (full instruction set)
  - PPU background rendering


Building
--------

To build retrobrite, first install the necessary dependencies:

  - Install [Rust](https://www.rust-lang.org/) per your preferred method
  - SDL2 and SDL2-image development libraries
    - SDL2-devel SDL2\_image-devel (package names may vary by Linux distribution)
  - [Just](https://github.com/casey/just) (optional dependency)

Compile and run with cargo:

    $ cargo build
    $ cargo run -- --help  # Shows help
    $ cargo run -- ROM     # Run ROM file

Compile and run with Just:

    $ just list            # Get list of commands
    $ just run ROM         # Run ROM (in debug mode)
    $ just runrelease ROM  # Run ROM (in release mode)
    $ just unittest        # Run unit tests
    $ just nestest         # Run nestest test rom
