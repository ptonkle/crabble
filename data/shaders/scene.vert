#version 300 es
// One vertex of a triangle list already placed in the world by the core:
// its position, its face's flat normal, its face's colour, its glow (0 lit,
// 1 unlit) and its alpha (1 solid).
in vec3 aPos;
in vec3 aNormal;
in vec3 aColour;
in float aGlow;
in float aAlpha;
uniform mat4 uVP;
out vec3 vWorld;
out vec3 vNormal;
out vec3 vColour;
out float vGlow;
out float vAlpha;
void main() {
  vWorld = aPos;
  vNormal = aNormal;
  vColour = aColour;
  vGlow = aGlow;
  vAlpha = aAlpha;
  gl_Position = uVP * vec4(aPos, 1.0);
}
