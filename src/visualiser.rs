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

struct FftCache {
	algorithm: Radix4<f32>,
	window: Box<[f32]>,
	scaling_factor: f32,
}

#[derive(Default)]
pub(crate) struct BufferManager {
	buffers: VecDeque<AudioBuffer>,
	/// key is the power to raise 2 to for the radix size
	ffts: HashMap<u8, FftCache>,
	flip: bool,
}

struct BufferSlice {
	values: Vec<f32>,
	rate: f32,
}

impl BufferManager {
	fn take_next(&mut self, interval: Duration) -> BufferSlice {
		let mut values = Vec::new();
		let mut buffers_taken = 0;
		let mut rate = 0.0;
		let mut remaining_interval = interval;
		let interval = interval.as_secs_f32();

		for buffer in &mut self.buffers {
			let buffer_rate = buffer.rate;
			let (slice, elapsed) = buffer.read(remaining_interval);

			rate += buffer_rate * elapsed.as_secs_f32() / interval;

			values.extend_from_slice(slice);
			remaining_interval = remaining_interval.saturating_sub(elapsed);

			// why not is_zero?: because floating point imprecision and rounding
			if remaining_interval.as_millis() < 1 {
				break;
			}

			buffers_taken += 1;
		}

		// to account for any remaining time, scale up the existing rate
		let total_elapsed = interval - remaining_interval.as_secs_f32();
		rate /= total_elapsed / interval;

		self.buffers.drain(0..buffers_taken);

		BufferSlice { values, rate }
	}

	pub fn fft_interval<const T: usize>(
		&mut self,
		interval: Duration,
	) -> Option<Box<[f32; T]>> {
		let BufferSlice { values, rate } = self.take_next(interval);

		if values.len() < 2 {
			return None;
		}

		let power_of_2 = f32::log2(values.len() as f32).floor() as u32;
		let size = 2_u32.pow(power_of_2) as usize;

		let fft = self.ffts.entry(power_of_2 as u8).or_insert_with(|| {
			FftCache {
				algorithm: Radix4::new(size, FftDirection::Forward),
				window: apodize::hamming_iter(size).map(|v| v as f32).collect(),
				scaling_factor: (size as f32).sqrt(),
			}
		});

		let mut truncated_data = values[0..size].iter()
			.cloned()
			.zip(fft.window.iter())
			.map(|(val, scale)| Complex { re: val * scale, im: 0.0 })
			.collect::<Vec<_>>();

		fft.algorithm.process(truncated_data.as_mut_slice());

		// NOTE: taking anything > rate/2 results in Hermitian symmetry
		const TARGET_MAX_FREQUENCY: f32 = 15_000.0;
		let frequency_ratio = TARGET_MAX_FREQUENCY / rate;
		let max_index = usize::min(size, (size as f32 * frequency_ratio) as usize);

		if max_index < 2 {
			// enterpolation needs at least two values
			return None;
		}
		
		fn power_range(base: f32, count: usize) -> Box<[f32]> {
			let power_data = (0..(count - 1))
				.map(|power| 1.0 - 1.0 / base.powf(power as f32));

			[0.0].into_iter().chain(power_data).collect()
		}

		const SCALE_POWER_BASE: f32 = 1.02;

		Some(Linear::builder()
			.elements(&truncated_data[0..max_index])
			.knots(power_range(SCALE_POWER_BASE, max_index).as_ref())
			//.equidistant::<f32>()
			//.normalized()
			.build()
			.unwrap()
			.take(T)
			.map(|Complex { re, im }| {
				let power = f32::sqrt(re * re + im * im);
				let value = power / fft.scaling_factor;
				let log_scale = f32::log10(1.0 + value);
				
				const SCALE: f32 = 1.0;
				log_scale * SCALE
			})
			.collect::<Box<_>>()
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
