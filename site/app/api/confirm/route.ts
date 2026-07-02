import { verifyConfirmToken } from '@/lib/confirm-token';

/**
 * Double opt-in confirmation: the link emailed by /api/subscribe lands
 * here; a valid token flips the Resend contact to `unsubscribed: false`.
 * Responds with a tiny self-contained HTML page either way.
 */

function page(title: string, body: string, status: number): Response {
  const html = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="robots" content="noindex" />
    <title>${title} — browserlane</title>
  </head>
  <body style="margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;padding:24px;background:#fafaf7;font-family:ui-sans-serif,system-ui,-apple-system,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#191919;">
    <div style="max-width:440px;text-align:center;">
      <div style="display:flex;align-items:center;justify-content:center;gap:9px;margin-bottom:16px;">
        <span style="display:block;width:22px;height:22px;border-radius:5px;background:#cc785c;color:#ffffff;text-align:center;line-height:22px;font-size:12px;font-weight:700;">bl</span>
        <span style="font-weight:600;font-size:18px;letter-spacing:-0.01em;">browserlane</span>
      </div>
      <h1 style="margin:0 0 10px;font-size:22px;letter-spacing:-0.02em;">${title}</h1>
      <p style="margin:0 0 24px;font-size:15px;line-height:1.6;color:#666663;">${body}</p>
      <a href="/" style="display:inline-block;background:#cc785c;color:#191919;text-decoration:none;font-weight:500;padding:10px 18px;border-radius:8px;">Back to browserlane.com</a>
    </div>
  </body>
</html>`;
  return new Response(html, {
    status,
    headers: { 'Content-Type': 'text/html; charset=utf-8' },
  });
}

export async function GET(request: Request) {
  const apiKey = process.env.RESEND_API_KEY;
  const audienceId = process.env.RESEND_AUDIENCE_ID;
  if (!apiKey || !audienceId) {
    return page(
      'Signups aren’t open yet',
      'This confirmation link isn’t active on this deployment. Email bl@browserlane.com instead.',
      503,
    );
  }

  const token = new URL(request.url).searchParams.get('token') ?? '';
  const result = verifyConfirmToken(token, apiKey);
  if (!result.ok) {
    return page(
      result.reason === 'expired'
        ? 'This link has expired'
        : 'This link isn’t valid',
      'Confirmation links expire after 48 hours. Head back to the site and subscribe again to get a fresh one.',
      400,
    );
  }

  try {
    const res = await fetch(
      `https://api.resend.com/audiences/${audienceId}/contacts/${encodeURIComponent(result.email)}`,
      {
        method: 'PATCH',
        headers: {
          Authorization: `Bearer ${apiKey}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ unsubscribed: false }),
      },
    );
    if (!res.ok) {
      return page(
        'Something went wrong',
        'We couldn’t confirm your subscription just now. Try the link again in a minute, or subscribe again from the site.',
        502,
      );
    }
  } catch {
    return page(
      'Something went wrong',
      'We couldn’t confirm your subscription just now. Try the link again in a minute, or subscribe again from the site.',
      502,
    );
  }

  return page(
    'You’re subscribed',
    'You’ll get browserlane release updates — new commands, MCP tools, and release notes. No noise, unsubscribe anytime.',
    200,
  );
}
