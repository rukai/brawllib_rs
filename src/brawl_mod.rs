use std::fs::File;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::fighter::Fighter;
use crate::wii_memory::WiiMemory;
use crate::wiird::WiiRDBlock;
use crate::wiird;
use crate::wiird_runner;
use crate::arc;

use failure::Error;
use failure::bail;

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
    /// You will need to use one of these crates to view the logged errors https://github.com/rust-lang-nursery/log#in-executables
    pub fn load_fighters(&self, single_model: bool) -> Result<Vec<Fighter>, Error> {
        let brawl_fighter_path = self.brawl_path.join("fighter");
        let brawl_fighter_dir = match fs::read_dir(&brawl_fighter_path) {
            Ok(dir) => dir,
            Err(err) => bail!("Cannot read fighter directory in the brawl dump: {}", err)
        };

        let mut mod_fighter_dir = None;
        if let Some(mod_path) = &self.mod_path {
            for dir in fs::read_dir(mod_path).unwrap() {
                if let Ok(dir) = dir {
                    let path = dir.path().join("pf/fighter");
                    if path.exists() {
                        mod_fighter_dir = Some(match fs::read_dir(path) {
                            Ok(dir) => dir,
                            Err(err) => bail!("Cannot read fighter directory in the brawl mod: {}", err),
                        });
                        break;
                    }
                }
            }
        }
        if self.mod_path.is_some() && mod_fighter_dir.is_none() {
            bail!("Missing mod_name/pf/fighter directory");
        }

        let common_fighter_path = brawl_fighter_path.join("Fighter.pac");
        let (common_fighter, wii_memory) = if let Ok(mut fighter_file) = File::open(common_fighter_path) {
            let mut file_data: Vec<u8> = vec!();
            if let Err(err) = fighter_file.read_to_end(&mut file_data) {
                bail!("Cannot read Fighter.pac in the brawl dump: {}", err);
            }

            let wii_memory = if self.mod_path.is_some() {
                let codeset = self.load_wiird_codeset()?;
                let sakurai_ram_offset = 0x80F9FC20;
                let sakurai_fighter_pac_offset = 0x80;
                let fighter_pac_offset = sakurai_ram_offset - sakurai_fighter_pac_offset;

                wiird_runner::process(&codeset, &mut file_data, fighter_pac_offset)
            } else {
                WiiMemory::new()
            };

            (arc::arc(&file_data, &wii_memory), wii_memory)
        } else {
            bail!("Missing Fighter.pac");
        };

        Ok(Fighter::load(brawl_fighter_dir, mod_fighter_dir, &common_fighter, &wii_memory, single_model))
    }

    pub fn load_wiird_codeset(&self) -> Result<WiiRDBlock, Error> {
        if let Some(mod_path) = &self.mod_path {
            wiird::wiird_load_gct(&mod_path.join("codes/RSBE01.gct"))
        } else {
            bail!("Not a mod, vanilla brawl does not have a WiiRD codeset.");
        }
    }
}
