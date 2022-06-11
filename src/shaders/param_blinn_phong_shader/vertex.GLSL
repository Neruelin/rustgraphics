#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;

// declare an interface block; see 'Advanced GLSL' for what these are.
out VS_OUT {
    vec3 FragPos;
    vec3 Normal;
} vs_out;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;
uniform mat4 rotation;

void main()
{
    vs_out.FragPos = aPos;
    vs_out.Normal = (rotation * vec4(aNormal, 1.0)).xyz;
    gl_Position = projection * view * model * vec4(aPos, 1.0);
}