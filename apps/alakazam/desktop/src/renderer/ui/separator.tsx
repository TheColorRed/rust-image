'use client';

import { cn } from '@/lib/util';

export interface SeparatorProps {
  /**
   * The direction of the separator.
   * - `horizontal`: A horizontal line (default).
   * - `vertical`: A vertical line.
   */
  direction?: 'horizontal' | 'vertical';
  /** Additional CSS classes to apply to the separator */
  className?: string;
}
/**
 * A simple separator component that can be used to visually separate content.
 * It can be rendered as either a horizontal or vertical line.
 * For vertical separators, it defaults to matching the height of its parent
 * (uses `h-full w-px self-stretch`), so make sure the parent has a defined
 * height (or is a flex container). Override with `className` if needed.
 * @param {SeparatorProps} props - The properties for the separator.
 * @param {string} [props.direction='horizontal'] - The direction of the separator. Can be 'horizontal' or 'vertical'.
 * @param {string} [props.className] - Additional CSS classes to apply to the separator.
 */
export function Separator({ direction = 'horizontal', className }: SeparatorProps) {
  if (direction === 'vertical') {
    // Stretch to the parent's height (align-self: stretch) without requiring an explicit parent height.
    return <div data-name="separator" className={cn('w-px self-stretch bg-white/50', className)} />;
  }

  return <hr data-name="separator" className={cn('w-full border-t border-white/50', className)} />;
}
