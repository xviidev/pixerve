// Read-only smoke check for a deployed public image. No credentials/config writes.
const input=process.argv[2];
if(!input)throw new Error('Usage: node scripts/check-cdn.mjs FULL_PUBLIC_IMAGE_URL');
const target=new URL(input);
if(!['https:','http:'].includes(target.protocol))throw new Error('HTTP(S) URL required');
const results=[];
for(let i=0;i<3;i++) {
  const start=performance.now();
  const response=await fetch(target,{headers:{Accept:'image/avif,image/webp,image/jpeg'},signal:AbortSignal.timeout(30000)});
  const headersMs=performance.now()-start;
  const bytes=(await response.arrayBuffer()).byteLength;
  const url=new URL(response.url);
  if(url.searchParams.has('sig'))url.searchParams.set('sig','REDACTED');
  if(url.searchParams.has('asig'))url.searchParams.set('asig','REDACTED');
  results.push({request:i+1,url:url.toString(),status:response.status,bytes,headersMs:Math.round(headersMs),totalMs:Math.round(performance.now()-start),
    contentType:response.headers.get('content-type'),cacheControl:response.headers.get('cache-control'),vary:response.headers.get('vary'),
    cloudflareCache:response.headers.get('cf-cache-status'),fastlyCache:response.headers.get('x-cache'),age:response.headers.get('age'),
    originCache:response.headers.get('x-img-cache'),setCookie:response.headers.has('set-cookie')});
}
console.log(JSON.stringify(results,null,2));
if(results.some(r=>r.status!==200))process.exitCode=1;
