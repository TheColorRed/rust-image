'use client';

import { useOverlayPosition } from '@/hooks/overlay-position';
import { cn } from '@/lib/util';
import { useCallback, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';

export interface TooltipProps {
  children: React.ReactNode;
  content: React.ReactNode;
  className?: string;
  position?: 'above' | 'below' | 'left' | 'right';
  delay?: number;
  disabled?: boolean;
}

export function Tooltip({
  children,
  content,
  position: positionProp = 'above',
  delay = 500,
  disabled = false,
  className,
}: TooltipProps) {
  const [show, setShow] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const ref = useRef<HTMLDivElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const calcPosition = useOverlayPosition();

  const calculateTooltipPosition = useCallback(() => {
    if (!ref.current || !tooltipRef.current) return;

    // Map tooltip positions to overlay hook placement/adjustment
    let placement: 'above' | 'below' | 'center' =
      positionProp === 'above' ? 'above' : positionProp === 'below' ? 'below' : 'center';
    let adjustment: 'left' | 'center' | 'right' | 'start' | 'end' = 'center';
    if (positionProp === 'left') adjustment = 'left';
    if (positionProp === 'right') adjustment = 'right';

    const pos = calcPosition(ref, tooltipRef, placement, adjustment);
    // offset between trigger and tooltip
    const offset = 8;
    const top = positionProp === 'above' ? pos.top - offset : positionProp === 'below' ? pos.top + offset : pos.top;
    const left = positionProp === 'left' ? pos.left - offset : positionProp === 'right' ? pos.left + offset : pos.left;
    setPosition({ top, left });
  }, [calcPosition, positionProp]);

  const handleMouseEnter = () => {
    if (disabled) return;
    timeoutRef.current = setTimeout(() => setShow(true), delay);
  };
  const handleMouseLeave = () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setShow(false);
  };

  useLayoutEffect(() => {
    if (disabled) setShow(false);
  }, [disabled]);

  useLayoutEffect(() => {
    if (!show) return;

    calculateTooltipPosition();

    // Recalculate on resize/scroll while tooltip is shown
    window.addEventListener('resize', calculateTooltipPosition);
    window.addEventListener('scroll', calculateTooltipPosition, true);
    return () => {
      window.removeEventListener('resize', calculateTooltipPosition);
      window.removeEventListener('scroll', calculateTooltipPosition, true);
    };
  }, [show, calculateTooltipPosition]);

  return (
    <div ref={ref} onMouseEnter={handleMouseEnter} onMouseLeave={handleMouseLeave} className="inline-block">
      {children}
      {show &&
        createPortal(
          <div
            ref={tooltipRef}
            style={{ position: 'absolute', top: position.top, left: position.left }}
            className={cn('pointer-events-none rounded-lg bg-black p-2 text-sm text-white', className)}
          >
            {content}
          </div>,
          document.body,
        )}
    </div>
  );
}
