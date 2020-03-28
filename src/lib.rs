//! In this example we:
//! *   Create a `BrawlMod` from a brawl mod sd card folder and a brawl dump folder.
//! *   Load `Fighter`s from the brawl_mod.
//!     This is the raw data from the fighter folder, stored in a tree of structs.
//! *   Create a `HighLevelFighter` from an exported brawl fighter directory.
//!     This contains processed data from the `Fighter` struct, stored in a tree of structs.
//!
//! ```rust,no_run
//! use brawllib_rs::brawl_mod::BrawlMod;
//! use brawllib_rs::high_level_fighter::HighLevelFighter;
//! use std::path::PathBuf;
//!
//! let brawl_path = PathBuf::from("path/to/a/brawl/dump/folder");
//! let mod_path = PathBuf::from("path/to/a/brawl/mod/sd/card/folder");
//! let brawl_mod = BrawlMod::new(&brawl_path, Some(&mod_path));
//!
//! for fighter in brawl_mod.load_fighters(false).unwrap() {
//!     println!("Fighter name: {}", fighter.cased_name);
//!     println!("The name of the first model file name: {}", fighter.models[0].name);
//!
//!     let hl_fighter = HighLevelFighter::new(&fighter);
//!     println!("Hurtboxes on the 4th frame of 'Run' action {:#?}", hl_fighter.subactions.iter().find(|x| x.name == "Run").unwrap().frames[4].hurt_boxes);
//! }
//! ```

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate bitflags;
#[macro_use] extern crate log;

pub mod arc;
pub mod brawl_mod;
pub mod bres;
pub mod chr0;
pub mod fighter;
pub mod high_level_fighter;
pub mod math;
pub mod mbox;
pub mod mdl0;
pub mod plt0;
pub mod renderer;
pub mod resources;
pub mod sakurai;
pub mod script;
pub mod script_ast;
pub mod script_runner;
pub mod user_data;
pub mod wii_memory;
pub mod wii_texture_formats;
pub mod wiird_runner;
mod util;
mod action_names;
mod fighter_maps;
mod init_hack_script;
