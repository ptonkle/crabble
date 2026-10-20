#version 300 es
precision highp float;
// Flat low-poly light under the water (after Honey Hole's): one warm sun
// from above, a fill from the water overhead to the sand below blended by
// which way the face points, the caustic net the ripples make playing on
// faces that look up, and distance fog that fades to the murk behind it
// (as sky.frag draws it: lighter looking up, darker looking down, with
// soft light shafts). An unlit (glowing) colour skips the light and takes
// only part of the fog. A see-through face keeps its alpha.
// Every value comes from the core as a uniform (crates/core/src/gfx/render.rs).
in vec3 vWorld;
in vec3 vNormal;
in vec3 vColour;
in float vGlow;
in float vAlpha;
uniform vec3 uEye;
uniform vec2 uFog;
uniform float uFogCurve;
uniform vec3 uFogColour;
uniform vec3 uZenith;
uniform vec2 uSkyBand;
uniform vec3 uDeep;
uniform vec2 uDeepBand;
uniform vec4 uShaft;
uniform vec4 uShaftLook;
uniform vec3 uShaftDir;
uniform vec3 uSheen;
uniform vec4 uRipple1;
uniform vec4 uRipple2;
uniform vec4 uRipple3;
uniform vec4 uCaustic;
uniform vec3 uSunDir;
uniform vec3 uSun;
uniform vec3 uSky;
uniform vec3 uGround;
uniform vec2 uHemi;
uniform float uGlow;
out vec4 outColour;
// The fog's share at distance d: from uFog.x to whole at uFog.y, quickly at
// first by uFogCurve (k: (1 - e^(-k t)) / (1 - e^(-k))), or an even S-curve
// when it is 0.
float fogAt(float d) {
  if (uFogCurve <= 0.0) return smoothstep(uFog.x, uFog.y, d);
  float t = clamp((d - uFog.x) / (uFog.y - uFog.x), 0.0, 1.0);
  return (1.0 - exp(-uFogCurve * t)) / (1.0 - exp(-uFogCurve));
}
vec3 murk(vec3 d) {
  vec3 c = mix(uFogColour, uZenith, smoothstep(uSkyBand.x, uSkyBand.y, d.y));
  c = mix(c, uDeep, smoothstep(uDeepBand.x, uDeepBand.y, -d.y));
  if (uShaftLook.x <= 0.0) return c;
  vec3 a = normalize(cross(uShaftDir, vec3(1.0, 0.0, 0.0)));
  vec3 b = cross(uShaftDir, a);
  float turn = atan(dot(d, b), dot(d, a));
  float rays = max(sin(turn * uShaft.x + uShaft.z) + sin(turn * uShaft.y + uShaft.w) - 1.0, 0.0);
  float fade = smoothstep(uShaftLook.z, uShaftLook.w, dot(d, uShaftDir));
  return c + uSheen * pow(rays, uShaftLook.y) * uShaftLook.x * fade;
}
// The caustic net: bright thin lines where three ripples cancel.
float caustic(vec2 p) {
  float s = sin(dot(p, uRipple1.xy) + uRipple1.z) + sin(dot(p, uRipple2.xy) + uRipple2.z) + sin(dot(p, uRipple3.xy) + uRipple3.z);
  return pow(clamp(1.0 - abs(s) * uCaustic.x, 0.0, 1.0), uCaustic.y);
}
void main() {
  vec3 n = normalize(vNormal);
  // either winding: a face seen from behind is lit on the side seen
  if (!gl_FrontFacing) n = -n;
  float sun = max(dot(n, uSunDir), 0.0);
  vec3 fill = mix(uGround, uSky, clamp(n.y * uHemi.x + uHemi.y, 0.0, 1.0));
  vec3 lit = mix(vColour * (fill + uSun * sun), vColour, vGlow);
  if (uCaustic.z > 0.0) {
    lit += uSheen * caustic(vWorld.xz) * uCaustic.z * clamp(n.y, 0.0, 1.0) * (1.0 - vGlow);
  }
  vec3 ray = vWorld - uEye;
  float f = fogAt(length(ray)) * mix(1.0, uGlow, vGlow);
  // the murk only where the fog has begun: mixing in none of it is the same
  vec3 sky = f > 0.0 ? murk(normalize(ray)) : lit;
  outColour = vec4(mix(lit, sky, f), vAlpha);
}
