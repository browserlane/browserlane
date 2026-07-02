import type { AnchorHTMLAttributes, ReactNode } from 'react';

type Variant = 'primary' | 'ghost';

const styles: Record<Variant, string> = {
  primary:
    'bg-clay text-ink hover:bg-kraft border border-clay hover:border-kraft',
  ghost:
    'border border-edge text-ivory-light hover:border-cloud hover:text-white',
};

export function CTAButton({
  variant = 'primary',
  size = 'md',
  children,
  className = '',
  ...props
}: {
  variant?: Variant;
  size?: 'sm' | 'md';
  children: ReactNode;
} & AnchorHTMLAttributes<HTMLAnchorElement>) {
  const sizing =
    size === 'sm' ? 'h-9 px-4 text-sm' : 'h-11 px-5 text-[15px]';
  return (
    <a
      className={`inline-flex items-center justify-center gap-2 rounded-lg font-medium tracking-tight transition-colors ${sizing} ${styles[variant]} ${className}`}
      {...props}
    >
      {children}
    </a>
  );
}
