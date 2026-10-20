// The WebGL 2 renderer. It uploads what the core returns and draws it;
// every size, stride, offset and uniform slot comes from the core's
// numbers(), so this file holds no layout of its own. (The machinery
// follows Honey Hole's gl.js, after Ape Race's; the look is the core's.)
//
// Each frame, in order: the solid triangles; the water behind them, on the
// far plane, only where none of those lies; and last the see-through
// triangles (a ghost tile), which the core has sorted far to near,
// blended over the rest without writing depth.

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader));
  }
  return shader;
}

function link(gl, vertexSource, fragmentSource) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertexSource));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragmentSource));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program));
  }
  return program;
}

// A program and a setter for the view values it reads.
function program(gl, N, vertexSource, fragmentSource) {
  const p = link(gl, vertexSource, fragmentSource);
  const uVP = gl.getUniformLocation(p, "uVP");
  const uniforms = N.uniforms
    .map(([name, size, at, call]) => ({ loc: gl.getUniformLocation(p, name), size, at, call }))
    .filter((u) => u.loc);
  return {
    use(view) {
      gl.useProgram(p);
      if (uVP) gl.uniformMatrix4fv(uVP, false, view.subarray(...N.matrix));
      uniforms.forEach((u) => gl[u.call](u.loc, view.subarray(u.at, u.at + u.size)));
    },
    // the attributes this program reads, of a list of [name, size, offset]
    attributes(list) {
      return list.map(([name, size, offset]) => ({ loc: gl.getAttribLocation(p, name), size, offset })).filter((a) => a.loc >= 0);
    },
  };
}

// Points attributes at the bound buffer.
function point(gl, attributes, stride) {
  attributes.forEach((a) => {
    gl.enableVertexAttribArray(a.loc);
    gl.vertexAttribPointer(a.loc, a.size, gl.FLOAT, false, stride, a.offset);
  });
}

// Returns null when the browser has no WebGL 2.
export function makeRenderer(canvas, N, shaders) {
  const gl = canvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
  if (!gl) return null;

  const scene = program(gl, N, shaders.vertex, shaders.fragment);
  const sky = program(gl, N, shaders.skyVertex, shaders.skyFragment);
  gl.enable(gl.DEPTH_TEST);
  gl.disable(gl.CULL_FACE);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  const triangles = gl.createBuffer();
  const corners = gl.createBuffer();
  const sceneVao = gl.createVertexArray();
  gl.bindVertexArray(sceneVao);
  gl.bindBuffer(gl.ARRAY_BUFFER, triangles);
  point(gl, scene.attributes(N.vertex.attributes), N.vertex.stride);
  const skyVao = gl.createVertexArray();
  gl.bindVertexArray(skyVao);
  gl.bindBuffer(gl.ARRAY_BUFFER, corners);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(N.sky), gl.STATIC_DRAW);
  point(gl, sky.attributes([["aCorner", N.sky_size, 0]]), 0);
  gl.bindVertexArray(null);

  function draw(vertices, count) {
    gl.bindVertexArray(sceneVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, triangles);
    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STREAM_DRAW);
    gl.drawArrays(gl.TRIANGLES, 0, count);
  }

  return {
    resize(width, height) {
      canvas.width = width;
      canvas.height = height;
      gl.viewport(0, 0, width, height);
    },
    // view: the core's view values; solid and clear: the triangles, each
    // with its vertex count
    frame(view, solid, solidCount, clear, clearCount) {
      if (!view.length) {
        gl.clearColor(0, 0, 0, 0);
        gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
        return;
      }
      const [r, g, b] = view.subarray(...N.fog_colour);
      gl.clearColor(r, g, b, 1);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      scene.use(view);
      if (solidCount) draw(solid, solidCount);
      // the far plane passes where nothing nearer was drawn
      sky.use(view);
      gl.bindVertexArray(skyVao);
      gl.depthFunc(gl.LEQUAL);
      gl.depthMask(false);
      gl.drawArrays(gl.TRIANGLES, 0, N.sky_count);
      gl.depthFunc(gl.LESS);
      if (clearCount) {
        scene.use(view);
        gl.enable(gl.BLEND);
        draw(clear, clearCount);
        gl.disable(gl.BLEND);
      }
      gl.depthMask(true);
      gl.bindVertexArray(null);
    },
  };
}
