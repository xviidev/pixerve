# Fastly VCL snippet: type recv. Scope this service to the image hostname.
# Keep the full URL/query cache key. Do not strip sig, exp, pv or transforms.
if (req.method != "GET" && req.method != "HEAD") {
  return(pass);
}
if (req.url.path == "/health" || req.url.path == "/metrics" || req.http.Authorization) {
  return(pass);
}
# RFC-aware negotiation including q=0. A normalized Accept yields at most
# three Vary: Accept variants. Fixed-format responses don't emit Vary.
set req.http.Accept = accept.media_lookup(
  "image/avif:image/webp:image/jpeg", "image/jpeg", "", req.http.Accept
);
# Configure Shielding on the backend nearest the origin in the Fastly UI.
# Optional ORIGIN_TOKEN: inject X-Img-Origin-Token via a protected request
# header configuration. Never place a real secret in this repository.
