#version 450

layout (binding = 0) uniform sampler1D frequency_magnitude;

layout (location = 0) in float frag_frequency;
layout (location = 1) in float target_amplitude;

layout (location = 0) out vec4 color;


vec3 hsv_to_rgb(float h) {
	vec3 hsv = vec3(h, 1.0, 1.0);

    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(hsv.xxx + K.xyz) * 6.0 - K.www);
    return hsv.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), hsv.y);
}

void main() {
	float amplitude = texture(frequency_magnitude, frag_frequency).r;

	if (amplitude > target_amplitude) {
		color = vec4(hsv_to_rgb(frag_frequency), 1.0);
	} else {
		color = vec4(0.0, 0.0, 0.0, 0.0);
	}
}