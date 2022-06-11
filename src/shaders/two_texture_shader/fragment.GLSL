// define fragment shader source code
// fragment shaders are supposed to determine the color at the gl_Position
// system assumes whatever vec4 is outputted is the output color regardless of variable name
// final_color variable is defined to be an output of type vec4

#version 330 core
out vec4 FragColor;
in vec2 tex_coord;
uniform sampler2D our_texture;
uniform sampler2D our_texture2;
uniform float mix_level;

void main()
{
    FragColor = mix(texture(our_texture, tex_coord), texture(our_texture2, tex_coord), mix_level);
}