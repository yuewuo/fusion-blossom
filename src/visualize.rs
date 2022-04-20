//! Visualizer
//! 
//! This module helps visualize the progress of a fusion blossom algorithm
//! 

use crate::serde_json;
use std::fs::File;
use crate::serde::{Serialize};
use std::io::{Write, Seek, SeekFrom};
use crate::chrono::Local;

pub trait FusionVisualizer {
    /// take a snapshot, set `abbrev` to true to save space
    fn snapshot(&self, abbrev: bool) -> serde_json::Value;
}

#[derive(Debug, Serialize)]
pub struct Visualizer {
    /// save to file if applicable
    #[serde(skip)]
    file: Option<File>,
    /// the previous snapshots
    snapshots: Vec<(String, serde_json::Value)>,
}

impl Visualizer {
    /// create a new visualizer with target filename and node layout
    pub fn new(filename: Option<String>) -> std::io::Result<Self> {
        let file = match filename {
            Some(filename) => Some(File::create(filename)?),
            None => None,
        };
        Ok(Self {
            file: file,
            snapshots: Vec::new(),
        })
    }

    /// append another snapshot of the fusion type, and also update the file in case 
    pub fn snapshot<FusionType: FusionVisualizer>(&mut self, name: String, fusion_algorithm: &FusionType) -> std::io::Result<()> {
        self.snapshots.push((name, fusion_algorithm.snapshot(true)));
        if let Some(file) = self.file.as_mut() {
            file.set_len(0)?;  // truncate the file
            file.seek(SeekFrom::Start(0))?;  // move the cursor to the front
            file.write_all(json!({
                "snapshots": &self.snapshots,
            }).to_string().as_bytes())?;
            file.sync_all()?;
        }
        Ok(())
    }

    /// save to file
    pub fn save(&mut self) {

    }
}

const DEFAULT_VISUALIZE_DATA_FOLDER: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/visualize/data/");

pub fn visualize_data_folder() -> String {
    DEFAULT_VISUALIZE_DATA_FOLDER.to_string()
}

pub fn static_visualize_data_filename() -> String {
    format!("static.json")
}

pub fn auto_visualize_data_filename() -> String {
    format!("{}.json", Local::now().format("%Y%m%d-%H-%M-%S%.3f"))
}

pub fn print_visualize_link(filename: &String) {
    let link = format!("http://localhost:8066?filename={}", filename);
    println!("opening link {} (you need to start local server by running ./visualize/server.sh)", link)
}
