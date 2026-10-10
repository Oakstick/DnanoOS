// Zips the production build into release/<name>-<version>.zip, ready to upload
// to itch.io, Netlify Drop, or any static host. Run `npm run package`.
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, rmSync, statSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const pkg = createRequire(import.meta.url)(path.join(root, 'package.json'));
const dist = path.join(root, 'dist');
const outDir = path.join(root, 'release');
const out = path.join(outDir, `${pkg.name}-${pkg.version}.zip`);

if (!existsSync(path.join(dist, 'index.html'))) {
  console.error('dist/index.html not found: run `npm run build` first');
  process.exit(1);
}
mkdirSync(outDir, { recursive: true });
rmSync(out, { force: true });

if (process.platform === 'win32') {
  execFileSync('powershell', [
    '-NoProfile', '-Command',
    `Compress-Archive -Path '${dist}\*' -DestinationPath '${out}' -CompressionLevel Optimal`,
  ], { stdio: 'inherit' });
} else {
  execFileSync('zip', ['-r', '-q', out, '.'], { cwd: dist, stdio: 'inherit' });
}
const kb = Math.round(statSync(out).size / 1024);
console.log(`wrote ${path.relative(root, out)} (${kb} KB)`);
