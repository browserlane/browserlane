/**
 * Newsletter signup → Resend Audience contact.
 *
 * Env (set in the Vercel project):
 *   RESEND_API_KEY      — Resend API key
 *   RESEND_AUDIENCE_ID  — the audience that collects subscribers
 *
 * Without them the endpoint degrades gracefully (503 + a human-readable
 * message), so the form is safe to ship before Resend is wired up.
 *
 * Abuse posture: same-origin check (blocks cross-site browser POSTs) +
 * honeypot. Script-driven list-bombing needs platform-level rate limiting
 * (Vercel WAF / firewall rules) and double opt-in in Resend — see
 * site/README.md.
 */

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export async function POST(request: Request) {
  // Browsers always send Origin on cross-site (and fetch-initiated) POSTs;
  // reject ones that don't come from this deployment's own pages.
  const origin = request.headers.get('origin');
  const host = request.headers.get('host');
  if (origin && host) {
    let originHost = '';
    try {
      originHost = new URL(origin).host;
    } catch {
      /* malformed origin → treated as mismatch below */
    }
    if (originHost !== host) {
      return Response.json({ error: 'Invalid request.' }, { status: 403 });
    }
  }

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return Response.json({ error: 'Invalid request.' }, { status: 400 });
  }
  // `null` is valid JSON, so json() can succeed and still not give an object.
  if (body === null || typeof body !== 'object') {
    return Response.json({ error: 'Invalid request.' }, { status: 400 });
  }
  const { email: rawEmail, company } = body as {
    email?: unknown;
    company?: unknown;
  };

  // Honeypot: real users never fill the hidden "company" field.
  if (typeof company === 'string' && company.length > 0) {
    return Response.json({ ok: true });
  }

  const email = typeof rawEmail === 'string' ? rawEmail.trim() : '';
  // Length first: the regex backtracks quadratically on adversarial input,
  // so never let an oversized string reach it.
  if (email.length === 0 || email.length > 254 || !EMAIL_RE.test(email)) {
    return Response.json(
      { error: 'Enter a valid email address.' },
      { status: 400 },
    );
  }

  const apiKey = process.env.RESEND_API_KEY;
  const audienceId = process.env.RESEND_AUDIENCE_ID;
  if (!apiKey || !audienceId) {
    return Response.json(
      { error: 'Signups aren’t open yet — email bl@browserlane.com instead.' },
      { status: 503 },
    );
  }

  try {
    const res = await fetch(
      `https://api.resend.com/audiences/${audienceId}/contacts`,
      {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${apiKey}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ email, unsubscribed: false }),
      },
    );

    // 409 = already subscribed; that's a success from the user's side.
    if (!res.ok && res.status !== 409) {
      return Response.json(
        { error: 'Something went wrong — try again in a minute.' },
        { status: 502 },
      );
    }
    return Response.json({ ok: true });
  } catch {
    return Response.json(
      { error: 'Something went wrong — try again in a minute.' },
      { status: 502 },
    );
  }
}
