import { applyGaussianBlur, applyPixelate } from './blurFilters';

export type AnnotationTool =
  | 'arrow'
  | 'rect'
  | 'ellipse'
  | 'line'
  | 'highlighter'
  | 'text'
  | 'step'
  | 'pixelate'
  | 'blur';

export interface SelectionBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface AnnotationItem {
  id: number;
  tool: AnnotationTool;
  color: string;
  strokeWidth: number;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  points?: { x: number; y: number }[];
  text?: string;
  stepNumber?: number;
}

export const CALM_COLORS = [
  '#DC2626',
  '#D97706',
  '#16A34A',
  '#2563EB',
  '#18181B',
  '#FFFFFF',
] as const;

export function normalizeRect(x1: number, y1: number, x2: number, y2: number): SelectionBox {
  const x = Math.min(x1, x2);
  const y = Math.min(y1, y2);
  return {
    x,
    y,
    w: Math.abs(x2 - x1),
    h: Math.abs(y2 - y1),
  };
}

export function drawAnnotation(
  ctx: CanvasRenderingContext2D,
  item: AnnotationItem,
  offsetX = 0,
  offsetY = 0,
  scaleX = 1,
  scaleY = 1
): void {
  const x1 = item.x1 - offsetX;
  const y1 = item.y1 - offsetY;
  const x2 = item.x2 - offsetX;
  const y2 = item.y2 - offsetY;

  // Pixelate & Gaussian Blur operate on raw physical ImageData pixels (ignoring canvas transform)
  if (item.tool === 'pixelate') {
    const r = normalizeRect(x1 * scaleX, y1 * scaleY, x2 * scaleX, y2 * scaleY);
    applyPixelate(ctx, r.x, r.y, r.w, r.h, Math.max(8, item.strokeWidth * 3 * scaleX));
    return;
  }

  if (item.tool === 'blur') {
    const r = normalizeRect(x1 * scaleX, y1 * scaleY, x2 * scaleX, y2 * scaleY);
    applyGaussianBlur(ctx, r.x, r.y, r.w, r.h, Math.max(8, item.strokeWidth * 4 * scaleX));
    return;
  }

  ctx.save();
  ctx.strokeStyle = item.color;
  ctx.fillStyle = item.color;
  ctx.lineWidth = item.strokeWidth;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';

  switch (item.tool) {
    case 'line': {
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
      break;
    }
    case 'arrow': {
      const dx = x2 - x1;
      const dy = y2 - y1;
      const len = Math.hypot(dx, dy);
      if (len < 2) break;
      const angle = Math.atan2(dy, dx);
      const headLen = Math.max(11, item.strokeWidth * 4.2);

      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2 - Math.cos(angle) * (headLen * 0.65), y2 - Math.sin(angle) * (headLen * 0.65));
      ctx.stroke();

      ctx.beginPath();
      ctx.moveTo(x2, y2);
      ctx.lineTo(
        x2 - headLen * Math.cos(angle - Math.PI / 6),
        y2 - headLen * Math.sin(angle - Math.PI / 6)
      );
      ctx.lineTo(
        x2 - headLen * Math.cos(angle + Math.PI / 6),
        y2 - headLen * Math.sin(angle + Math.PI / 6)
      );
      ctx.closePath();
      ctx.fill();
      break;
    }
    case 'rect': {
      const r = normalizeRect(x1, y1, x2, y2);
      ctx.strokeRect(r.x, r.y, r.w, r.h);
      break;
    }
    case 'ellipse': {
      const r = normalizeRect(x1, y1, x2, y2);
      ctx.beginPath();
      ctx.ellipse(r.x + r.w / 2, r.y + r.h / 2, r.w / 2, r.h / 2, 0, 0, Math.PI * 2);
      ctx.stroke();
      break;
    }
    case 'highlighter': {
      const pts = item.points ?? [];
      if (pts.length < 2) break;
      ctx.globalAlpha = 0.34;
      ctx.lineWidth = Math.max(14, item.strokeWidth * 5);
      ctx.beginPath();
      ctx.moveTo(pts[0].x - offsetX, pts[0].y - offsetY);
      for (let i = 1; i < pts.length; i++) {
        ctx.lineTo(pts[i].x - offsetX, pts[i].y - offsetY);
      }
      ctx.stroke();
      break;
    }
    case 'text': {
      if (!item.text) break;
      const fontSize = Math.max(14, item.strokeWidth * 5 + 6);
      ctx.font = `600 ${fontSize}px -apple-system, BlinkMacSystemFont, "Segoe UI Variable", sans-serif`;
      ctx.textBaseline = 'top';
      // Subtle contrast backing for readability over arbitrary desktop content
      ctx.shadowColor = item.color === '#FFFFFF' ? 'rgba(0,0,0,0.65)' : 'rgba(255,255,255,0.45)';
      ctx.shadowBlur = 2;
      ctx.fillText(item.text, x1, y1);
      break;
    }
    case 'step': {
      const num = item.stepNumber ?? 1;
      const rad = 13;
      ctx.beginPath();
      ctx.arc(x1, y1, rad, 0, Math.PI * 2);
      ctx.fill();
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = item.color === '#FFFFFF' ? '#18181B' : '#FFFFFF';
      ctx.stroke();

      ctx.fillStyle = item.color === '#FFFFFF' ? '#18181B' : '#FFFFFF';
      ctx.font = '600 13px -apple-system, BlinkMacSystemFont, "Segoe UI Variable", sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String(num), x1, y1 + 0.5);
      break;
    }
  }

  ctx.restore();
}

export function exportSelectionToPng(
  bgImage: HTMLImageElement | HTMLCanvasElement,
  selection: SelectionBox,
  annotations: AnnotationItem[],
  scaleX = 1,
  scaleY = 1
): string {
  const sx = Math.max(0, Math.round(selection.x * scaleX));
  const sy = Math.max(0, Math.round(selection.y * scaleY));
  const sw = Math.max(1, Math.round(selection.w * scaleX));
  const sh = Math.max(1, Math.round(selection.h * scaleY));

  const out = document.createElement('canvas');
  out.width = sw;
  out.height = sh;
  const ctx = out.getContext('2d')!;
  ctx.imageSmoothingEnabled = false;

  // Copy exact 1:1 native physical desktop pixels (lossless, zero blur/pixelation)
  ctx.drawImage(bgImage, sx, sy, sw, sh, 0, 0, sw, sh);

  ctx.save();
  ctx.setTransform(scaleX, 0, 0, scaleY, 0, 0);
  for (const item of annotations) {
    drawAnnotation(ctx, item, selection.x, selection.y, scaleX, scaleY);
  }
  ctx.restore();

  return out.toDataURL('image/png');
}
