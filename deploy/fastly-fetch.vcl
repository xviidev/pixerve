# Fastly VCL snippet: type fetch. Do not override TTL with Surrogate-Control.
# The origin supplies s-maxage, max-age, stale-while-revalidate and stale-if-error.
if (beresp.http.Cache-Control ~ "(?i)(private|no-store|no-cache)" || beresp.http.Set-Cookie) {
  return(pass);
}
if ((beresp.status >= 300 && beresp.status != 304) || beresp.status < 200) {
  return(pass);
}
