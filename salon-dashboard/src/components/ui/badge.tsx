import type { HTMLAttributes } from 'react'

type Variant = 'default' | 'secondary' | 'accent' | 'destructive' | 'outline'

const styles: Record<Variant, string> = {
  default: 'bg-primary text-primary-foreground',
  secondary: 'bg-secondary/15 text-secondary',
  accent: 'bg-accent/15 text-accent',
  destructive: 'bg-destructive/15 text-destructive',
  outline: 'border border-border text-muted-foreground',
}

export function Badge({
  variant = 'default',
  className = '',
  ...props
}: HTMLAttributes<HTMLSpanElement> & { variant?: Variant }) {
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-semibold ${styles[variant]} ${className}`}
      {...props}
    />
  )
}
