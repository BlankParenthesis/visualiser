#version 450

layout (binding = 0) uniform sampler1D frequency_magnitude;

layout (location = 0) in float frag_frequency;
layout (location = 1) in float target_amplitude;

layout (location = 0) out vec4 color;

void main() {
	float amplitude = texture(frequency_magnitude, frag_frequency).r;

	if (amplitude > target_amplitude) {
		color = vec4(0.5, 0.7, 0.8, 1.0);
	} else {
		color = vec4(0.0, 0.0, 0.0, 0.0);
	}
}