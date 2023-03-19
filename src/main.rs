mod graphics;
mod window;
mod audio;
mod visualiser;

use std::sync::{Arc, RwLock};

use window::Window;
use visualiser::BufferManager;

const BUFFER_SIZE: usize = 10;

fn main() {
	let buffer_manager = Arc::new(RwLock::new(BufferManager::default()));

	audio::main(Arc::clone(&buffer_manager));

	let mut window = Window::new(buffer_manager);
	window.run();
}
