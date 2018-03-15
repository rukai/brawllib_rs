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
#[macro_use] extern crate log;
#[macro_use] extern crate bitflags;

pub mod arc;
pub mod bres;
pub mod chr0;
pub mod fighter;
pub mod mbox;
pub mod mdl0;
pub mod misc_section;
pub mod resources;
pub mod sakurai;
mod util;
