'use client';

import { cn } from '@/lib/util';
import { cva } from 'class-variance-authority';

export interface CardProps {
  children: React.ReactNode;
  label?: string;
  size?: 'sm' | 'md' | 'lg';
  variant?: 'bordered' | 'elevated';
  className?: string;
  tabIndex?: number;
  onClick?: () => void;
}

export interface ActionCardProps extends CardProps {
  cardClassName?: string;
  href?: string;
  next?: string;
  dom?: string;
}

const cardVariants = cva('bg-default', {
  variants: {
    variant: {
      bordered: 'border border-neutral-400',
      elevated: 'shadow-lg',
    },
    size: {
      sm: 'rounded-xl',
      md: 'rounded-2xl',
      lg: 'rounded-3xl',
    },
  },
  defaultVariants: {
    size: 'sm',
    variant: 'elevated',
  },
});

/**
 * A card that can be used to display content in a visually appealing way.
 * It can be used for various purposes like displaying information, images, or actions.
 * The `size` prop controls the border radius of the card.
 * - `sm` for small (default)
 * - `md` for medium
 * - `lg` for large
 */
export function Card({ children, label, size, variant, className, tabIndex = -1, onClick }: CardProps) {
  return (
    <section
      data-name="card"
      className={cn(cardVariants({ size, variant }), 'relative', className)}
      tabIndex={tabIndex}
      onClick={onClick}
    >
      {label && <div className="text-md bg-default absolute -top-5.5 left-4 px-3 pt-3 font-semibold">{label}</div>}
      {children}
    </section>
  );
}
/**
 * Content of the card, this gives the card its padding and spacing.
 * Use this to wrap the main content of the card.
 */
export function CardContent({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div data-name="card-content" className={cn('gap-2 p-3 md:p-6', className)}>
      {children}
    </div>
  );
}
