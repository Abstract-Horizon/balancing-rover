//! [dma_gpio](index.html) is a library for pi's GPIO that uses DMA ([Direct Memory Access](https://en.wikipedia.org/wiki/Direct_memory_access)) and [PWM via DMA](https://stackoverflow.com/questions/50427275/raspberry-how-does-the-pwm-via-dma-work).
//! By using DMA and interacting directly with hardware memory, it manages to be fast (Max Raw Speed ≈ 1.6 MHz on Pi3) while having almost no CPU usage ( ~2%).
//! This project was inspired by its predecessor projects in C, such as [PiBits](https://github.com/richardghirst/PiBits), [pi-blaster](https://github.com/sarfata/pi-blaster) and [RPIO](https://github.com/metachris/RPIO/tree/master/source); however, this project bypasses L1 cache which the DMA Controller does not recognize, resulting in a slightly faster GPIO Speed.
//! 
//! Be sure to run your binary file in sudo.
//! 
//! ## Cross-Compilation
//! Note that this library will only compile on a 32-bit machine a.k.a. a raspberry pi. So, if you try to compile this on your personal computer, it will most likely fail.
//! So, if you want to test and compile your project with this library on your PC, you should do so by cross-compiling using a specific target such as armv7-unknown-linux-gnueabihf.
//!
//! Great resource for cross-compilations to Pi that I found helpful is [rust-cross](https://github.com/japaric/rust-cross). For pi 2 and 3, use armv7-unknown-linux-gnueabihf, and, for pi 1 and zero, use armv6-unknown-linux-gnueabihf.
 
//! # Getting Started
//! First, add the crate to the dependencies.
//! ```no_run
//! Cargo.toml
//! 
//! ...
//! [dependencies]
//! dma_gpio = "0.1.8"
//! ```
//! [pi](pi/index.html) module will have everything you need.
//! 
//! The easiest way to get started using this library is initializing a [Board](pi/struct.Board.html) using [BoardBuilder](pi/struct.BoardBuilder.html).
//! BoardBuilder will configure the default setting for the DMA, but manual configuration is also available (read more on this in [BoardBuiler](pi/struct.BoardBuilder.html#building-with-custom-settings)).
//! When the Board is initialized, it recognizes which Pi-version it is running on, and interacts with the hardware memory accordingly.
//! 
//! ## Example
//! Below example initializes the board using BoardBuilder with default configurations, set the PWM of gpio pins 21 and 22 to 25%, 50%, 75%, then 100% for 1 second each.
//! 
//! Make sure to run the binary file in sudo.
//! 
//! ```no_run
//! use std::thread::sleep;
//! use std::time::Duration;
//! use dma_gpio::pi::BoardBuilder;
//! 
//! fn main() {
//!     let mut board = BoardBuilder::new().build_with_pins(vec![21, 22]).unwrap();
//!     board.print_info();
//!     
//!     board.set_all_pwm(0.25).unwrap();
//!     let sec = Duration::from_millis(1000);
//!     sleep(millis);
//!     
//!     board.set_all_pwm(0.5).unwrap();
//!     sleep(millis);
//!     
//!     board.set_all_pwm(0.75).unwrap();
//!     sleep(millis);
//!     
//!     board.set_all_pwm(1.0).unwrap();
//!     sleep(millis);
//! }
//! 
//! ```
//! 
//! # Features
//! There are two features you can enable in this crate: 'debug' and 'bind_process'. To enable these features, write the dependency for this crate as shown below.
//! ```no_run
//! Cargo.toml
//! 
//! ...
//! [dependencies]
//! ...
//! 
//! [dependencies.dma_gpio]
//! version = "0.1.8"
//! features = ["debug", "bind_process"]
//! ```
//! 
//! ## 'debug' feature
//! By enabling this feature, the library will print out the process every step of the way. This feature will be useful when debugging.
//! Also, after enabling this feature in Cargo.toml, you have to call [enable_logger](fn.enable_logger.html) function to see the logs in the terminal.
//! ```no_run
//! use std::thread::sleep;
//! use std::time::Duration;
//! use dma_gpio::pi::BoardBuilder;
//! 
//! fn main() {
//!     dma_gpio::enable_logger();
//!     let mut board = BoardBuilder::new().build_with_pins(vec![21, 22]).unwrap();
//!     
//!     board.set_all_pwm(0.5).unwrap();
//!     let sec = Duration::from_millis(2000);
//!     sleep(millis);
//! }
//! 
//! ```
//! ## 'bind_process' feature
//! This feature lets you access the [pi_core](pi_core/index.html) module which only has one function [bind_process_to_last](pi_core/fn.bind_process_to_last.html). This function binds the process to the last core of the Pi. However, to use this function, you have to first install a C library called [hwloc](https://github.com/daschl/hwloc-rs#install-hwloc-on-os-x). Also, enabling debug feature will print out if you have correctly bound process to the last core.
//! ```no_run
//! use std::thread::sleep;
//! use std::time::Duration;
//! use dma_gpio::{pi::BoardBuilder, pi_core};
//! 
//! fn main() {
//!     pi_core::bind_process_to_last();
//!     let mut board = BoardBuilder::new().build_with_pins(vec![21, 22]).unwrap();
//!     
//!     board.set_all_pwm(0.5).unwrap();
//!     let sec = Duration::from_millis(2000);
//!     sleep(millis);
//! }
//! ```
//! # Contact
//! If you have any questions or recommendations for the betterment of this project, please email me at Jack.Y.L.Dev@gmail.com

#[macro_use] extern crate log;

#[cfg(feature = "debug")]
use env_logger;

pub mod mailbox;
pub mod pi;

// if you wanna bind the process to core for better performance?
#[cfg(feature = "bind_process")]
pub mod pi_core;

/// Only accessable with "debug" feature. Use it to see traces when running
#[cfg(feature = "debug")]
pub fn enable_logger(){
    env_logger::Builder::new()
    .default_format()
    .default_format_module_path(false)
    .default_format_timestamp(false)
    .filter_level(log::LevelFilter::Warn)
    .filter_module("dma_gpio", log::LevelFilter::Trace)
    .init();
}