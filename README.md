# mi-plaits-dsp

Native Rust port of the DSP code used by the [Mutable Instruments Plaits](https://mutable-instruments.net/modules/plaits/) Eurorack module.

This port is work in progress and based on <https://github.com/pichenettes/eurorack>

## Background

`Plaits` is a Eurorack module released by Mutable Instruments in 2018. It is a macro oscillator featuring 16 different engines.

Since the original firmware as well as other design files like schematics were released under permissive open sources licenses, a number of derivates of this module were made by several people. Also, alternative firmware versions exist for the module and parts of the DSP code were even used in unrelated commercial products.

## Goals

The goal of this crate is to give access to the algorithms to the Rust community, not to set a foundation for replicating the original hardware. As a result, no hardware dependent parts are included. Nevertheless, the crate is `no_std`, so use on embedded hardware is possible.

The major motivation behind this port is:

- To provide building blocks that can be used independently
- Make performance comparisons to the original C++ code and improve it

The APIs used in this crate are kept close to the original ones intentionally, resulting in a number of clippy warnings that have been surpressed.

## License

Published under the MIT license. All contributions to this project must be provided under the same license conditions.

Author: Oliver Rockstedt <info@sourcebox.de>  
Original author: Emilie Gillet <emilie.o.gillet@gmail.com>
