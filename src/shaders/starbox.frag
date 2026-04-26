#version 330 core

in float vBrightness;
out vec4 Color;

void main() {
    Color = vec4(vec3(vBrightness), vBrightness);
}