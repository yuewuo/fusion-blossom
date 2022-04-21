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
pub struct VisualizePosition {
    /// vertical axis, -i is up, +i is down (left-up corner is smallest i,j)
    i: f64,
    /// horizontal axis, -j is left, +j is right (left-up corner is smallest i,j)
    j: f64,
    /// time axis, top and bottom (orthogonal to the initial view, which looks at -t direction)
    t: f64,
}

impl VisualizePosition {
    /// create a visualization position
    pub fn new(i: f64, j: f64, t: f64) -> Self {
        Self {
            i: i, j: j, t: t
        }
    }
}

#[derive(Debug)]
pub struct Visualizer {
    /// save to file if applicable
    file: Option<File>,
    /// basic snapshot
    base: serde_json::Value,
    /// positions of the nodes
    positions: Vec<VisualizePosition>,
    /// all snapshots
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
            base: json!({}),
            positions: Vec::new(),
            snapshots: Vec::new(),
        })
    }

    /// append another snapshot of the fusion type, and also update the file in case 
    pub fn snapshot<FusionType: FusionVisualizer>(&mut self, name: String, fusion_algorithm: &FusionType) -> std::io::Result<()> {
        self.snapshots.push((name, fusion_algorithm.snapshot(true)));
        self.save()?;
        Ok(())
    }

    /// save to file
    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(file) = self.file.as_mut() {
            file.set_len(0)?;  // truncate the file
            file.seek(SeekFrom::Start(0))?;  // move the cursor to the front
            file.write_all(json!({
                "base": &self.base,
                "snapshots": &self.snapshots,
                "positions": &self.positions,
            }).to_string().as_bytes())?;
            file.sync_all()?;
        }
        Ok(())
    }

    /// set positions of the node and optionally center all positions
    pub fn set_positions(&mut self, mut positions: Vec<VisualizePosition>, center: bool) {
        if center {
            let (mut ci, mut cj, mut ct) = (0., 0., 0.);
            for position in positions.iter() {
                ci += position.i;
                cj += position.j;
                ct += position.t;
            }
            ci /= positions.len() as f64;
            cj /= positions.len() as f64;
            ct /= positions.len() as f64;
            for position in positions.iter_mut() {
                position.i -= ci;
                position.j -= cj;
                position.t -= ct;
            }
        }
        self.positions = positions;
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
