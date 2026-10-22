import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { resolve, relative, sep, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const output = fileURLToPath(new URL('../apps/docs/dist/', import.meta.url));
const base = '/' + (process.env.DOCS_BASE ?? '').split('/').filter(Boolean).join('/');
const prefix = base === '/' ? '/' : base + '/';
const origin = 'https://docs.test';
const failures = new Set<string>();
const ids = new Map<string, Set<string>>();

function pages(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const file = resolve(directory, entry.name);
    return entry.isDirectory() ? pages(file) : entry.name.endsWith('.html') ? [file] : [];
  });
}

const files = pages(output);
for (const file of files) {
  const html = readFileSync(file, 'utf8');
  const route = relative(output, file).split(sep).join('/').replace(/index\.html$/, '');
  for (const match of html.matchAll(/\bhref=["']([^"']+)["']/g)) {
    const href = match[1]!.replaceAll('&amp;', '&');
    const url = new URL(href, origin + prefix + route);
    if (url.origin !== origin) continue;
    if (!url.pathname.startsWith(prefix)) {
      failures.add(`${route}: link escapes DOCS_BASE: ${href}`);
      continue;
    }
    const pathname = decodeURIComponent(url.pathname.slice(prefix.length));
    let target = resolve(output, pathname || 'index.html');
    if (!extname(target)) target = resolve(target, 'index.html');
    if (!target.startsWith(output) || !existsSync(target)) {
      failures.add(`${route}: missing target: ${href}`);
      continue;
    }
    if (url.hash && target.endsWith('.html')) {
      if (!ids.has(target)) {
        ids.set(target, new Set([...readFileSync(target, 'utf8').matchAll(/\bid=["']([^"']+)["']/g)].map(m => m[1]!)));
      }
      if (!ids.get(target)!.has(decodeURIComponent(url.hash.slice(1)))) {
        failures.add(`${route}: missing anchor: ${href}`);
      }
    }
  }
}
if (failures.size) throw new Error([...failures].join('\n'));
console.log(`PASS: links and anchors across ${files.length} documentation pages.`);
