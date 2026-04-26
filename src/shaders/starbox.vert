#version 330 core

layout (location = 0) in vec4 aPos;

uniform mat4 view;
uniform mat4 projection;

out float vBrightness;

void main() {
    vBrightness = aPos.w;
    // Remove translation from view matrix so stars are infinitely far
    vec4 world_dir = vec4(aPos.xyz, 0.0);
    gl_Position = projection * view * world_dir;
    // Force to far plane
    gl_Position = projection * view * vec4(aPos.xyz, 0.0);
    // Make stars render as points
    gl_PointSize = aPos.w * 1.5;
}