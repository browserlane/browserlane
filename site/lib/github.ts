export const GITHUB_REPO_URL = 'https://github.com/browserlane/browserlane';

/**
 * Star count for browserlane/browserlane, revalidated hourly (ISR).
 * Returns null on any failure (offline builds, rate limits) — callers
 * render the GitHub link without a count.
 */
export async function getGitHubStars(): Promise<number | null> {
  try {
    const res = await fetch(
      'https://api.github.com/repos/browserlane/browserlane',
      {
        headers: { Accept: 'application/vnd.github+json' },
        next: { revalidate: 3600 },
      },
    );
    if (!res.ok) return null;
    const data = (await res.json()) as { stargazers_count?: unknown };
    return typeof data.stargazers_count === 'number'
      ? data.stargazers_count
      : null;
  } catch {
    return null;
  }
}

export function formatStars(count: number): string {
  if (count >= 1000) {
    const k = count / 1000;
    return `${k >= 10 ? Math.round(k) : Math.round(k * 10) / 10}k`;
  }
  return String(count);
}
