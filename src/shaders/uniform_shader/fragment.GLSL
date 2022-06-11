// define fragment shader source code
// fragment shaders are supposed to determine the color at the gl_Position
// system assumes whatever vec4 is outputted is the output color regardless of variable name
// final_color variable is defined to be an output of type vec4

#version 330 core
out vec4 final_color;
uniform vec3 our_color;

void main() {
    final_color = vec4(our_color, 0.0);
}