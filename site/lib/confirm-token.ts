import { createHmac, timingSafeEqual } from 'node:crypto';

/**
 * Signed, expiring tokens for the newsletter double opt-in confirmation
 * link: `b64url(email).expiresAtSeconds.hmac`. The HMAC key is the Resend
 * API key — already secret, server-only, and requires no extra env var
 * (rotating it merely invalidates in-flight 48h confirmation links).
 */

const TTL_SECONDS = 48 * 60 * 60;

function sign(payload: string, secret: string): string {
  return createHmac('sha256', secret).update(payload).digest('base64url');
}

export function createConfirmToken(
  email: string,
  secret: string,
  nowMs = Date.now(),
): string {
  const emailPart = Buffer.from(email, 'utf8').toString('base64url');
  const exp = Math.floor(nowMs / 1000) + TTL_SECONDS;
  const payload = `${emailPart}.${exp}`;
  return `${payload}.${sign(payload, secret)}`;
}

export type ConfirmTokenResult =
  | { ok: true; email: string }
  | { ok: false; reason: 'malformed' | 'bad-signature' | 'expired' };

export function verifyConfirmToken(
  token: string,
  secret: string,
  nowMs = Date.now(),
): ConfirmTokenResult {
  const parts = token.split('.');
  if (parts.length !== 3) return { ok: false, reason: 'malformed' };
  const [emailPart, expPart, sig] = parts;

  const exp = Number(expPart);
  if (!Number.isInteger(exp) || exp <= 0) {
    return { ok: false, reason: 'malformed' };
  }

  const expected = sign(`${emailPart}.${expPart}`, secret);
  const a = Buffer.from(sig);
  const b = Buffer.from(expected);
  if (a.length !== b.length || !timingSafeEqual(a, b)) {
    return { ok: false, reason: 'bad-signature' };
  }

  if (exp * 1000 < nowMs) return { ok: false, reason: 'expired' };

  const email = Buffer.from(emailPart, 'base64url').toString('utf8');
  if (!email) return { ok: false, reason: 'malformed' };
  return { ok: true, email };
}
