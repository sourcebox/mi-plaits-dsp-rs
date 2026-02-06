# mi-plaits-dsp

Native Rust port of the DSP code used by the [Mutable Instruments Plaits](https://mutable-instruments.net/modules/plaits/) Eurorack module.

This port is based on firmware release 1.2 as published at <https://github.com/pichenettes/eurorack>.

**NOTE:** The original code runs at a sampling frequency of 48kHz. Using other sample rates is possible, but there are noticeable differences in sound.

## Background

`Plaits` is a Eurorack module released by Mutable Instruments in 2018. It is a macro oscillator featuring 24 different engines. Please refer to the [original user manual](https://mutable-instruments.net/modules/plaits/manual/) for further details.

Since the original firmware as well as other design files like schematics were released under permissive open sources licenses, a number of derivatives of this module were made by several people. Also, alternative firmware versions exist for the module and parts of the DSP code were even used in unrelated commercial products.

## Goals

The goal of this crate is to give access to the algorithms to the Rust community, not to set a foundation for replicating the original hardware. As a result, no hardware dependent parts are included. Nevertheless, the crate is `no_std`, so use on embedded hardware is possible.

The major motivation behind this port is:

- To provide building blocks that can be used independently
- Make performance comparisons to the original C++ code and improve it

The APIs used in this crate are kept close to the original ones intentionally, resulting in a number of clippy warnings that have been suppressed.

## Tests

Run `cargo test` to run a number of integration tests that produce `WAV` files in the `./out` directory.

## License

Published under the MIT license. All contributions to this project must be provided under the same license conditions.

Author: Oliver Rockstedt <info@sourcebox.de>  
Original author: Emilie Gillet <emilie.o.gillet@gmail.com>

## Donations

If you like to support my work, you can [buy me a coffee.](https://www.buymeacoffee.com/sourcebox)

<a href="https://www.buymeacoffee.com/sourcebox" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/default-orange.png" alt="Buy Me A Coffee" height="41" width="174"></a>
