// Renames the inlined build to something a person would recognise on a
// desktop, and reports what it costs. Run through `npm run build:standalone`.
import { readFileSync, renameSync, statSync } from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const dir = path.join(root, 'standalone');
const from = path.join(dir, 'index.html');
const to = path.join(dir, 'void-harvest.html');

renameSync(from, to);
const html = readFileSync(to, 'utf8');
const external = [...html.matchAll(/<(?:script|link)[^>]*(?:src|href)="(?!data:)([^"]+)"/g)].map((m) => m[1]);
if (external.length > 0) {
  console.error(`not self-contained, still points at: ${external.join(', ')}`);
  process.exit(1);
}
console.log(`wrote standalone/void-harvest.html (${Math.round(statSync(to).size / 1024)} KB, nothing external)`);
