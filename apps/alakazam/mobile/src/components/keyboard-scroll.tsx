import { createContext, forwardRef, useCallback, useContext, useEffect, useRef, useState, type ComponentRef, type ReactNode } from 'react';
import { Keyboard, ScrollView, TextInput, type ScrollViewProps, type TextInputProps } from 'react-native';

/** Asks the scroll view around an input to scroll until the bottom of the input, as a y in the window, is above the keyboard. */
type Reveal = (bottom: number) => void;
const RevealContext = createContext<Reveal | null>(null);

/** The room kept between a revealed input and the keyboard, in dp. */
const REVEAL_MARGIN = 24;
/** How long a focused input waits for the keyboard to finish opening before it gives up asking for room, in milliseconds. */
const KEYBOARD_WAIT_MS = 1500;

/**
 * A scroll view whose `RevealInput`s are brought into view when the keyboard opens over them. Android resizes the screen for the keyboard
 * only for some windows (a Modal with a translucent status bar is not resized, and a panel inside a screen keeps the height it had), so the
 * input being typed in could be under the keyboard with nothing showing what is typed. While the keyboard is open this adds as much room
 * under the content as the keyboard covers, so even the last input can be scrolled above it.
 */
export function KeyboardScrollView({ children, contentContainerStyle, onScroll, ...rest }: ScrollViewProps & { children?: ReactNode }) {
  const scroll = useRef<ComponentRef<typeof ScrollView>>(null);
  // Where the content is scrolled to, kept in a ref and not in state so scrolling does not draw this again.
  const offset = useRef(0);
  const [covered, setCovered] = useState(0);

  useEffect(() => {
    const show = Keyboard.addListener('keyboardDidShow', event => setCovered(event.endCoordinates.height));
    const hide = Keyboard.addListener('keyboardDidHide', () => setCovered(0));
    return () => {
      show.remove();
      hide.remove();
    };
  }, []);

  const reveal = useCallback<Reveal>(bottom => {
    const top = Keyboard.metrics()?.screenY;
    if (top === undefined) return;
    const hidden = bottom + REVEAL_MARGIN - top;
    if (hidden > 0) scroll.current?.scrollTo({ y: offset.current + hidden, animated: true });
  }, []);

  return (
    <RevealContext.Provider value={reveal}>
      <ScrollView
        ref={scroll}
        keyboardShouldPersistTaps="handled"
        scrollEventThrottle={16}
        {...rest}
        onScroll={event => {
          offset.current = event.nativeEvent.contentOffset.y;
          onScroll?.(event);
        }}
        contentContainerStyle={[contentContainerStyle, covered > 0 && { paddingBottom: covered }]}
      >
        {children}
      </ScrollView>
    </RevealContext.Provider>
  );
}

/**
 * A text input that, when it is focused inside a `KeyboardScrollView`, scrolls itself above the keyboard once the keyboard is open. Outside
 * one it is a plain `TextInput`.
 */
export const RevealInput = forwardRef<ComponentRef<typeof TextInput>, TextInputProps>(function RevealInput({ onFocus, ...rest }, forwarded) {
  const reveal = useContext(RevealContext);
  const own = useRef<ComponentRef<typeof TextInput> | null>(null);
  const waiting = useRef<{ stop: () => void } | null>(null);
  useEffect(() => () => waiting.current?.stop(), []);

  return (
    <TextInput
      {...rest}
      ref={node => {
        own.current = node;
        if (typeof forwarded === 'function') forwarded(node);
        else if (forwarded) forwarded.current = node;
      }}
      onFocus={event => {
        onFocus?.(event);
        if (!reveal) return;
        const place = () => own.current?.measureInWindow((_x, y, _width, height) => reveal(y + height));
        waiting.current?.stop();
        // The keyboard may already be open (moving from one input to the next), in which case there is nothing to wait for.
        if (Keyboard.isVisible()) {
          place();
          return;
        }
        const timer = setTimeout(() => waiting.current?.stop(), KEYBOARD_WAIT_MS);
        const shown = Keyboard.addListener('keyboardDidShow', () => {
          waiting.current?.stop();
          // The scroll view adds its room in the same moment, so the input is measured after that has been laid out.
          setTimeout(place, 50);
        });
        waiting.current = {
          stop: () => {
            clearTimeout(timer);
            shown.remove();
            waiting.current = null;
          },
        };
      }}
    />
  );
});
