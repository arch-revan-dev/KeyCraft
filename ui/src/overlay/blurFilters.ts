/**
 * Deterministic offline censorship filters operating directly on Canvas2D ImageData:
 * - Pixelation (block-averaging mosaic)
 * - Fast separable 2-pass Gaussian Blur (for hiding passwords, emails, keys)
 */

export function applyPixelate(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  blockSize = 10
): void {
  const rx = Math.max(0, Math.floor(x));
  const ry = Math.max(0, Math.floor(y));
  const rw = Math.min(ctx.canvas.width - rx, Math.ceil(w));
  const rh = Math.min(ctx.canvas.height - ry, Math.ceil(h));
  if (rw <= 1 || rh <= 1) return;

  const img = ctx.getImageData(rx, ry, rw, rh);
  const d = img.data;
  const step = Math.max(4, Math.round(blockSize));

  for (let by = 0; by < rh; by += step) {
    const bh = Math.min(step, rh - by);
    for (let bx = 0; bx < rw; bx += step) {
      const bw = Math.min(step, rw - bx);
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      const count = bw * bh;

      for (let py = 0; py < bh; py++) {
        for (let px = 0; px < bw; px++) {
          const idx = ((by + py) * rw + (bx + px)) * 4;
          r += d[idx];
          g += d[idx + 1];
          b += d[idx + 2];
          a += d[idx + 3];
        }
      }

      const ar = Math.round(r / count);
      const ag = Math.round(g / count);
      const ab = Math.round(b / count);
      const aa = Math.round(a / count);

      for (let py = 0; py < bh; py++) {
        for (let px = 0; px < bw; px++) {
          const idx = ((by + py) * rw + (bx + px)) * 4;
          d[idx] = ar;
          d[idx + 1] = ag;
          d[idx + 2] = ab;
          d[idx + 3] = aa;
        }
      }
    }
  }

  ctx.putImageData(img, rx, ry);
}

function buildGaussianKernel(radius: number): Float32Array {
  const sigma = Math.max(1, radius / 2.2);
  const size = radius * 2 + 1;
  const kernel = new Float32Array(size);
  let sum = 0;
  for (let i = -radius; i <= radius; i++) {
    const val = Math.exp(-(i * i) / (2 * sigma * sigma));
    kernel[i + radius] = val;
    sum += val;
  }
  for (let i = 0; i < size; i++) {
    kernel[i] /= sum;
  }
  return kernel;
}

export function applyGaussianBlur(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  radius = 12
): void {
  const rx = Math.max(0, Math.floor(x));
  const ry = Math.max(0, Math.floor(y));
  const rw = Math.min(ctx.canvas.width - rx, Math.ceil(w));
  const rh = Math.min(ctx.canvas.height - ry, Math.ceil(h));
  if (rw <= 1 || rh <= 1) return;

  const img = ctx.getImageData(rx, ry, rw, rh);
  const src = img.data;
  const temp = new Uint8ClampedArray(src.length);
  const dst = new Uint8ClampedArray(src.length);
  const r = Math.max(2, Math.min(24, Math.round(radius)));
  const kernel = buildGaussianKernel(r);

  // Horizontal pass
  for (let row = 0; row < rh; row++) {
    const rowOff = row * rw;
    for (let col = 0; col < rw; col++) {
      let cr = 0;
      let cg = 0;
      let cb = 0;
      let ca = 0;
      for (let k = -r; k <= r; k++) {
        const sc = Math.max(0, Math.min(rw - 1, col + k));
        const idx = (rowOff + sc) * 4;
        const wgt = kernel[k + r];
        cr += src[idx] * wgt;
        cg += src[idx + 1] * wgt;
        cb += src[idx + 2] * wgt;
        ca += src[idx + 3] * wgt;
      }
      const out = (rowOff + col) * 4;
      temp[out] = cr;
      temp[out + 1] = cg;
      temp[out + 2] = cb;
      temp[out + 3] = ca;
    }
  }

  // Vertical pass
  for (let row = 0; row < rh; row++) {
    for (let col = 0; col < rw; col++) {
      let cr = 0;
      let cg = 0;
      let cb = 0;
      let ca = 0;
      for (let k = -r; k <= r; k++) {
        const sr = Math.max(0, Math.min(rh - 1, row + k));
        const idx = (sr * rw + col) * 4;
        const wgt = kernel[k + r];
        cr += temp[idx] * wgt;
        cg += temp[idx + 1] * wgt;
        cb += temp[idx + 2] * wgt;
        ca += temp[idx + 3] * wgt;
      }
      const out = (row * rw + col) * 4;
      dst[out] = cr;
      dst[out + 1] = cg;
      dst[out + 2] = cb;
      dst[out + 3] = ca;
    }
  }

  img.data.set(dst);
  ctx.putImageData(img, rx, ry);
}
