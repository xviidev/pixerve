// Test a portable Windows package without the developer SDK in its environment.
import assert from 'node:assert/strict';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';

const directory = path.resolve(process.argv[2] || '');
assert(process.argv[2], 'Usage: node scripts/check-package.mjs EXTRACTED_PACKAGE');
assert.equal(process.platform, 'win32', 'This checker validates the Windows package');
const manifest = JSON.parse(await readFile(path.join(directory, 'manifest.json'), 'utf8'));
for (const file of manifest.files) {
  const target = path.resolve(directory, file.path);
  assert(target.startsWith(directory + path.sep), 'Unsafe manifest path');
  const bytes = await readFile(target);
  assert.equal(bytes.length, file.bytes, file.path);
  assert.equal(createHash('sha256').update(bytes).digest('hex'), file.sha256, file.path);
}
const reserved = net.createServer(); reserved.listen(0, '127.0.0.1');
await once(reserved, 'listening');
const port = reserved.address().port;
await new Promise(resolve => reserved.close(resolve));
const cache = await mkdtemp(path.join(os.tmpdir(), 'pixerve-package-'));
const windows = process.env.SystemRoot || 'C:\\Windows';
const env = { ...process.env, PATH: `${windows}\\System32;${windows}`, PORT: String(port),
  SOURCE_DIR: path.join(directory, 'static'), STATIC_DIR: path.join(directory, 'static'),
  OG_WATERMARK: path.join(directory, 'assets/og_wm.png'), DISK_CACHE_DIR: cache,
  SIGNING_KEY: '', CACHE_VISIBILITY: 'public', ORIGIN_TOKEN: '', METRICS_TOKEN: '',
  OPEN_RPS: '100', OPEN_WINDOW: '60', OPEN_ALLOWED_FORMATS: 'avif,webp,jpeg,png',
  PIPELINE_VERSION: '1', PRESETS_ONLY: 'false',
  CANONICAL_REDIRECTS: 'true', RUST_LOG: 'pixerve=warn' };
for (const key of Object.keys(env)) {
  if (['path','vips_lib_dir','vips_home','gi_typelib_path','pkg_config_path','ld_library_path'].includes(key.toLowerCase())) delete env[key];
}
env.PATH = `${windows}\\System32;${windows}`;
let logs = '';
const child = spawn(path.join(directory, 'pixerve.exe'), [], { cwd: directory, env,
  windowsHide: true, stdio: ['ignore','pipe','pipe'] });
child.stdout.on('data', bytes => { logs += bytes; });
child.stderr.on('data', bytes => { logs += bytes; });
let spawnError;
child.on('error', error => { spawnError = error; });
const base = `http://127.0.0.1:${port}`;
try {
  let ready = false;
  for (let i = 0; i < 100; i++) {
    if (spawnError) throw spawnError;
    if (child.exitCode !== null) throw new Error(`Package exited: ${child.exitCode}; ${logs}`);
    try { ready = (await fetch(base+'/health', {signal:AbortSignal.timeout(1000)})).ok; } catch {}
    if (ready) break;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  assert(ready, 'Package did not start: '+logs);
  for (const format of ['jpeg','png','webp','avif']) {
    const response = await fetch(`${base}/example.png?w=320&f=${format}`, {signal:AbortSignal.timeout(30000)});
    assert.equal(response.status, 200, `${format}: ${response.ok ? '' : await response.clone().text()}`);
    assert.equal(response.headers.get('content-type'), `image/${format}`);
    const bytes = Buffer.from(await response.arrayBuffer()); assert(bytes.length > 50);
    if (format==='png') { assert.equal(bytes.readUInt32BE(16),320); assert.equal(bytes.readUInt32BE(20),180); }
    const conditional = await fetch(response.url, {headers:{'If-None-Match':response.headers.get('etag')}});
    assert.equal(conditional.status,304);
  }
  assert.equal((await fetch(base+'/og/example.png')).status,200);
  assert.equal((await fetch(base+'/static/example.png')).status,200);
  console.log(`PASS: ${manifest.files.length} bundled file hashes; isolated startup, JPEG/PNG/WebP/AVIF, conditional requests, OG, static.`);
} finally {
  if (child.exitCode === null && !spawnError) { const exited=once(child,'exit'); child.kill(); await exited; }
  await rm(cache, {recursive:true,force:true});
}
