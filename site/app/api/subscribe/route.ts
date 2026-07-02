import { createConfirmToken } from '@/lib/confirm-token';

/**
 * Newsletter signup → Resend Audience contact, with DOUBLE OPT-IN
 * (Resend has no native toggle; this is their documented pattern):
 *
 *   1. POST here creates the contact with `unsubscribed: true`
 *   2. We email a signed 48h confirmation link (from RESEND_FROM)
 *   3. GET /api/confirm flips the contact to `unsubscribed: false`
 *
 * Bombed/typo'd addresses get one confirmation email and are never
 * actually subscribed.
 *
 * Env (set in the Vercel project):
 *   RESEND_API_KEY      — Resend API key (also signs confirm tokens)
 *   RESEND_AUDIENCE_ID  — the audience that collects subscribers
 *   RESEND_FROM         — optional sender override
 *                         (default: "browserlane <bl@browserlane.com>";
 *                         the domain must be verified in Resend)
 *
 * Without the required env the endpoint degrades gracefully (503 + a
 * human-readable message), so the form is safe to ship before Resend is
 * wired up.
 *
 * Abuse posture: same-origin check (blocks cross-site browser POSTs) +
 * honeypot + double opt-in. Script-driven spam needs platform-level rate
 * limiting (Vercel WAF rule on this path) — see site/README.md.
 */

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const DEFAULT_FROM = 'browserlane <bl@browserlane.com>';

function confirmEmailHtml(confirmUrl: string): string {
  return `<!doctype html>
<html>
  <body style="margin:0;padding:32px 16px;background:#fafaf7;font-family:ui-sans-serif,system-ui,-apple-system,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#191919;">
    <div style="max-width:480px;margin:0 auto;">
      <table role="presentation" cellpadding="0" cellspacing="0" border="0">
        <tr>
          <td style="width:22px;height:22px;border-radius:5px;background:#cc785c;text-align:center;vertical-align:middle;">
            <span style="color:#ffffff;font-size:12px;font-weight:700;line-height:1;">bl</span>
          </td>
          <td style="padding-left:9px;font-weight:600;font-size:18px;letter-spacing:-0.01em;vertical-align:middle;color:#191919;">browserlane</td>
        </tr>
      </table>
      <p style="margin:20px 0 8px;font-size:15px;line-height:1.6;">
        Confirm your subscription to browserlane release updates — new
        commands, MCP tools, and release notes. No noise.
      </p>
      <p style="margin:24px 0;">
        <a href="${confirmUrl}" style="display:inline-block;background:#cc785c;color:#191919;text-decoration:none;font-weight:500;padding:10px 18px;border-radius:8px;">Confirm subscription</a>
      </p>
      <p style="margin:16px 0 0;font-size:13px;color:#666663;line-height:1.6;">
        If you didn’t request this, ignore this email — you won’t be
        subscribed. The link expires in 48 hours.
      </p>
    </div>
  </body>
</html>`;
}

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

  const resendHeaders = {
    Authorization: `Bearer ${apiKey}`,
    'Content-Type': 'application/json',
  };

  try {
    // 1. Create as unsubscribed (pending). 409 = already exists — fine,
    //    we still (re)send the confirmation link.
    const createRes = await fetch(
      `https://api.resend.com/audiences/${audienceId}/contacts`,
      {
        method: 'POST',
        headers: resendHeaders,
        body: JSON.stringify({ email, unsubscribed: true }),
      },
    );
    if (!createRes.ok && createRes.status !== 409) {
      return Response.json(
        { error: 'Something went wrong — try again in a minute.' },
        { status: 502 },
      );
    }

    // 2. Send the signed confirmation link.
    const token = createConfirmToken(email, apiKey);
    const base = host
      ? `${request.headers.get('x-forwarded-proto') ?? 'https'}://${host}`
      : 'https://browserlane.com';
    const confirmUrl = `${base}/api/confirm?token=${encodeURIComponent(token)}`;

    const sendRes = await fetch('https://api.resend.com/emails', {
      method: 'POST',
      headers: resendHeaders,
      body: JSON.stringify({
        from: process.env.RESEND_FROM ?? DEFAULT_FROM,
        to: [email],
        subject: 'Confirm your browserlane subscription',
        html: confirmEmailHtml(confirmUrl),
        text: `Confirm your subscription to browserlane release updates:\n\n${confirmUrl}\n\nIf you didn’t request this, ignore this email — you won’t be subscribed. The link expires in 48 hours.`,
      }),
    });
    if (!sendRes.ok) {
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
