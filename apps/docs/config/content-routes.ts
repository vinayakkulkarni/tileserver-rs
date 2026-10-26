import { readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { contentFileToRoute } from './seo';

export const CONTENT_DIR = fileURLToPath(
  new URL('../content', import.meta.url),
);

/** Markdown files under content/, relative to it, sorted. */
export function contentFiles(dir: string = CONTENT_DIR): string[] {
  return readdirSync(dir, { recursive: true, encoding: 'utf8' })
    .filter((file) => file.endsWith('.md'))
    .map((file) => file.replace(/\\/g, '/'))
    .sort();
}

/** Route of every markdown page under content/. */
export function contentRoutes(dir: string = CONTENT_DIR): string[] {
  return contentFiles(dir).map(contentFileToRoute);
}
