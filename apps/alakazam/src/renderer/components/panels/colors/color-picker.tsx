import { ActiveColors } from '@/components/tools/active-colors';
import { cursor, mouseDown$, mouseMove$, mouseUp$ } from '@/events/body';
import { toPngCursor } from '@/lib/cursor';
import { activeColorState, backgroundColor, foregroundColor, hue, hue$, setTheActiveColor } from '@/state/color';
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { filter, fromEvent, map, merge } from 'rxjs';

function HueSelector({ pickerSize }: { pickerSize: [number, number] }) {
  const [hueImageData, setHueImageData] = useState<ImageData | null>(null);
  const [hueSize, setHueSize] = useState<[number, number]>([20, 256]);
  const [isPickerMouseDown, setIsPickerMouseDown] = useState(false);
  const [cursorPosition, setCursorPosition] = useState<number>(0);

  const hueCanvasRef = useRef<HTMLCanvasElement>(null);
  const arrowCursorRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    let [hueWidth, hueHeight] = hueSize;
    if (hueCanvasRef.current) {
      const { width, height } = hueCanvasRef.current.getBoundingClientRect();
      hueWidth = Math.floor(width);
      hueHeight = Math.floor(height);
      setHueSize([hueWidth, hueHeight]);
    }
  }, []);

  useEffect(() => {
    // Fetch hue gradient image.
    window.gizmos.colorPicker
      .gradientHue(hueSize[0], hueSize[1])
      .then(imageData =>
        setHueImageData(new ImageData(new Uint8ClampedArray(imageData.data), imageData.width, imageData.height)),
      );
    window.gizmos.cursor.arrowCursor(12, 'right', '#ffffff').then(imageData => {
      if (arrowCursorRef.current) {
        const cursorStyle = arrowCursorRef.current.style;
        cursorStyle.width = `${imageData.width}px`;
        cursorStyle.height = `${imageData.height}px`;
        cursorStyle.left = `-${imageData.width}px`;
        cursorStyle.backgroundImage = `url(${toPngCursor(imageData)})`;
        cursorStyle.backgroundRepeat = 'no-repeat';
        cursorStyle.pointerEvents = 'none';
      }
    });
    // Get initial cursor position.
    // window.gizmos.colorPicker.getHueAt(e.x / hueSize[0], e.y / hueSize[1]).then(color => {
    //   setCursorPosition(color.offset * hueSize[1]);
    //   setHue(color.offset * 360);
    // });
  }, []);

  useEffect(() => {
    if (arrowCursorRef.current) {
      const cursorStyle = arrowCursorRef.current.style;
      cursorStyle.top = `${cursorPosition - arrowCursorRef.current.offsetHeight / 2}px`;
    }
  }, [cursorPosition]);

  useEffect(() => {
    // Update arrow position when hue changes (e.g., when switching active colors)
    const hueSub = hue.subscribe(hueValue => {
      window.gizmos.colorPicker.getHueFrom(hueValue).then(hueInfo => {
        setCursorPosition(hueSize[1] - hueInfo.offset * hueSize[1]);
      });
    });

    return () => {
      hueSub.unsubscribe();
    };
  }, [hueSize]);

  useEffect(() => {
    const mouseDownSubscription = fromEvent<MouseEvent>(hueCanvasRef.current!, 'mousedown').subscribe(() =>
      setIsPickerMouseDown(true),
    );
    const mouseUpSubscription = mouseUp$.subscribe(() => setIsPickerMouseDown(false));

    const huePickerSubscription = merge(mouseDown$, mouseMove$)
      .pipe(
        filter(e => isPickerMouseDown || e.type === 'mousedown'),
        map(e => ({ rect: hueCanvasRef.current!.getBoundingClientRect(), event: e })),
        // Stop if outside picker area
        // - if the mousedown event is fired outside the picker, ignore it
        // - if the mousemove event is fired outside the picker don't ignore it, but clamp the position to the picker area
        filter(({ event, rect }) =>
          event.type === 'mousedown'
            ? event.clientX >= rect.left &&
              event.clientX <= rect.right &&
              event.clientY >= rect.top &&
              event.clientY <= rect.bottom
            : true,
        ),
        // Calculate x,y relative to picker, and clamp to picker area
        map(({ event, rect }) => {
          const x = Math.min(Math.max(event.clientX - rect.left, 0), rect.width);
          const y = Math.min(Math.max(event.clientY - rect.top, 0), rect.height);
          return { y, x, height: rect.height };
        }),
      )
      .subscribe(({ y, x, height }) => {
        window.gizmos.colorPicker.getHueAt(height - y, height).then(async data => {
          setCursorPosition(hueSize[1] - data.offset * hueSize[1]);
          hue.next(data.hex);

          // Get current active color and preserve its saturation/value, but update hue
          const active = activeColorState.getValue();
          const currentColor = active === 'fg' ? foregroundColor.getValue() : backgroundColor.getValue();
          const [x, y] = await window.gizmos.colorPicker.getCursorPosition(currentColor, pickerSize[0], pickerSize[1]);

          // Recalculate color with new hue but same saturation/value position
          const newColor = await window.gizmos.colorPicker.getColorAt(data.hex, x / pickerSize[0], y / pickerSize[1]);
          setTheActiveColor(newColor);
          active === 'fg' ? foregroundColor.next(newColor) : backgroundColor.next(newColor);
        });
      });

    return () => {
      huePickerSubscription.unsubscribe();
      mouseDownSubscription.unsubscribe();
      mouseUpSubscription.unsubscribe();
    };
  }, [isPickerMouseDown, hueSize, pickerSize]);

  return (
    <div className="relative flex h-full w-6">
      <div ref={arrowCursorRef} className="absolute z-10" />
      <canvas
        className="block h-full flex-none"
        width={hueSize[0]}
        height={hueSize[1]}
        style={{ height: '100%', width: 'auto' }}
        ref={canvas => {
          if (canvas) hueCanvasRef.current = canvas;
          if (canvas && hueImageData) {
            const ctx = canvas.getContext('2d');
            if (ctx) {
              ctx.putImageData(hueImageData, 0, 0);
            }
          }
        }}
      />
    </div>
  );
}

export function ColorPicker() {
  const [imageData, setImageData] = useState<ImageData | null>(null);
  const [pickerCursor, setPickerCursor] = useState<string>('');
  const [selectCursorPosition, setSelectCursorPosition] = useState<[number, number]>([0, 0]);
  const [pickerSize, setPickerSize] = useState<[number, number]>([256, 256]);
  const [isPickerMouseDown, setIsPickerMouseDown] = useState(false);
  const [selectorImageSize, setSelectorImageSize] = useState<number>(0);

  const CURSOR_SIZE = 16;
  const SELECTOR_SIZE = 14;

  const pickerCanvasRef = useRef<HTMLCanvasElement>(null);
  const pickerWrapperRef = useRef<HTMLDivElement>(null);
  const selectedCursorRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    if (pickerWrapperRef.current) {
      const { width, height } = pickerWrapperRef.current.getBoundingClientRect();
      const pickerWidth = Math.floor(width);
      const pickerHeight = Math.floor(height);
      setPickerSize([pickerWidth, pickerHeight]);
    }
  }, []);

  useEffect(() => {
    const [pickerWidth, pickerHeight] = pickerSize;

    // Subscribe to color changes and update picker accordingly
    const colorsSub = activeColorState.subscribe(() => {
      const active = activeColorState.getValue();
      const color = active === 'fg' ? foregroundColor.getValue() : backgroundColor.getValue();

      // Update hue based on current active color
      window.gizmos.colorPicker.getHueFrom(color).then(hueInfo => {
        hue.next(hueInfo.hex);
      });

      // Update cursor position
      window.gizmos.colorPicker.getCursorPosition(color, pickerWidth, pickerHeight).then(p => {
        setSelectCursorPosition(p);
      });
    });

    return () => {
      colorsSub.unsubscribe();
    };
  }, [pickerSize]);

  useEffect(() => {
    const [pickerWidth, pickerHeight] = pickerSize;

    // Fetch color picker image based on current hue
    const hueSub = hue$.subscribe(hue => {
      window.gizmos.colorPicker
        .colorPicker(hue.hex, pickerWidth, pickerHeight)
        .then(imageData =>
          setImageData(new ImageData(new Uint8ClampedArray(imageData.data), imageData.width, imageData.height)),
        );
    });
    // Fetch the mouse cursor.
    window.gizmos.cursor.circleCursor(CURSOR_SIZE).then(imageData => setPickerCursor(toPngCursor(imageData)));
    // Fetch the selected color cursor.
    window.gizmos.cursor.circleCursor(SELECTOR_SIZE).then(imageData => {
      setSelectorImageSize(imageData.width);
      if (selectedCursorRef.current) {
        selectedCursorRef.current.style.backgroundImage = `url(${toPngCursor(imageData)})`;
        selectedCursorRef.current.style.backgroundRepeat = 'no-repeat';
        selectedCursorRef.current.style.pointerEvents = 'none';
      }
    });
    return () => {
      hueSub.unsubscribe();
    };
  }, [pickerSize]);

  useEffect(() => {
    if (selectedCursorRef.current && pickerWrapperRef.current) {
      const { left, top } = pickerWrapperRef.current.getBoundingClientRect();
      const [x, y] = selectCursorPosition;
      selectedCursorRef.current.style.left = `${x - selectorImageSize / 2 + left}px`;
      selectedCursorRef.current.style.top = `${y - selectorImageSize / 2 + top}px`;
      selectedCursorRef.current.style.width = `${selectorImageSize}px`;
      selectedCursorRef.current.style.height = `${selectorImageSize}px`;
    }
  }, [selectCursorPosition, selectorImageSize]);

  useEffect(() => {
    const mouseEnterSubscription = fromEvent(pickerCanvasRef.current!, 'mouseenter').subscribe(() => {
      cursor.next({
        cursor: pickerCursor,
        origin: [CURSOR_SIZE / 2, CURSOR_SIZE / 2],
        size: CURSOR_SIZE,
      });
    });

    const mouseLeaveSubscription = fromEvent(pickerCanvasRef.current!, 'mouseleave').subscribe(() => {
      cursor.next({
        cursor: 'default',
        origin: [0, 0],
        size: 0,
      });
    });

    const mouseDownSubscription = fromEvent<MouseEvent>(pickerCanvasRef.current!, 'mousedown').subscribe(() =>
      setIsPickerMouseDown(true),
    );
    const mouseUpSubscription = mouseUp$.subscribe(() => setIsPickerMouseDown(false));

    const pickColorSubscription = merge(mouseDown$, mouseMove$)
      .pipe(
        filter(e => isPickerMouseDown || e.type === 'mousedown'),
        map(e => ({ rect: pickerCanvasRef.current!.getBoundingClientRect(), event: e })),
        // Stop if outside picker area
        // - if the mousedown event is fired outside the picker, ignore it
        // - if the mousemove event is fired outside the picker don't ignore it, but clamp the position to the picker area
        filter(({ event, rect }) =>
          event.type === 'mousedown'
            ? event.clientX >= rect.left &&
              event.clientX <= rect.right &&
              event.clientY >= rect.top &&
              event.clientY <= rect.bottom
            : true,
        ),
        // Calculate x,y relative to picker, and clamp to picker area
        map(({ event, rect }) => {
          const x = Math.min(Math.max(event.clientX - rect.left, 0), rect.width);
          const y = Math.min(Math.max(event.clientY - rect.top, 0), rect.height);
          return { x, y };
        }),
      )
      .subscribe(async ({ x, y }) => {
        const hueValue = hue.getValue();
        window.gizmos.colorPicker.getColorAt(hueValue, x / pickerSize[0], y / pickerSize[1]).then(color => {
          setTheActiveColor(color);
          setSelectCursorPosition([x, y]);
        });
      });

    return () => {
      mouseEnterSubscription.unsubscribe();
      mouseLeaveSubscription.unsubscribe();
      pickColorSubscription.unsubscribe();
      mouseDownSubscription.unsubscribe();
      mouseUpSubscription.unsubscribe();
    };
  }, [pickerCursor, pickerSize, isPickerMouseDown]);

  return (
    <div className="flex h-full flex-col gap-2">
      <ActiveColors layout="horizontal" />
      <div className="flex h-full gap-4">
        <div ref={pickerWrapperRef} className="min-h-0 min-w-0 flex-1 overflow-hidden">
          <div ref={selectedCursorRef} className="pointer-events-none absolute z-10" />
          <canvas
            className="block h-full w-full"
            width={pickerSize[0]}
            height={pickerSize[1]}
            ref={canvas => {
              if (canvas) pickerCanvasRef.current = canvas;
              if (canvas && imageData) {
                const ctx = canvas.getContext('2d');
                if (ctx) {
                  ctx.putImageData(imageData, 0, 0);
                }
              }
            }}
          />
        </div>
        <HueSelector pickerSize={pickerSize} />
      </div>
    </div>
  );
}
