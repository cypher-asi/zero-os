import { useRef, useState, useEffect, CSSProperties } from 'react';

/**
 * Hook for animating container height when content changes.
 *
 * Uses a two-container pattern:
 * - Outer container: animated height + overflow:hidden + transition
 * - Inner container: natural sizing, measured by ResizeObserver
 *
 * @example
 * ```tsx
 * const { containerStyle, contentRef } = useAnimatedHeight();
 *
 * return (
 *   <div style={containerStyle}>      // Animated container
 *     <div ref={contentRef}>          // Measured content
 *       {children}
 *     </div>
 *   </div>
 * );
 * ```
 */
export function useAnimatedHeight(duration = 250) {
  const contentRef = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState<number | 'auto'>('auto');

  useEffect(() => {
    if (!contentRef.current) return;

    const updateHeight = () => {
      if (contentRef.current) {
        setHeight(contentRef.current.scrollHeight);
      }
    };

    // Initial measurement
    updateHeight();

    // Observe size changes
    const observer = new ResizeObserver(updateHeight);
    observer.observe(contentRef.current);

    return () => observer.disconnect();
  }, []);

  const containerStyle: CSSProperties = {
    height,
    overflow: 'hidden',
    transition: `height ${duration}ms cubic-bezier(0.4, 0, 0.2, 1)`,
  };

  return { containerStyle, contentRef };
}
