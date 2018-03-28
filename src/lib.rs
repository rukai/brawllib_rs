//! In the below example we:
//! *   Create a Fighter from an exported brawl fighter directory.
//!     This is the raw data from the fighter folder, stored in a tree of structs
//! *   Create a HighLevelFighter from an exported brawl fighter directory.
//!     This contains processed data from the Fighter struct, stored in a tree of structs
//! ```rust,no_run
//! use brawllib_rs::fighter::Fighter;
//! use brawllib_rs::high_level_fighter::HighLevelFighter;
//! use std::fs;
//!
//! for fighter in Fighter::load(fs::read_dir("some/real/dir/fighter").unwrap(), None, false) {
//!     println!("Fighter name: {}", fighter.cased_name);
//!     println!("The name of the first model file name: {}", fighter.models[0].name);
//!
//!     let hl_fighter = HighLevelFighter::new(&fighter);
//!     println!("Hurtboxes on the 4th frame of 'Run' action {:#?}", hl_fighter.actions.iter().find(|x| x.name == "Run").unwrap().frames[4].hurt_boxes);
//! }
//! ```

#![feature(nll)]

             extern crate byteorder;
             extern crate cgmath;
#[macro_use] extern crate log;
#[macro_use] extern crate bitflags;

pub mod arc;
pub mod bres;
pub mod chr0;
pub mod fighter;
pub mod high_level_fighter;
pub mod mbox;
pub mod mdl0;
pub mod misc_section;
pub mod resources;
pub mod sakurai;
pub mod script;
mod math;
mod util;
