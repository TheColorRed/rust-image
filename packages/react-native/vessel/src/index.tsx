import { forwardRef, useEffect, useState } from 'react';
import {
  Platform,
  UIManager,
  useWindowDimensions,
  requireNativeComponent,
  type StyleProp,
  type ViewProps,
  type ViewStyle,
} from 'react-native';

/** Anything Vessel can show on a `VesselView`: a component made in Rust that has `mount` and `unmount`. */
export interface VesselSource {
  setHostFont(path: string | undefined, size: number): boolean;
  mount(view: number): void;
  unmount(view: number): void;
}

export interface VesselViewProps extends ViewProps {
  /** What to show. Nothing is drawn while it is `null`. */
  source: VesselSource | null;
  style?: StyleProp<ViewStyle>;
  /** Optional local font-file path. The platform UI font is used by default. */
  fontPath?: string;
  /** Optional logical text size. Defaults to the platform's normal text size. */
  fontSize?: number;
  /** Whether accessibility text scaling applies. Defaults to true. */
  allowFontScaling?: boolean;
}

interface NativeVesselViewProps extends ViewProps {
  /** The number this view reports its surface under; Vessel draws what is mounted on it. */
  surfaceId: number;
}

const NativeVesselView =
  Platform.OS === 'android' || Platform.OS === 'ios'
    ? requireNativeComponent<NativeVesselViewProps>('VesselView')
    : null;

/** Gives each view its own number, so the developer never picks one. */
let lastView = 0;

function hostFontDefaults(): { size: number; path?: string } {
  const fallback = { size: Platform.OS === 'android' ? 14 : 17 };
  if (!NativeVesselView) return fallback;
  const config = UIManager.getViewManagerConfig('VesselView');
  const constants = config && 'Constants' in config ? config.Constants : undefined;
  if (!constants || typeof constants !== 'object') return fallback;
  const size = 'defaultFontSize' in constants ? constants.defaultFontSize : undefined;
  const path = 'defaultFontPath' in constants ? constants.defaultFontPath : undefined;
  return {
    size: typeof size === 'number' && Number.isFinite(size) && size > 0 ? size : fallback.size,
    path: typeof path === 'string' ? path : undefined,
  };
}

const fontDefaults = hostFontDefaults();

/**
 * A view that Vessel draws on directly from Rust: give it a `source` and it is shown, at the view's size, with nothing else
 * to set up. It works out which native view it is and connects the source to it. Android and iOS; elsewhere it is `null`.
 */
export const VesselView = NativeVesselView
  ? forwardRef<React.ElementRef<typeof NativeVesselView>, VesselViewProps>(function VesselView(
      { source, fontPath, fontSize, allowFontScaling = true, ...props },
      ref,
    ) {
      const [view] = useState(() => ++lastView);
      const { scale, fontScale } = useWindowDimensions();
      const logicalSize = fontSize ?? fontDefaults.size;
      if (!Number.isFinite(logicalSize) || logicalSize <= 0) {
        throw new Error('VesselView fontSize must be finite and positive');
      }
      const pixelSize = Math.max(1, Math.round(logicalSize * scale * (allowFontScaling ? fontScale : 1)));
      if (!Number.isFinite(pixelSize) || pixelSize > 0xffffffff) {
        throw new Error('VesselView scaled fontSize exceeds the supported pixel range');
      }
      const path = fontPath ?? fontDefaults.path;
      useEffect(() => {
        if (source && !source.setHostFont(path, pixelSize)) {
          throw new Error(`VesselView could not load its host font: ${path}`);
        }
      }, [source, path, pixelSize]);
      useEffect(() => {
        if (!source) return;
        source.mount(view);
        return () => {
          try {
            source.unmount(view);
          } catch {
            // The source was freed first, which took it off the view already.
          }
        };
      }, [source, view]);
      return <NativeVesselView ref={ref} surfaceId={view} {...props} />;
    })
  : null;
