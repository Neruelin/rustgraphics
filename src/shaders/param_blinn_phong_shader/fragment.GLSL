#version 330 core
out vec4 FragColor;

in VS_OUT {
    vec3 FragPos;
    vec3 Normal;
} fs_in;

uniform vec3 lightPos;
uniform vec3 viewPos;

uniform vec3 ambient_color;
uniform vec3 diffuse_color;
uniform vec3 specular_color;

uniform float optical_density;
uniform float dissolve;

void main()
{           
    // ambient
    vec3 ambient = 0.05 * ambient_color;
    // diffuse
    vec3 lightDir = normalize(lightPos - fs_in.FragPos);
    vec3 normal = normalize(fs_in.Normal);
    float diff = max(dot(lightDir, normal), 0.0);
    vec3 diffuse = diff * diffuse_color;
    // specular
    // vec3 viewDir = normalize(viewPos - fs_in.FragPos);
    // vec3 reflectDir = reflect(-lightDir, normal);
    // float spec = 0.0;
    // vec3 halfwayDir = normalize(lightDir + viewDir);  
    // spec = pow(max(dot(normal, halfwayDir), 0.0), 32.0);
    // vec3 specular = spec * specular_color;
     
    // FragColor = vec4(ambient + diffuse + specular, 1.0);
    FragColor = vec4(ambient + diffuse, 1.0);
}