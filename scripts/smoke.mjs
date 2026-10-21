import assert from 'node:assert/strict';
import { createHmac, createHash } from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import net from 'node:net';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const target = path.join(root, 'target');
await mkdir(target, { recursive: true });
const fixture = await mkdtemp(path.join(target, 'smoke-'));
const input = path.join(fixture, 'source');
await mkdir(input);
const staticInput=path.join(fixture,'static');
await mkdir(staticInput);
await writeFile(path.join(staticInput,'logo.png'),await readFile(path.resolve(root,'static/example.png')));
const secret = 'smoke-test-secret';
const executable = process.env.PIXERVE_BIN || path.join(target, 'release', process.platform === 'win32' ? 'pixerve.exe' : 'pixerve');
const keys = ['w', 'h', 'q', 'bl', 'bg', 'f', 'fit', 'fo'];
let server;
let origin;
let logs = '';
let checks = 0;
let s3Mock;
const pass = () => { checks++; };

async function start(extra = {}) {
  const portServer = net.createServer();
  portServer.listen(0, '127.0.0.1');
  await once(portServer, 'listening');
  const port = portServer.address().port;
  await new Promise(resolve => portServer.close(resolve));
  origin = `http://127.0.0.1:${port}`;
  logs = '';
  server = spawn(executable, [], {
    cwd: fixture, windowsHide: true,
    env: {
      ...process.env, PORT: String(port), SOURCE_DIR: input,
      STATIC_DIR: staticInput, OG_WATERMARK: path.resolve(root, 'assets/og_wm.png'),
      DISK_CACHE_DIR: path.join(fixture, 'cache'), DISK_CACHE_MB: '16', MEM_CACHE_MB: '2',
      CACHE_TTL: '300', CACHE_IDLE_SECS: '300', MAX_CONCURRENT: '1', MAX_QUEUE: '4', MAX_REQUESTS: '64',
      MAX_SOURCE_MB: '1', MAX_INPUT_PIXELS: '40000000', MP_LIMIT: '12', VIPS_CONCURRENCY: '1',
      SIGNING_KEY: secret, RUST_LOG: 'pixerve=warn',
      PIPELINE_VERSION:'1', CACHE_VISIBILITY:'public', CANONICAL_REDIRECTS:'true', PRESETS_ONLY:'false',
      ORIGIN_TOKEN:'',
      METRICS_TOKEN:'smoke-metrics-token-secret', BROWSER_CACHE_TTL:'300', CDN_CACHE_TTL:'86400',
      IMMUTABLE_CACHE_TTL:'31536000', STALE_WHILE_REVALIDATE:'60', STALE_IF_ERROR:'86400', OPEN_MAX_W:'1280',
      ...extra,
    }, stdio: ['ignore', 'pipe', 'pipe'],
  });
  server.stdout.on('data', b => { logs += b; });
  server.stderr.on('data', b => { logs += b; });
  let launchError;
  server.on('error', e => { launchError = e; });
  for (let i = 0; i < 150; i++) {
    if (launchError) throw launchError;
    if (server.exitCode !== null) throw new Error(`Server exited ${server.exitCode}: ${logs}`);
    try { if ((await fetch(`${origin}/health`)).ok) return; } catch {}
    await new Promise(r => setTimeout(r, 100));
  }
  throw new Error(`Startup timeout: ${logs}`);
}

async function stop() {
  if (!server || server.exitCode !== null) return;
  const exited = once(server, 'exit');
  server.kill();
  await exited;
}

function signed(file, params, token = false) {
  const expiry = Math.floor(Date.now() / 1000) + 60;
  const norm = keys.filter(k => params[k] !== undefined).map(k => `${k}=${params[k]}`).join(',');
  const signingFile=file.replace(/^(og|static)\//,'');
  const sig = createHmac('sha256', secret).update(`${signingFile}|${norm}|${expiry}`).digest('hex');
  const encoded=file.split('/').map(encodeURIComponent).join('/');
  if (token) return `/tr:${keys.filter(k => params[k] !== undefined).map(k => `${k}-${params[k]}`).join(',')},s-${sig},e-${expiry}/${encoded}`;
  return `/${encoded}?${new URLSearchParams({ ...params, sig, exp: String(expiry) })}`;
}

async function image(url, expected, headers = {}) {
  const response = await fetch(origin + url, { headers });
  assert.equal(response.status, 200, `${url}: ${await (response.ok ? Promise.resolve('') : response.text())}; ${logs}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  const metadata = await sharp(bytes).metadata();
  for (const [key, value] of Object.entries(expected)) assert.equal(metadata[key], value, `${url}: ${key}`);
  assert.match(response.headers.get('etag'), /^W\/"[a-f0-9]+"$/);
  pass();
  return { response, bytes, metadata };
}

try {
  await sharp({ create: { width: 1600, height: 1000, channels: 3, background: '#b05020' } }).jpeg().toFile(path.join(input, 'photo.jpg'));
  await sharp({ create: { width: 600, height: 400, channels: 4, background: { r: 255, g: 0, b: 0, alpha: 0.5 } } }).png().toFile(path.join(input, 'alpha.png'));
  await sharp({ create: { width: 600, height: 400, channels: 3, background: '#112233' } }).withMetadata({ orientation: 6 }).jpeg().toFile(path.join(input, 'rotated.jpg'));
  await writeFile(path.join(input, 'large.bin'), Buffer.alloc(1024 * 1024 + 1));
  await writeFile(path.join(input, 'invalid.jpg'), 'invalid image');
  await start();
  assert.deepEqual(await (await fetch(origin + '/health')).json(), { ok: true }); pass();
  assert.equal((await fetch(origin + '/static/logo.png')).status, 200); pass();
  const raw = await fetch(origin + '/photo.jpg');
  assert.equal(raw.headers.get('content-type'), 'image/jpeg');
  assert.deepEqual(Buffer.from(await raw.arrayBuffer()), await readFile(path.join(input, 'photo.jpg'))); pass();

  for (const f of ['jpeg', 'png', 'webp', 'avif']) {
    const encoded = await image(signed('photo.jpg', { w: 320, h: 180, f, q: 70 }), { width: 320, height: 180, format: f === 'avif' ? 'heif' : f });
    if (f === 'avif') {
      assert.equal(encoded.bytes.toString('ascii', 4, 8), 'ftyp');
      assert.equal(encoded.bytes.toString('ascii', 8, 12), 'avif', 'AVIF must contain AV1, not HEVC'); pass();
      await writeFile(path.join(input, 'roundtrip.avif'), encoded.bytes);
    }
  }
  assert.doesNotMatch(logs, /ignoring (trellis_quant|overshoot_deringing|optimize_scans)/); pass();
  await image(signed('roundtrip.avif', { w: 160, f: 'png' }), { width: 160, height: 90, format: 'png' });
  for (const [fit, width, height] of [['cover', 300, 300], ['contain', 300, 300], ['fill', 300, 300], ['inside', 300, 188], ['outside', 480, 300]]) {
    await image(signed('photo.jpg', { w: 300, h: 300, f: 'png', fit }), { width, height });
  }
  for (const fit of ['cover', 'contain', 'fill', 'inside', 'outside']) {
    for (const [w, h] of [[900, 900], [900, 300], [300, 900]]) {
      const { info } = await sharp(path.join(input, 'alpha.png')).rotate().resize(w, h, { fit, withoutEnlargement: true }).png().toBuffer({ resolveWithObject: true });
      await image(signed('alpha.png', { w, h, fit, f: 'png' }), { width: info.width, height: info.height });
    }
  }
  await image(signed('alpha.png', { w: 300, fit: 'fill', f: 'png' }), { width: 300, height: 200 });
  await image(signed('alpha.png', { h: 200, fit: 'fill', f: 'png' }), { width: 300, height: 200 });
  for (const fo of ['center', 'top', 'left', 'bottom', 'right', 'top_left', 'top_right', 'bottom_left', 'bottom_right', 'entropy', 'attention']) {
    await image(signed('photo.jpg', { w: 320, h: 180, f: 'jpeg', fo }, true), { width: 320, height: 180 });
  }
  await image(signed('rotated.jpg', { w: 200, f: 'jpeg' }), { width: 200, height: 300 });
  await image(signed('alpha.png', { w: 300, h: 300, fit: 'contain', f: 'png' }), { width: 300, height: 300, hasAlpha: true });
  const flattened = await image(signed('alpha.png', { w: 100, bg: '0f0', f: 'jpeg' }), { width: 100, hasAlpha: false });
  const pixel = await sharp(flattened.bytes).raw().toBuffer();
  assert.ok(pixel[0] > 100 && pixel[1] > 100 && pixel[2] < 20, 'JPEG must flatten alpha against green background'); pass();
  await image('/og/photo.jpg', { width: 1200, height: 630, format: 'jpeg' });
  await image('/og/alpha.png', { width: 1200, height: 630, format: 'jpeg' });
  for (const [accept, format] of [['image/webp', 'webp'], ['image/avif', 'heif'], ['image/avif;q=0,image/webp', 'webp']]) {
    await image(signed('photo.jpg', { w: 180 }), { format }, { Accept: accept });
  }
  const cachedUrl = signed('photo.jpg', { w: 333, f: 'webp' });
  const cached = await image(cachedUrl, { width: 333 });
  const conditional = await fetch(origin + cachedUrl, { headers: { 'If-None-Match': cached.response.headers.get('etag') } });
  assert.equal(conditional.status, 304); assert.equal((await conditional.arrayBuffer()).byteLength, 0); pass();
  const head = await fetch(origin + cachedUrl, { method: 'HEAD' });
  assert.equal(head.status, 200); assert.equal((await head.arrayBuffer()).byteLength, 0); pass();
  assert.equal((await fetch(origin + '/photo.jpg?w=300')).status, 403); pass();
  assert.equal((await fetch(origin + signed('photo.jpg', { w: 100 }).replace('sig=', 'sig=00'))).status, 403); pass();
  assert.equal((await fetch(origin + '/photo.jpg?w=0')).status, 400); pass();
  assert.equal((await fetch(origin + '/photo.jpg?bl=NaN')).status, 400); pass();
  assert.equal((await fetch(origin + signed('invalid.jpg', { w: 100 }))).status, 422); pass();
  assert.equal((await fetch(origin + '/large.bin')).status, 413); pass();
  assert.equal((await fetch(origin + '/missing.jpg')).status, 404); pass();

  // Add a non-uniform source so blur must produce a different cached result.
  await sharp({ create: { width: 400, height: 300, channels: 3, background: 'white' } })
    .composite([{ input: Buffer.from('<svg width="200" height="300"><rect width="200" height="300" fill="black"/></svg>'), left: 0, top: 0 }])
    .png().toFile(path.join(input, 'edges.png'));
  const clear = await image(signed('edges.png', { w: 200, f: 'png' }), { width: 200 });
  const blurred = await image(signed('edges.png', { w: 200, f: 'png', bl: 5 }), { width: 200 });
  assert.notDeepEqual(clear.bytes, blurred.bytes); pass();
  // CDN correctness: headers, version integrity, canonical URLs and presets.
  const fixed=await image(signed('alpha.png',{w:240,f:'webp'}),{width:240,format:'webp'});
  assert.equal(fixed.response.headers.get('vary'),null);
  assert.ok(!fixed.response.headers.get('cache-control').includes('immutable'));
  assert.match(fixed.response.headers.get('cache-control'),/s-maxage=/);
  assert.match(fixed.response.headers.get('cache-control'),/stale-if-error=/); pass();
  const normalized=await image(signed('alpha.png',{w:240,q:95,f:'webp',fit:'cover',fo:'center'}),{width:240});
  assert.equal(normalized.response.headers.get('x-img-cache'),'RAM'); pass();
  const auto=await image(signed('alpha.png',{w:240}),{format:'webp'},{Accept:'image/webp'});
  assert.equal(auto.response.headers.get('vary'),'Accept'); pass();
  const cliEnv={...process.env,SIGNING_KEY:secret,PIPELINE_VERSION:'1',CACHE_VISIBILITY:'public',METRICS_TOKEN:'',ORIGIN_TOKEN:''};
  const version=JSON.parse(execFileSync(executable,['version',path.join(input,'photo.jpg'),'news'],{env:cliEnv,encoding:'utf8',windowsHide:true,cwd:fixture}));
  const sourceBytes=await readFile(path.join(input,'photo.jpg'));
  assert.equal(version.sha256,createHash('sha256').update(sourceBytes).digest('hex')); pass();
  await mkdir(path.dirname(path.join(input,version.key)),{recursive:true});
  await writeFile(path.join(input,version.key),sourceBytes);
  const versionAgain=JSON.parse(execFileSync(executable,['version',path.join(input,version.key),'news'],{env:cliEnv,encoding:'utf8',windowsHide:true,cwd:fixture}));
  assert.equal(versionAgain.key,version.key);pass();
  const immutableUrl=signed(version.key,{w:320,f:'webp'})+'&pv=1';
  const immutable=await image(immutableUrl,{width:320,format:'webp'});
  assert.match(immutable.response.headers.get('cache-control'),/max-age=31536000.*immutable/);
  assert.equal(immutable.response.headers.get('vary'),null); pass();
  const notModified=await fetch(origin+immutableUrl,{headers:{'If-None-Match':immutable.response.headers.get('etag')}});
  assert.equal(notModified.status,304);assert.match(notModified.headers.get('cache-control'),/immutable/);
  assert.equal(notModified.headers.get('vary'),null);pass();
  const mutableHash=await image(signed(version.key,{w:320,f:'webp'}),{width:320});
  assert.ok(!mutableHash.response.headers.get('cache-control').includes('immutable'));pass();
  const autoHash=await image(signed(version.key,{w:320})+'&pv=1',{width:320},{Accept:'image/webp'});
  assert.equal(autoHash.response.headers.get('vary'),'Accept');
  assert.ok(!autoHash.response.headers.get('cache-control').includes('immutable'));pass();
  assert.equal((await fetch(origin+signed(version.key,{w:320,f:'webp'})+'&pv=old')).status,410);pass();
  const badName='bad-'+ '0'.repeat(64)+'.jpg';
  await writeFile(path.join(input,badName),sourceBytes);
  const badHash=await fetch(origin+'/'+badName+'?pv=1');
  assert.equal(badHash.status,409);assert.equal(badHash.headers.get('cache-control'),'no-store');pass();
  await writeFile(path.join(staticInput,'asset-'+version.sha256+'.jpg'),sourceBytes);
  const staticVersion=await fetch(origin+'/static/asset-'+version.sha256+'.jpg?pv=1');
  assert.equal(staticVersion.status,200);assert.match(staticVersion.headers.get('cache-control'),/immutable/);
  assert.equal(staticVersion.headers.get('vary'),null);await staticVersion.arrayBuffer();pass();
  for (const [p,w,h] of [['thumbnail',320,180],['card',640,360],['article',960,600],['wide',1280,800]]) {
    const resolved={w,q:p==='thumbnail'||p==='card'?75:80,f:'webp',fit:p==='thumbnail'||p==='card'?'cover':'inside'};
    if(p==='thumbnail'||p==='card')resolved.h=h;
    const url=new URL(signed('photo.jpg',resolved),origin);
    for(const key of keys)url.searchParams.delete(key);
    url.searchParams.set('p',p);
    await image(url.pathname+url.search,{width:w,height:h,format:'webp'});
  }
  const canonical=await fetch(origin+signed('photo.jpg',{w:320,f:'webp'})+'&utm_source=test',{redirect:'manual'});
  assert.equal(canonical.status,308);assert.equal(canonical.headers.get('cache-control'),'no-store');
  assert.ok(canonical.headers.get('location').startsWith('/tr:f-webp,w-320/'));
  assert.ok(!canonical.headers.get('location').includes('utm_source'));pass();
  assert.equal((await fetch(origin+'/alpha.png?w=200&w=300')).status,400);pass();
  assert.equal((await fetch(origin+'/tr:f-webp,w-200,f-auto/alpha.png')).status,400);pass();
  await writeFile(path.join(input,'foto % ü.png'),await readFile(path.join(input,'alpha.png')));
  await image(signed('foto % ü.png',{w:120,f:'webp'}),{width:120});
  const until=String(Math.floor(Date.now()/1000)+3600);
  const generated=execFileSync(executable,['sign-until',version.key,'p=card',until],{env:cliEnv,encoding:'utf8',windowsHide:true,cwd:fixture}).trim();
  const repeated=execFileSync(executable,['sign-until',version.key,'p=card',until],{env:cliEnv,encoding:'utf8',windowsHide:true,cwd:fixture}).trim();
  assert.equal(generated,repeated);pass();
  const generatedImage=await image(generated,{width:640,height:360,format:'webp'});
  assert.match(generatedImage.response.headers.get('cache-control'),/immutable/);pass();
  const unsigned=execFileSync(executable,['url',version.key,'p=article,f=avif'],{env:cliEnv,encoding:'utf8',windowsHide:true,cwd:fixture}).trim();
  assert.ok(unsigned.startsWith('/tr:f-avif,w-960,q-80,fit-inside/'));
  assert.ok(unsigned.endsWith('?pv=1'));pass();
  const assetUrl=execFileSync(executable,['asset',version.key,'p=card'],{env:cliEnv,encoding:'utf8',windowsHide:true,cwd:fixture}).trim();
  const assetAgain=execFileSync(executable,['asset',version.key,'p=card'],{env:cliEnv,encoding:'utf8',windowsHide:true,cwd:fixture}).trim();
  assert.equal(assetUrl,assetAgain);assert.ok(!new URL(assetUrl,origin).searchParams.has('exp'));pass();
  const stableAsset=await image(assetUrl,{width:640,height:360,format:'webp'});
  assert.match(stableAsset.response.headers.get('cache-control'),/immutable/);pass();
  const brokenAsset=new URL(assetUrl,origin);
  const assetSig=brokenAsset.searchParams.get('asig');
  brokenAsset.searchParams.set('asig',assetSig.slice(0,-1)+(assetSig.endsWith('0')?'1':'0'));
  assert.equal((await fetch(brokenAsset)).status,403);pass();
  const wrongPipeline=new URL(assetUrl,origin);wrongPipeline.searchParams.set('pv','2');
  assert.equal((await fetch(wrongPipeline)).status,403);pass();
  assert.equal((await fetch(origin+assetUrl.replace('f-webp','f-auto'))).status,403);pass();
  assert.equal((await fetch(origin+assetUrl.replace(version.key,'alpha.png'))).status,403);pass();
  const metricsHeaders={Authorization:'Bearer smoke-metrics-token-secret'};
  assert.equal((await fetch(origin+'/metrics')).status,404);pass();
  const metricsResponse=await fetch(origin+'/metrics',{headers:metricsHeaders});
  const stats=await metricsResponse.text();
  assert.equal(metricsResponse.status,200);assert.equal(metricsResponse.headers.get('cache-control'),'no-store');
  assert.match(stats,/img_ram_hits_total [1-9]/);assert.match(stats,/img_transforms_total [1-9]/);
  assert.match(stats,/img_request_duration_seconds_bucket\{le="\+Inf"\}/);
  assert.ok(!stats.includes('smoke-test-secret')&&!stats.includes('photo.jpg'));pass();
  const requests=stats.match(/^img_requests_total (\d+)$/m)[1];
  const again=await (await fetch(origin+'/metrics',{headers:metricsHeaders})).text();
  assert.equal(again.match(/^img_requests_total (\d+)$/m)[1],requests);pass();
  const coalesced = await Promise.all(Array.from({ length: 16 }, () => fetch(origin + signed('photo.jpg', { w: 555, f: 'webp' }))));
  assert.ok(coalesced.every(r => r.status === 200)); pass();
  await Promise.all(coalesced.map(r => r.arrayBuffer()));
  // A cache hit must survive removal of the source, including after restart.
  await rm(path.join(input, 'photo.jpg'));
  await image(cachedUrl, { width: 333 });
  await stop();
  await start();
  await image(cachedUrl, { width: 333 });
  const restartStats=await (await fetch(origin+'/metrics',{headers:metricsHeaders})).text();
  assert.match(restartStats,/img_disk_hits_total [1-9]/);pass();
  await stop();
  await start({CACHE_VISIBILITY:'private'});
  const privateAssetDenied=await fetch(origin+assetUrl);
  assert.equal(privateAssetDenied.status,403);assert.equal(privateAssetDenied.headers.get('cache-control'),'no-store');pass();
  for(const resource of ['/alpha.png','/og/alpha.png','/static/logo.png']) {
    const denied=await fetch(origin+resource);assert.equal(denied.status,403);
    assert.equal(denied.headers.get('cache-control'),'no-store');pass();
  }
  const privateRaw=await fetch(origin+signed('alpha.png',{}));
  assert.equal(privateRaw.status,200);assert.equal(privateRaw.headers.get('cache-control'),'private, no-store');
  await privateRaw.arrayBuffer();pass();
  const privateStatic=await fetch(origin+signed('static/logo.png',{}));
  assert.equal(privateStatic.status,200);assert.equal(privateStatic.headers.get('cache-control'),'private, no-store');
  await privateStatic.arrayBuffer();pass();
  const privateOg=await image(signed('og/alpha.png',{w:1200,h:630,q:70,f:'jpeg',fit:'cover',fo:'entropy'}),{width:1200,height:630});
  assert.equal(privateOg.response.headers.get('cache-control'),'private, no-store');pass();
  const privateVersion=await image(immutableUrl,{width:320});
  assert.equal(privateVersion.response.headers.get('cache-control'),'private, no-store');pass();
  assert.equal((await fetch(origin+'/alpha.png')).status,403);pass();
  await stop();
  await start({PRESETS_ONLY:'true'});
  assert.equal((await fetch(origin+signed('alpha.png',{w:111,f:'webp'}))).status,400);pass();
  await image(signed('alpha.png',{w:320,h:180,q:75,f:'webp',fit:'cover'}),{width:320,height:180});
  assert.equal((await fetch(origin+signed('alpha.png',{w:320,h:180,q:74,f:'webp',fit:'cover'}))).status,400);pass();
  await stop();
  await start({ORIGIN_TOKEN:'smoke-origin-secret',METRICS_TOKEN:''});
  assert.equal((await fetch(origin+'/alpha.png')).status,403);pass();
  const originAllowed=await fetch(origin+'/alpha.png',{headers:{'X-Img-Origin-Token':'smoke-origin-secret'}});
  assert.equal(originAllowed.status,200);assert.equal(originAllowed.headers.get('x-img-origin-token'),null);
  await originAllowed.arrayBuffer();pass();
  assert.equal((await fetch(origin+'/metrics',{headers:metricsHeaders})).status,404);pass();
  await stop();
  await start({ SIGNING_KEY: '', OPEN_RPS: '5', OPEN_WINDOW: '60' });
  assert.equal((await fetch(origin + '/alpha.png?w=1281')).status, 413); pass();
  assert.equal((await fetch(origin + '/alpha.png?w=100')).status, 200); pass();
  assert.equal((await fetch(origin + '/alpha.png?w=100')).status, 200); pass();
  assert.equal((await fetch(origin + '/alpha.png?w=100')).status, 200); pass();
  assert.equal((await fetch(origin + '/alpha.png?w=100')).status, 200); pass();
  assert.equal((await fetch(origin + '/alpha.png?w=100')).status, 429); pass();
  await stop();

  // Exercise actual S3 HTTP/signing and backpressure against a local mock.
  const s3Image = await sharp({ create: { width: 800, height: 500, channels: 3, background: 'blue' } }).jpeg().toBuffer();
  let s3Calls = 0, active = 0, maxActive = 0;
  s3Mock = http.createServer((req, res) => {
    s3Calls++;
    active++;
    maxActive = Math.max(maxActive, active);
    assert.match(req.headers.authorization || '', /^AWS4-HMAC-SHA256 /);
    assert.ok(req.url.startsWith('/smoke-bucket/'));
    if(req.url==='/smoke-bucket/forbidden.jpg') {
      active--;
      res.writeHead(403,{'Content-Type':'application/xml'});
      res.end('<Error><Code>AccessDenied</Code><Message>Mock denied</Message></Error>');
      return;
    }
    setTimeout(() => {
      active--;
      res.writeHead(200, { 'Content-Type': 'image/jpeg', 'Content-Length': s3Image.length, ETag: '"mock-source"', 'Last-Modified': new Date(0).toUTCString() });
      res.end(s3Image);
    }, 150);
  });
  s3Mock.listen(0, '127.0.0.1');
  await once(s3Mock, 'listening');
  await start({ SOURCE_DIR: '', S3_ENDPOINT: `http://127.0.0.1:${s3Mock.address().port}`, S3_BUCKET: 'smoke-bucket', S3_ACCESS_KEY_ID: 'test-key', S3_ACCESS_KEY_SECRET: 'test-secret', S3_REGION: 'auto' });
  const burst = await Promise.all(Array.from({ length: 16 }, () => fetch(origin + signed('coalesced.jpg', { w: 200, f: 'webp' }))));
  assert.ok(burst.every(r => r.status === 200));
  await Promise.all(burst.map(r => r.arrayBuffer()));
  assert.equal(s3Calls, 1, 'Identical requests must download once'); pass();
  await image(signed('coalesced.jpg', { w: 200, f: 'webp' }), { width: 200 });
  assert.equal(s3Calls, 1, 'RAM hit must not download'); pass();
  const overload = await Promise.all(Array.from({ length: 16 }, (_, i) => fetch(origin + `/slow-${i}.jpg`)));
  assert.ok(overload.some(r => r.status === 503), 'Bounded queue must reject excess work');
  assert.ok(overload.every(r => [200, 503].includes(r.status)));
  await Promise.all(overload.map(r => r.arrayBuffer()));
  assert.equal(maxActive, 1, 'Source downloads must obey MAX_CONCURRENT'); pass();
  const backendError=await fetch(origin+'/forbidden.jpg');
  assert.equal(backendError.status,502);assert.equal(backendError.headers.get('cache-control'),'no-store');pass();
  console.log(`PASS: ${checks} HTTP/image checks (codecs, fits, focus, EXIF, alpha, OG, signing, cache, restart, limits).`);
} catch (e) {
  console.error(logs);
  throw e;
} finally {
  await stop();
  if (s3Mock) await new Promise(resolve => s3Mock.close(resolve));
  if (fixture.startsWith(target + path.sep) && path.basename(fixture).startsWith('smoke-')) {
    await rm(fixture, { recursive: true, force: true });
  }
}
