#version 330 core

layout (location = 0) in vec3 aPos;

uniform mat4 projection;
uniform mat4 view;
uniform mat4 model;
uniform float u_seam; // 0.0 to 1.0, fraction along the orbit where the craft is
uniform int u_num_vertices;

out float vAlpha;

void main() {
    // gl_VertexID gives us which vertex we are, 0 to u_num_vertices
    float t = float(gl_VertexID) / float(u_num_vertices);
    float dist = mod(t - u_seam + 1.0, 1.0); // circular distance from seam
    vAlpha = 1.0 - pow(0.894 * (dist - 1.0), 2.0);

    gl_Position = (projection * view * model * vec4(aPos, 1.0));
}
