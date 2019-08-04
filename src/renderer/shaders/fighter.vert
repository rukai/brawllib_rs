#version 450

layout(location = 0) in vec4 a_pos;
layout(location = 1) in vec4 a_color;

layout(location = 0) out vec4 v_color;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_transform;
};

void main() {
    gl_Position = u_transform * a_pos;
    v_color = a_color;
}
