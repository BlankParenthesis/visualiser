#version 450

layout (binding = 0) uniform sampler1D frequency_magnitude;

layout (location = 0) in vec2 vertex_position;
layout (location = 1) in float vertex_frequency;
layout (location = 2) in float vertex_amplitude;

layout (location = 0) out float frag_frequency;
layout (location = 1) out float target_amplitude;

void main() {
	frag_frequency = vertex_frequency;
	target_amplitude = vertex_amplitude;
	gl_Position = vec4(vertex_position, 0.0, 1.0);
}