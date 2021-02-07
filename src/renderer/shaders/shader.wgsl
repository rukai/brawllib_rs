[[location(0)]]
var<in> a_position: vec4<f32>;
[[location(1)]]
var<in> a_color: vec4<f32>;

[[block]]
struct Locals {
    transform: mat4x4<f32>;
};
[[group(0), binding(0)]]
var u_locals: Locals;

[[location(0)]]
var<out> v_color: vec4<f32>;
[[builtin(position)]]
var<out> v_position: vec4<f32>;

[[stage(vertex)]]
fn vs_main() {
    v_position = u_locals.transform * a_position;
    v_color = a_color;
}


[[location(0)]]
var<in> v_color: vec4<f32>;

[[location(0)]]
var<out> f_color: vec4<f32>;

[[stage(fragment)]]
fn fs_main() {
    f_color = v_color;
}
