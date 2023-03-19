use std::time::Duration;
use std::collections::{VecDeque, HashMap};

use enterpolation::{linear::Linear, Curve};
use rustfft::{FftDirection, Fft};
use rustfft::algorithm::Radix4;
use rustfft::num_complex::Complex;

const BUFFER_TARGET: usize = 3;

struct AudioBuffer {
	data: Box<[f32]>,
	position: usize,
	rate: f32,
}

impl AudioBuffer {
	fn read(&mut self, duration: Duration) -> (&[f32], Duration) {
		let desired_read_count = (duration.as_secs_f32() * self.rate).floor() as usize;
		
		let max_read_count = self.data.len() - self.position;
		let values_to_read = usize::min(max_read_count, desired_read_count);

		let elapsed = Duration::from_secs_f32((values_to_read) as f32 / self.rate);
		let next_position = self.position + values_to_read;

		let data = &self.data[self.position..next_position];

		self.position = next_position;

		(data, elapsed)
	}
}

#[derive(Default)]
pub(crate) struct BufferManager {
	buffers: VecDeque<AudioBuffer>,
	/// key is the power to raise 2 to for the radix size
	ffts: HashMap<u8, Radix4<f32>>,
}

impl BufferManager {
	fn take_next(&mut self, mut interval: Duration) -> Vec<f32> {
		let mut values = Vec::new();
		let mut buffers_taken = 0;

		for buffer in &mut self.buffers {
			let (slice, elapsed) = buffer.read(interval);

			values.extend_from_slice(slice);
			interval = interval.saturating_sub(elapsed);

			// why not is_zero?: because floating point imprecision and rounding
			if interval.as_millis() < 1 {
				break;
			}

			buffers_taken += 1;
		}

		self.buffers.drain(0..buffers_taken);

		values
	}

	pub fn fft_interval<const T: usize>(
		&mut self,
		interval: Duration,
	) -> Option<Box<[f32; T]>> {
		let data = self.take_next(interval);

		if data.len() < 2 {
			return None;
		}

		let power_of_2 = f32::log2(data.len() as f32).floor() as u32;
		let size = 2_u32.pow(power_of_2) as usize;

		let fft = self.ffts.entry(power_of_2 as u8).or_insert_with(|| {
			println!("creating fft of size {}", size);
			Radix4::new(size, FftDirection::Forward)
		});

		let mut truncated_data = data[0..size].iter()
			.cloned()
			.map(|re| Complex { re, im: 0.0 })
			.collect::<Vec<_>>();

		fft.process(truncated_data.as_mut_slice());

		Some(Linear::builder()
			.elements(truncated_data)
			.equidistant::<f32>()
			.normalized()
			.build()
			.unwrap()
			.take(T)
			.map(|Complex { re, .. }| re)
			.collect::<Vec<_>>()
			.into_boxed_slice()
			.try_into().unwrap())
	}

	pub fn fill_buffer(&mut self, buffer: &[f32], rate: u32) {
		if self.buffers.len() >= BUFFER_TARGET {
			// render thread is behind (or not drawing)
			// pause as to not waste resources copying data
			return;
		}

		self.buffers.push_back(AudioBuffer {
			position: 0,
			rate: rate as f32,
			data: Vec::from(buffer).into_boxed_slice(),
		});
	}
}
