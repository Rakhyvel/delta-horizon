#version 330 core

in float vAlpha;
uniform vec4 u_color;

out vec4 Color;

void main()
{
    Color = vec4(u_color.rgb, u_color.a * vAlpha);
}