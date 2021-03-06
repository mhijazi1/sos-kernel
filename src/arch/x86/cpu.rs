//
//  SOS: the Stupid Operating System
//  by Hawk Weisman (hi@hawkweisman.me)
//
//  Copyright (c) 2015 Hawk Weisman
//  Released under the terms of the MIT license. See `LICENSE` in the root
//  directory of this repository for more information.
//
//! Code for interacting with the x86 CPU.
//!
//! Currently this module contains a quick implementation of CPU port
//! input and output, and little else.
//!

#[path = "../x86_all/cpu.rs"] mod cpu_all;
#[path = "../x86_all/pics.rs"] pub mod pics;

pub use self::cpu_all::*;
