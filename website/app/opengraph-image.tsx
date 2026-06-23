import { ImageResponse } from 'next/og';

// Static OpenGraph / social-card image for the site root. Rendered at build
// time by next/og (Satori). 1200x630 is the canonical OG size.
export const alt =
  'browserlane — browser automation for humans and AI agents, via CLI and MCP, on WebDriver BiDi in Rust';
export const size = { width: 1200, height: 630 };
export const contentType = 'image/png';

export default function OpengraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: '100%',
          height: '100%',
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          background: '#0B0D12',
          color: '#F5F7FA',
          padding: '72px',
          fontFamily: 'monospace',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '24px' }}>
          <div
            style={{
              width: '96px',
              height: '96px',
              borderRadius: '20px',
              border: '3px solid #2B6CF6',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: '52px',
              fontWeight: 700,
              letterSpacing: '-2px',
            }}
          >
            bl
          </div>
          <div style={{ fontSize: '52px', fontWeight: 700 }}>browserlane</div>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
          <div style={{ fontSize: '62px', fontWeight: 700, lineHeight: 1.1 }}>
            Browser automation for humans and AI agents
          </div>
          <div style={{ fontSize: '34px', color: '#9AA4B2', lineHeight: 1.3 }}>
            One Rust binary — a CLI and an MCP server — driving Chrome over
            WebDriver BiDi.
          </div>
        </div>
        <div style={{ fontSize: '30px', color: '#2B6CF6' }}>
          docs.browserlane.com
        </div>
      </div>
    ),
    size,
  );
}
