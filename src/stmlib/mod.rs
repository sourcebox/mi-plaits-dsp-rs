//! Utility functions originally coded for STM32 and shared between several devices
//! (now platform-independent).
//!
//! This module contains ports of functions that were used in several Mutable Instruments
//! devices in common and were made for the STM32 platform - hence the name `stmlib`.
//! The name is intentionally kept for cross-reference with the original C++ code, even if no
//! platform-specific implementations are used in this module.

pub mod dsp;
pub mod utils;
