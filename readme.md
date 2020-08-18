# brawllib\_rs [![Build Status](https://travis-ci.org/rukai/brawllib_rs.svg?branch=master)](https://travis-ci.org/rukai/brawllib_rs) [![Crates.io](https://img.shields.io/crates/v/brawllib_rs.svg)](https://crates.io/crates/brawllib_rs)

A brawl file parser, based on brawlbox/brawllib, written in rust.

Doesn't support modifying files, only reading them.

## Example

In this example we:
*   Create a `BrawlMod` from the a brawl mod sd card folder and a brawl dump folder.
*   Load `Fighter`s from the brawl_mod.
    This is the raw data from the fighter folder, stored in a tree of structs.
*   Create a `HighLevelFighter` from an exported brawl fighter directory.
    This contains processed data from the `Fighter` struct, stored in a tree of structs.

```rust
use brawllib_rs::brawl_mod::BrawlMod;
use brawllib_rs::high_level_fighter::HighLevelFighter;
use std::path::PathBuf;

let brawl_path = PathBuf::from("path/to/a/brawl/dump/folder");
let mod_path = PathBuf::from("path/to/a/brawl/mod/sd/card/folder");
let brawl_mod = BrawlMod::new(&brawl_path, Some(&mod_path));

for fighter in brawl_mod.load_fighters(false).unwrap() {
    println!("Fighter name: {}", fighter.cased_name);
    println!("The name of the first model file name: {}", fighter.models[0].name);

    let hl_fighter = HighLevelFighter::new(&fighter);
    println!("Hurtboxes on the 4th frame of 'Run' action {:#?}", hl_fighter.subactions.iter().find(|x| x.name == "Run").unwrap().frames[4].hurt_boxes);
}
```

## Documentation

Refer to [docs.rs](https://docs.rs/brawllib_rs) for the full API.
