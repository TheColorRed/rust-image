import { useEffect, useMemo } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import { missingModels } from '@/src/lib/ai/ai-model-availability';
import { commitRetouch } from '@/src/lib/editor/editor-history';
import { flushFullReplay } from '@/src/lib/editor/editor-replay';
import { combineArea } from '@/src/lib/native/area-mask';
import { refreshAiModels } from '@/src/lib/ai/ai-model-catalog';
import { INPAINT, removeObjects } from '@/src/lib/tools/object-removal';
import { errorMessage } from '@/src/lib/utils/error-message';
import { useTheme, type ThemeColors } from '@/src/lib/ui/theme';
import { aiModelList, aiModelList$, aiModelsRequested, type AiModelListing } from '@/src/state/ai-models';
import { objectRemovalRequested$ } from '@/src/state/commands';
import { isObjectRemovalFocused$ } from '@/src/state/edits';
import { removalMessage, removalMessage$, removerRole, removerRole$ } from '@/src/state/object-removal';
import { historyReady } from '@/src/state/project-session';
import { busy, currentImage } from '@/src/state/session';
import { useAreaTarget } from '@/src/components/tools/draw-area';
import { OBJECT_REMOVAL_TOOL_KEY } from '@/src/lib/project/edit-sections/keys';
import { areaRings, areas, areas$, setArea, type DrawMode, type DrawnPoint } from '@/src/state/draw-area';

const MESSAGE_MS = 4000;

/** Every model the catalog lists with the capability to fill a masked part of a photo, in the catalog's order. */
const removersIn = (models: readonly AiModelListing[]) =>
  models.filter(model => model.enabled && model.kinds.includes(INPAINT));

/** The remover to use: the one picked if the catalog still lists it, else the first. */
const removerFor = (models: readonly AiModelListing[], picked: string | null) => {
  const removers = removersIn(models);
  return removers.find(model => model.id === picked) ?? removers[0];
};

/** A loop was drawn around something to remove (see `AreaTarget`). It starts the circled area over, joins it, or takes part of it back out. */
function circleToRemove(loop: DrawnPoint[], mode: DrawMode) {
  const image = currentImage.value;
  const circled = areaRings(areas.value, OBJECT_REMOVAL_TOOL_KEY);
  // Taking away from nothing circled would circle the rest of the photo.
  if (!image || (mode === 'subtract' && circled.length === 0)) return;
  setArea(OBJECT_REMOVAL_TOOL_KEY, combineArea(circled, loop, mode, { width: image.width(), height: image.height() }));
}

/** The controls beneath the tool: what to do, and the button that removes what is circled. */
export function ObjectRemovalControls({ onRemove }: { onRemove: () => void }) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const drawn = useObservable(areas$, areas.value);
  const said = useObservable(removalMessage$, null);
  const picked = useObservable(removerRole$, removerRole.value);
  const models = useObservable(aiModelList$, aiModelList.value);
  const removers = removersIn(models);
  const chosen = removerFor(models, picked);
  // The catalog is read when a screen needs it. Read it for this tool too, so the removers are here when it opens.
  useEffect(() => {
    if (removersIn(aiModelList.value).length === 0) refreshAiModels().catch(() => {});
  }, []);
  const circled = areaRings(drawn, OBJECT_REMOVAL_TOOL_KEY).length > 0;
  return (
    <View style={styles.controlRow}>
      {removers.length > 1 ? (
        <View style={styles.choices}>
          {removers.map(({ id, title }) => (
            <Pressable
              key={id}
              accessibilityRole="radio"
              accessibilityState={{ selected: id === chosen?.id }}
              style={[styles.choice, id === chosen?.id && styles.choiceOn]}
              onPress={() => removerRole.next(id)}
            >
              <Text style={[styles.choiceText, id === chosen?.id && styles.choiceTextOn]} numberOfLines={2}>
                {title}
              </Text>
            </Pressable>
          ))}
        </View>
      ) : null}
      <View style={styles.topRow}>
        <Text style={said ? styles.message : styles.hint}>
          {said ??
            (circled
              ? 'Tap + to circle more, or tap Remove'
              : 'Circle what you want gone. Tap Draw area to pan and zoom')}
        </Text>
      </View>
      <Pressable style={[styles.removeButton, !circled && styles.disabled]} onPress={onRemove} disabled={!circled}>
        <Text style={styles.removeButtonText}>Remove</Text>
      </Pressable>
    </View>
  );
}

/**
 * Wires up everything the object removal tool does on the editor: removing what is circled when Remove is pressed (offering to
 * download the AI model first), and forgetting the circled area when the tool closes.
 */
export function ObjectRemovalToolController() {
  const focused = useObservable(isObjectRemovalFocused$, false);
  // Circling is the whole of this tool, so it opens ready to draw.
  const target = useMemo(
    () =>
      focused
        ? {
            key: OBJECT_REMOVAL_TOOL_KEY,
            canDraw: true,
            startsDrawing: true,
            hint: {
              new: 'Circle what you want gone. Circling again starts over',
              add: 'Circle more to remove as well',
              subtract: 'Circle to take part of it back out',
            },
            opacity: 0.55,
            minShare: 0.0002,
            onClear: () => setArea(OBJECT_REMOVAL_TOOL_KEY, []),
            onDrawn: circleToRemove,
          }
        : null,
    [focused],
  );
  useAreaTarget('object-removal', target);

  useEffect(() => {
    let clearTimer: ReturnType<typeof setTimeout> | undefined;
    const say = (text: string) => {
      removalMessage.next(text);
      clearTimeout(clearTimer);
      clearTimer = setTimeout(() => removalMessage.next(null), MESSAGE_MS);
    };

    const run = async () => {
      // The removal is made on the full-size photo, so it must have this edit's full-size version first.
      flushFullReplay();
      const image = currentImage.value;
      const area = areaRings(areas.value, OBJECT_REMOVAL_TOOL_KEY);
      const role = removerFor(aiModelList.value, removerRole.value)?.id;
      // Until the history is built the image is a project's saved picture, and a removal made on it would be lost.
      if (!image || !historyReady.value || area.length === 0 || !role) return;
      busy.next(true);
      try {
        await removeObjects(image, area, role);
        // Undo or closing the editor while the model ran leaves a different photo open, which this removal is not for.
        if (currentImage.value !== image) return;
        const width = image.width();
        const height = image.height();
        const spots = area.map(ring => {
          const xs = ring.map(([x]) => x * width);
          const ys = ring.map(([, y]) => y * height);
          const [left, right, top, bottom] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
          return { x: (left + right) / 2, y: (top + bottom) / 2, radius: Math.hypot(right - left, bottom - top) / 2 };
        });
        setArea(OBJECT_REMOVAL_TOOL_KEY, []);
        removalMessage.next(null);
        commitRetouch({ type: 'object-removal', area, spots });
      } catch (error) {
        say(`Could not remove it: ${errorMessage(error)}`);
      } finally {
        busy.next(false);
      }
    };

    const subscriptions = [
      objectRemovalRequested$.subscribe(() => {
        if (areaRings(areas.value, OBJECT_REMOVAL_TOOL_KEY).length === 0) return;
        // The AI model is not on the device until it is downloaded. Offer it, and remove once it is here.
        const role = removerFor(aiModelList.value, removerRole.value)?.id;
        if (!role) {
          say('No object removal model is listed. Check your connection and try again.');
          return;
        }
        if (missingModels([role]).length > 0) {
          aiModelsRequested.next({ title: 'Object removal', roles: [role], onReady: run });
          return;
        }
        run();
      }),
      isObjectRemovalFocused$.subscribe(focused => {
        if (focused) return;
        setArea(OBJECT_REMOVAL_TOOL_KEY, []);
        removalMessage.next(null);
      }),
    ];
    return () => {
      clearTimeout(clearTimer);
      subscriptions.forEach(subscription => subscription.unsubscribe());
    };
  }, []);

  return null;
}

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
    controlRow: { alignItems: 'center', gap: 8, paddingHorizontal: 16 },
    topRow: { flexDirection: 'row', alignItems: 'center', gap: 10, alignSelf: 'stretch' },
    hint: { flex: 1, color: colors.textSecondary, fontSize: 12 },
    message: { flex: 1, color: colors.danger, fontSize: 12, fontWeight: '600' },
    disabled: { opacity: 0.45 },
    choices: { flexDirection: 'row', alignSelf: 'stretch', gap: 8 },
    choice: { flex: 1, alignItems: 'center', paddingVertical: 8, borderRadius: 16, borderWidth: 1, borderColor: colors.textSecondary },
    choiceOn: { backgroundColor: colors.accent, borderColor: colors.accent },
    choiceText: { color: colors.textPrimary, fontSize: 13, fontWeight: '600' },
    choiceTextOn: { color: '#fff' },
    removeButton: { paddingHorizontal: 20, paddingVertical: 10, borderRadius: 18, backgroundColor: colors.accent },
    removeButtonText: { color: '#fff', fontSize: 14, fontWeight: '700' },
  });
}
