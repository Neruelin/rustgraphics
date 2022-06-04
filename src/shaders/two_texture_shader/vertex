// define vertex shader program as a string
// vertex shader is supposed to read in vertex attribute values from the buffer and write to the gl_Position variable optionally modifying the values before writing them
// layout location qualifier specifies the index the variable's data comes from
// aPos variable is defined as input with "in" and as a vec3
// aColor variable is the 3-5 values in the buffer's stride representing rgb
// aTexCoord variable is the 7-8 values in the buffer's stride representing the texel coords

#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;
out vec2 tex_coord;
uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;
uniform mat4 transform;
void main() {
    gl_Position = projection * view * model * transform * vec4(aPos, 1.0);
    tex_coord = aTexCoord;
}
