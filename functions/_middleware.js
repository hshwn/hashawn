// Second lock behind Cloudflare Access. Access stops strangers at the edge;
// this refuses any request that arrives without a valid Access token anyway
// (hashawn.pages.dev, a deleted policy, a misrouted host).

const TEAM = 'https://teskga.cloudflareaccess.com';
const AUD = 'e3b3b98aeccf3d1fa85d7216bb0d4b9f8ea5f7cb2eafdfe1f7d4f440b5bf46a7';

let jwks = null;
let jwksAt = 0;

async function keys() {
  if (!jwks || Date.now() - jwksAt > 3600e3) {
    const res = await fetch(`${TEAM}/cdn-cgi/access/certs`);
    jwks = (await res.json()).keys;
    jwksAt = Date.now();
  }
  return jwks;
}

const b64url = (s) => Uint8Array.from(atob(s.replace(/-/g, '+').replace(/_/g, '/')), (c) => c.charCodeAt(0));
const json = (s) => JSON.parse(new TextDecoder().decode(b64url(s)));

async function valid(token) {
  const [h, p, sig] = token.split('.');
  if (!sig) return false;
  const head = json(h);
  const claims = json(p);
  const jwk = (await keys()).find((k) => k.kid === head.kid);
  if (!jwk || head.alg !== 'RS256') return false;
  const key = await crypto.subtle.importKey('jwk', jwk, { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }, false, ['verify']);
  const ok = await crypto.subtle.verify('RSASSA-PKCS1-v1_5', key, b64url(sig), new TextEncoder().encode(`${h}.${p}`));
  const aud = [].concat(claims.aud);
  return ok && claims.iss === TEAM && aud.includes(AUD) && claims.exp * 1000 > Date.now();
}

function token(request) {
  const header = request.headers.get('Cf-Access-Jwt-Assertion');
  if (header) return header;
  const m = (request.headers.get('Cookie') || '').match(/(?:^|;\s*)CF_Authorization=([^;]+)/);
  return m ? m[1] : null;
}

export async function onRequest({ request, next }) {
  const url = new URL(request.url);
  const t = token(request);
  if (t && (await valid(t).catch(() => false))) return next();
  if (url.hostname.endsWith('.pages.dev')) {
    return Response.redirect(`https://hashawn.com${url.pathname}${url.search}`, 302);
  }
  return new Response('locked', { status: 403, headers: { 'Cache-Control': 'no-store' } });
}
