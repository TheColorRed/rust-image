import { Platform, requireNativeComponent, type StyleProp, type ViewProps, type ViewStyle } from 'react-native';

/** The id the editor's preview view reports its surface under; a live preview attaches to it. */
export const LIVE_SURFACE_ID = 1;

interface AbraLiveViewProps extends ViewProps {
  surfaceId: number;
  style?: StyleProp<ViewStyle>;
}

/** A view the Rust live preview draws on directly. Android only; elsewhere frames come back through JS. */
export const AbraLiveView = Platform.OS === 'android' ? requireNativeComponent<AbraLiveViewProps>('AbraLiveView') : null;
