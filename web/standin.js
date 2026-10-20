// The stand-in Play screen's picture: replays the flat shapes the core
// works out (crates/core/src/standin/draw.rs) on a 2D canvas, back to
// front. It decides nothing and holds no number: every point, colour and
// line width comes from the core. Kept in this one file, apart from the
// WebGL renderer the real game will use.

export function drawScene(canvas, scene) {
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = scene.bg || "transparent";
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  scene.shapes.forEach((s) => {
    ctx.beginPath();
    s.p.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
    if (s.fill) {
      ctx.closePath();
      ctx.fillStyle = s.fill;
      ctx.fill();
    } else {
      ctx.strokeStyle = s.stroke;
      ctx.lineWidth = s.w;
      ctx.stroke();
    }
  });
}
