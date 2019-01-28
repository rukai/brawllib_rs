# brawllib\_rs [![Build Status](https://travis-ci.org/rukai/brawllib_rs.svg?branch=master)](https://travis-ci.org/rukai/brawllib_rs) [![dependency status](https://deps.rs/repo/github/rukai/brawllib_rs/status.svg)](https://deps.rs/repo/github/rukai/brawllib_rs) [![Crates.io](https://img.shields.io/crates/v/brawllib_rs.svg)](https://crates.io/crates/brawllib_rs)

A brawl character file parser, based on brawlbox/brawllib, written in rust.

Doesn't support modifying files, only reading them.

## Example

In this example we:
*   Create a Fighter from an exported brawl fighter directory.
    This is the raw data from the fighter folder, stored in a tree of structs
*   Create a HighLevelFighter from an exported brawl fighter directory.
    This contains processed data from the Fighter struct, stored in a tree of structs
```rust
use brawllib_rs::fighter::Fighter;
use brawllib_rs::high_level_fighter::HighLevelFighter;
use std::fs;

for fighter in Fighter::load(fs::read_dir("some/real/dir/fighter").unwrap(), None, false) {
    println!("Fighter name: {}", fighter.cased_name);
    println!("The name of the first model file name: {}", fighter.models[0].name);

    let hl_fighter = HighLevelFighter::new(&fighter);
    println!("Hurtboxes on the 4th frame of 'Run' action {:#?}", hl_fighter.subactions.iter().find(|x| x.name == "Run").unwrap().frames[4].hurt_boxes);
}
```

## Documentation

Refer to [docs.rs](https://docs.rs/brawllib_rs) for the full API.
