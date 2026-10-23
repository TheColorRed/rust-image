import { useTheme } from '@/src/lib/ui/theme';
import {
  areaDrawKey,
  areaDrawKey$,
  areaDrawMode,
  areaDrawMode$,
  areaRings,
  areas,
  areas$,
  areaTarget,
  areaTarget$,
} from '@/src/state/draw-area';
import Eraser from 'lucide-react-native/icons/eraser';
import Minus from 'lucide-react-native/icons/minus';
import Pencil from 'lucide-react-native/icons/pencil';
import Plus from 'lucide-react-native/icons/plus';
import { useEffect, useMemo } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import { distinctUntilChanged, map } from 'rxjs/operators';

const ICON = '#fff';

/** What to tell the person while they draw, or null when nothing is being drawn. */
export function useDrawAreaHint(): string | null {
  const target = useObservable(areaTarget$, areaTarget.value);
  const drawing = useObservable(areaDrawKey$, areaDrawKey.value);
  const mode = useObservable(areaDrawMode$, areaDrawMode.value);
  return target && drawing === target.key ? target.hint[mode] : null;
}

/**
 * Draw area, + and −: the buttons that turn drawing on for the area the open control offers (see `AreaTarget`), and say whether a loop
 * starts the area over, adds to it or takes away from it. They show only while a control offers an area, so any control can use them. Mounted once,
 * because it also ends the drawing when the area changes, so a loop never lands on the wrong one.
 */
export function DrawAreaButtons() {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const target = useObservable(areaTarget$, areaTarget.value);
  const drawing = useObservable(areaDrawKey$, areaDrawKey.value);
  const mode = useObservable(areaDrawMode$, areaDrawMode.value);
  const drawn = useObservable(areas$, areas.value);

  useEffect(() => {
    // A control that starts drawing as it opens (it is nothing but drawing) has the drawing on; every other starts with it off.
    const subscription = areaTarget$
      .pipe(
        map(open => open?.key ?? null),
        distinctUntilChanged(),
      )
      .subscribe(() => {
        areaDrawMode.next('new');
        areaDrawKey.next(areaTarget.value?.startsDrawing ? areaTarget.value.key : null);
      });
    return () => subscription.unsubscribe();
  }, []);

  if (!target) return null;
  const active = drawing === target.key;
  // "+", "−" and Clear only edit an area that is already drawn. "Draw area" starts the first one.
  const editable = target.canDraw && areaRings(drawn, target.key).length > 0;
  // "Draw area" turns drawing on and off, so the photo can be panned and zoomed in between. While it is on, a loop makes a new area in
  // place of the old one, unless "+" is selected (it adds to the area) or "−" is (it takes away from it). Tapping the one that is
  // selected again goes back to making a new area, and drawing stays on.
  const toggleDrawing = () => {
    areaDrawMode.next('new');
    areaDrawKey.next(active ? null : target.key);
  };
  const toggleMode = (next: 'add' | 'subtract') => {
    areaDrawMode.next(active && areaDrawMode.value === next ? 'new' : next);
    areaDrawKey.next(target.key);
  };

  return (
    <View style={styles.group}>
      <Pressable
        accessibilityRole="switch"
        accessibilityState={{ checked: active, disabled: !target.canDraw }}
        accessibilityLabel="Draw area"
        disabled={!target.canDraw}
        style={[styles.segment, active && styles.on, !target.canDraw && styles.locked]}
        hitSlop={8}
        onPress={toggleDrawing}
      >
        <Pencil color={ICON} size={16} />
        <Text style={styles.label}>Draw area</Text>
      </Pressable>
      <View style={styles.divider} />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Clear the area"
        accessibilityState={{ disabled: !editable }}
        disabled={!editable}
        style={[styles.segment, !editable && styles.locked]}
        hitSlop={8}
        onPress={target.onClear}
      >
        <Eraser color={ICON} size={16} />
      </Pressable>
      <View style={styles.divider} />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={target.hint.add}
        accessibilityState={{ checked: active && mode === 'add', disabled: !editable }}
        disabled={!editable}
        style={[styles.segment, active && mode === 'add' && styles.on, !editable && styles.locked]}
        hitSlop={8}
        onPress={() => toggleMode('add')}
      >
        <Plus color={ICON} size={16} />
      </Pressable>
      <View style={styles.divider} />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={target.hint.subtract}
        accessibilityState={{ checked: active && mode === 'subtract', disabled: !editable }}
        disabled={!editable}
        style={[styles.segment, active && mode === 'subtract' && styles.on, !editable && styles.locked]}
        hitSlop={8}
        onPress={() => toggleMode('subtract')}
      >
        <Minus color={ICON} size={16} />
      </Pressable>
    </View>
  );
}

function createStyles(colors: ReturnType<typeof useTheme>['colors']) {
  return StyleSheet.create({
    group: {
      flexDirection: 'row',
      alignItems: 'center',
      height: 34,
      borderRadius: 17,
      overflow: 'hidden',
      backgroundColor: colors.overlayButton,
    },
    segment: {
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 6,
      height: 34,
      paddingHorizontal: 12,
    },
    divider: { width: 1, alignSelf: 'stretch', backgroundColor: 'rgba(255,255,255,0.35)' },
    on: { backgroundColor: colors.accent },
    locked: { opacity: 0.6 },
    label: { color: ICON, fontSize: 13, fontWeight: '600' },
  });
}
