import { useEffect, useRef, useState, useCallback } from 'react';

export function useFocusTrap(enabled: boolean = true) {
  const containerRef = useRef<HTMLDivElement>(null);
  const previousActiveElement = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!enabled || !containerRef.current) return;

    previousActiveElement.current = document.activeElement as HTMLElement;
    const container = containerRef.current;

    const focusableElements = container.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const firstElement = focusableElements[0];
    const lastElement = focusableElements[focusableElements.length - 1];

    firstElement?.focus();

    const handleTab = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;

      if (e.shiftKey) {
        if (document.activeElement === firstElement) {
          e.preventDefault();
          lastElement?.focus();
        }
      } else {
        if (document.activeElement === lastElement) {
          e.preventDefault();
          firstElement?.focus();
        }
      }
    };

    container.addEventListener('keydown', handleTab);
    return () => {
      container.removeEventListener('keydown', handleTab);
      previousActiveElement.current?.focus();
    };
  }, [enabled]);

  return containerRef;
}

export function useKeyboardNavigation(
  itemsCount: number,
  onSelect: (index: number) => void,
  options: {
    loop?: boolean;
    orientation?: 'vertical' | 'horizontal';
    onEscape?: () => void;
  } = {}
) {
  const { loop = true, orientation = 'vertical', onEscape } = options;
  const [focusedIndex, setFocusedIndex] = useState(0);

  // Latest-Ref pattern: all values read inside the callback via refs,
  // deps stay [] so the handler identity is stable.
  const onSelectRef = useRef(onSelect);
  const focusedIndexRef = useRef(focusedIndex);
  const itemsCountRef = useRef(itemsCount);
  const loopRef = useRef(loop);
  const orientationRef = useRef(orientation);
  const onEscapeRef = useRef(onEscape);

  useEffect(() => { onSelectRef.current = onSelect; }, [onSelect]);
  useEffect(() => { focusedIndexRef.current = focusedIndex; }, [focusedIndex]);
  useEffect(() => { itemsCountRef.current = itemsCount; }, [itemsCount]);
  useEffect(() => { loopRef.current = loop; }, [loop]);
  useEffect(() => { orientationRef.current = orientation; }, [orientation]);
  useEffect(() => { onEscapeRef.current = onEscape; }, [onEscape]);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    const isVertical = orientationRef.current === 'vertical';
    const nextKey = isVertical ? 'ArrowDown' : 'ArrowRight';
    const prevKey = isVertical ? 'ArrowUp' : 'ArrowLeft';

    switch (e.key) {
      case nextKey:
        e.preventDefault();
        setFocusedIndex((prev) =>
          prev < itemsCountRef.current - 1 ? prev + 1 : loopRef.current ? 0 : prev
        );
        break;
      case prevKey:
        e.preventDefault();
        setFocusedIndex((prev) =>
          prev > 0 ? prev - 1 : loopRef.current ? itemsCountRef.current - 1 : prev
        );
        break;
      case 'Home':
        e.preventDefault();
        setFocusedIndex(0);
        break;
      case 'End':
        e.preventDefault();
        setFocusedIndex(itemsCountRef.current - 1);
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        onSelectRef.current(focusedIndexRef.current);
        break;
      case 'Escape':
        onEscapeRef.current?.();
        break;
    }
  }, []);

  return { focusedIndex, setFocusedIndex, onKeyDown: handleKeyDown };
}

export function useAnnouncer() {
  const [_message, setMessage] = useState('');
  const liveRegionRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const div = document.createElement('div');
    div.setAttribute('role', 'status');
    div.setAttribute('aria-live', 'polite');
    div.setAttribute('aria-atomic', 'true');
    div.className = 'sr-only';
    document.body.appendChild(div);
    liveRegionRef.current = div;
    return () => {
      div.remove();
      liveRegionRef.current = null;
    };
  }, []);

  const announce = useCallback((text: string) => {
    setMessage('');
    setTimeout(() => {
      setMessage(text)
      if (liveRegionRef.current) liveRegionRef.current.textContent = text
    }, 0);
  }, []);

  return { announce, liveRegionRef };
}