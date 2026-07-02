export function SectionHeading({
  eyebrow,
  title,
  sub,
  align = 'left',
}: {
  eyebrow: string;
  title: string;
  sub?: string;
  align?: 'left' | 'center';
}) {
  const alignment = align === 'center' ? 'text-center mx-auto' : '';
  return (
    <div className={`max-w-3xl ${alignment}`}>
      <p className="font-mono text-xs uppercase tracking-[0.22em] text-clay">
        {eyebrow}
      </p>
      <h2 className="mt-4 text-3xl font-semibold tracking-tight text-ivory-light md:text-[2.75rem] md:leading-[1.1]">
        {title}
      </h2>
      {sub ? (
        <p
          className={`mt-5 max-w-2xl text-base leading-relaxed text-cloud-light md:text-lg ${
            align === 'center' ? 'mx-auto' : ''
          }`}
        >
          {sub}
        </p>
      ) : null}
    </div>
  );
}
