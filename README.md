Retrobrite
==========

An NES emulator written in Rust.

What works:

  - 6502 CPU emulation (full instruction set + illegal opcodes)
  - Background rendering
  - Sprite rendering
  - Basic NES controller input
    - Currently keyboard controls are hard coded as:
      - A: Keyboard 'A'
      - B: Keyboard 'S'
      - Select: Keyboard 'D'
      - Start: Keyboard 'F'
      - D-Pad: Keyboard up/down/left/right keys
    - USB/wireless gamepads should work but button mappings are not (yet) 
  - Mapper Support
    - 0 - NROM
    - 2 - UNROM
    - 71 - Camerica (UNROM clone)

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
    $ just run ROM         # Run ROM
    $ just debugrun ROM    # Run ROM (in debug mode)
    $ just unittest        # Run unit tests
    $ just nestest         # Run nestest test rom
