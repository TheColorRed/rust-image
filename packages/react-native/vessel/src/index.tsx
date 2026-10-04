import { forwardRef, useEffect, useState } from 'react';
import { Platform, requireNativeComponent, type StyleProp, type ViewProps, type ViewStyle } from 'react-native';

/** Anything Vessel can show on a `VesselView`: a component made in Rust that has `mount` and `unmount`. */
export interface VesselSource {
  mount(view: number): void;
  unmount(view: number): void;
}

export interface VesselViewProps extends ViewProps {
  /** What to show. Nothing is drawn while it is `null`. */
  source: VesselSource | null;
  style?: StyleProp<ViewStyle>;
}

interface NativeVesselViewProps extends ViewProps {
  /** The number this view reports its surface under; Vessel draws what is mounted on it. */
  surfaceId: number;
}

const NativeVesselView = Platform.OS === 'android' ? requireNativeComponent<NativeVesselViewProps>('VesselView') : null;

/** Gives each view its own number, so the developer never picks one. */
let lastView = 0;

/**
 * A view that Vessel draws on directly from Rust: give it a `source` and it is shown, at the view's size, with nothing else
 * to set up. It works out which native view it is and connects the source to it. Android only; elsewhere it is `null`.
 */
export const VesselView = NativeVesselView
  ? forwardRef<React.ElementRef<typeof NativeVesselView>, VesselViewProps>(function VesselView({ source, ...props }, ref) {
      const [view] = useState(() => ++lastView);
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
