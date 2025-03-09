use std::fs;
use std::path::{Path, PathBuf};

use crate::arc;
use crate::fighter::Fighter;
use crate::wii_memory::WiiMemory;
use crate::wiird_runner;

use anyhow::{Error, bail};

use fancy_slice::FancySlice;

/// This is very cheap to create, it just contains the passed paths.
/// All the actual work is done in the `load_*` methods.
pub struct BrawlMod {
    brawl_path: PathBuf,
    mod_path: Option<PathBuf>,
}

impl BrawlMod {
    /// This is the main entry point of the library.
    /// Provide the path to a brawl dump and optionally a brawl mod sd card.
    ///
    /// Then you can load various other structs from the BrawlMod methods.
    pub fn new(brawl_path: &Path, mod_path: Option<&Path>) -> BrawlMod {
        BrawlMod {
            brawl_path: brawl_path.to_path_buf(),
            mod_path: mod_path.map(|x| x.to_path_buf()),
        }
    }

    /// Returns Err(..) on failure to read required files from disk.
    /// Fighter specific missing files and errors encountered when parsing data is reported via the `error!()` macro from the log crate.
    /// You will need to use one of these crates to view the logged errors <https://github.com/rust-lang-nursery/log#in-executables>
    pub fn load_fighters(&self, single_model: bool) -> Result<Vec<Fighter>, Error> {
        let brawl_fighter_path = self.brawl_path.join("fighter");
        let brawl_fighter_dir = match fs::read_dir(&brawl_fighter_path) {
            Ok(dir) => dir,
            Err(err) => bail!("Cannot read fighter directory in the brawl dump: {}", err),
        };

        let mut mod_fighter_dir = None;
        if let Some(mod_path) = &self.mod_path {
            let dir_reader = match fs::read_dir(mod_path) {
                Ok(dir) => dir,
                Err(err) => bail!("Cannot read brawl mod directory: {}", err),
            };

            for dir in dir_reader.flatten() {
                let path = dir.path().join("pf/fighter");
                if path.exists() {
                    mod_fighter_dir = Some(match fs::read_dir(path) {
                        Ok(dir) => dir,
                        Err(err) => {
                            bail!("Cannot read fighter directory in the brawl mod: {}", err)
                        }
                    });
                    break;
                }
            }
        }
        if self.mod_path.is_some() && mod_fighter_dir.is_none() {
            bail!("Missing mod_name/pf/fighter directory");
        }

        let common_fighter_path = brawl_fighter_path.join("Fighter.pac");
        let (common_fighter, wii_memory) =
            if let Ok(mut file_data) = std::fs::read(common_fighter_path) {
                let wii_memory = if self.mod_path.is_some() {
                    let codeset = self.load_wiird_codeset_raw()?;
                    let sakurai_ram_offset = 0x80F9FC20;
                    let sakurai_fighter_pac_offset = 0x80;
                    let fighter_pac_offset = sakurai_ram_offset - sakurai_fighter_pac_offset;

                    wiird_runner::process(&codeset, &mut file_data, fighter_pac_offset)
                } else {
                    WiiMemory::new()
                };

                let data = FancySlice::new(&file_data);

                (arc::arc(data, &wii_memory, false), wii_memory)
            } else {
                bail!("Missing Fighter.pac");
            };

        Ok(Fighter::load(
            brawl_fighter_dir,
            mod_fighter_dir,
            &common_fighter,
            &wii_memory,
            single_model,
        ))
    }

    pub fn load_wiird_codeset_raw(&self) -> Result<Vec<u8>, Error> {
        // RSBE01.gct is usually located in the codes folder but can also be in the main sub folder e.g. LXP 2.1
        // Additionally P+ now has a second codeset file called BOOST.GCT
        // So we will load every *.gct file from within every subdirectory of the root.
        // If we have already read a file of the same name, we assert the contents are equal, then skip it.
        // Once all files are loaded, strip the headers and concatenate them in a deterministic fashion.

        struct GCTFile {
            pub name: String,
            pub data: Vec<u8>,
        }

        let mut gct_files: Vec<GCTFile> = vec![];
        if let Some(mod_path) = &self.mod_path {
            for entry in fs::read_dir(mod_path).unwrap().flatten() {
                if entry.path().is_dir() {
                    let child_dir = entry.path();
                    for entry in fs::read_dir(child_dir).unwrap().flatten() {
                        let name = entry.file_name().into_string().unwrap();
                        if name.ends_with(".gct") || name.ends_with(".GCT") {
                            let codeset_path = entry.path();
                            if codeset_path.exists() {
                                match std::fs::read(&codeset_path) {
                                    Ok(data) => {
                                        if data.len() < 8 {
                                            bail!(
                                                "Not a WiiRD gct codeset file: File size is less than 8 bytes"
                                            );
                                        }
                                        if let Some(matching_file) =
                                            gct_files.iter().find(|x| x.name == name)
                                        {
                                            assert_eq!(matching_file.data, data);
                                        } else {
                                            gct_files.push(GCTFile { name, data });
                                        }
                                    }
                                    Err(err) => bail!(
                                        "Cannot read WiiRD codeset {:?}: {}",
                                        codeset_path,
                                        err
                                    ),
                                };
                            }
                        }
                    }
                }
            }
        } else {
            bail!("Not a mod, vanilla brawl does not have a WiiRD codeset.");
        }

        // Very important that the resulting wiird_codeset is deterministic
        // Will make issues much easier to reproduce.
        gct_files.sort_by_key(|x| x.name.clone());

        let mut result = vec![];
        for gct_file in gct_files {
            result.extend(&gct_file.data[8..]); // skip the header
        }
        Ok(result)
    }

    /// returns true if modded files are used.
    /// Otherwise is just vanilla brawl and false is returned.
    pub fn is_mod(&self) -> bool {
        self.mod_path.is_some()
    }
}
