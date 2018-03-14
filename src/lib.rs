//! Create a Fighter from an exported brawl fighter directory.
//! ```rust,no_run
//! use brawllib_rs::fighter::Fighter;
//! use std::fs;
//!
//! for fighter in Fighter::load(fs::read_dir("some/real/dir/fighter").unwrap(), None, false) {
//!     println!("fighter name: {}", fighter.cased_name);
//! }
//! ```
//! You can now read data from the tree of structs contained within the Fighter.

#![feature(nll)]
extern crate byteorder;
extern crate cgmath;

#[macro_use] extern crate bitflags;

pub mod parse;
pub mod util;
pub mod bres;
pub mod resources;
pub mod chr0;
pub mod mdl0;
pub mod mbox;
pub mod misc_section;
pub mod fighter;
