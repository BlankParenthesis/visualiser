#![feature(once_cell)]

mod graphics;
mod window;
mod audio;
mod visualiser;

use std::{sync::{Arc, RwLock}, path::PathBuf};

use window::Window;
use visualiser::BufferManager;

const BUFFER_SIZE: usize = 512;

use clap::Parser;

#[derive(Debug, Parser)]
struct Arguments {
	/// Shift the spectrum to show more detail at the lower frequencies at
	/// values greater than 1 and higher frequencies at less than 1.
	#[arg(short, long, default_value_t = 1.02)]
	power_scale_frequencies: f32,
	/// Clip the spectrum to have this frequency be the highest pitch
	#[arg(short, long, default_value_t = 15000.0)]
	ceiling_frequency: f32,
	/// Clip the spectrum to have this frequency be the lowest pitch
	#[arg(short, long, default_value_t = 0.0)]
	floor_frequency: f32,
	/// Multiply the output levels by the value
	#[arg(short, long, default_value_t = 1.0)]
	scale: f32,
	/// Path to the obj file to use for displaying data
	layout: Option<PathBuf>, 
}

lazy_static::lazy_static! {
	static ref CONFIG: Arguments = Arguments::parse();
}

fn main() {
	let buffer_manager = Arc::new(RwLock::new(BufferManager::default()));

	audio::main(Arc::clone(&buffer_manager));

	let mut window = Window::new(buffer_manager);
	window.run();
}
