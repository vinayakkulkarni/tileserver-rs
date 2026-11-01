/**
 * Theme token → `rgba()` string for MapLibre paint properties.
 *
 * The theme defines colors as OKLch custom properties. MapLibre's color
 * parser does not read `oklch()`, so the browser converts the token: a 1×1
 * canvas is filled with the token and the pixel is read back in sRGB.
 */
export function tokenRgba(token: string, alpha = 1): string {
  if (!import.meta.client) return `rgba(0, 0, 0, ${alpha})`;
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(token)
    .trim();
  const canvas = document.createElement('canvas');
  canvas.width = 1;
  canvas.height = 1;
  const ctx = canvas.getContext('2d');
  if (!ctx || !value) return `rgba(0, 0, 0, ${alpha})`;
  ctx.fillStyle = value;
  ctx.fillRect(0, 0, 1, 1);
  const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
