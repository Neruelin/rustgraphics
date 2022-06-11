// define vertex shader program as a string
// vertex shader is supposed to read in vertex attribute values from the buffer and write to the gl_Position variable optionally modifying the values before writing them
// layout location qualifier specifies the index the variable's data comes from
// pos variable is defined as input with "in" and as a vec3

#version 330 core

layout (location = 0) in vec3 aPos;
uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;
uniform mat4 transform;
void main() {
    gl_Position = projection * view * model * transform * vec4(aPos, 1.0);
}