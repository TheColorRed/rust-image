import { useEffect, useMemo } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import { combineArea } from '@/src/lib/native/area-mask';
import { refreshAiModels } from '@/src/lib/ai/ai-model-catalog';
import { errorMessage } from '@/src/lib/utils/error-message';
import { useTheme, type ThemeColors } from '@/src/lib/ui/theme';
import { aiModelList, aiModelList$, type AiModelListing } from '@/src/state/ai-models';
import { focusedControlKey, isObjectRemovalFocused$ } from '@/src/state/edits';
import { currentImage } from '@/src/state/session';
import { useAreaTarget } from '@/src/components/tools/draw-area';
import { areaRings, areas, areas$, setArea, type DrawMode, type DrawnPoint } from '@/src/state/draw-area';
import { objectRemovalToolKey, objectRemovalToolKey$ } from '@/src/state/plugin-tool-panels';
import { toolAiModelSelections, toolAiModelSelections$, setToolAiModel } from '@/src/state/tool-ai-models';
import { toolMessages, toolMessages$ } from '@/src/state/tool-messages';
import { app } from '@/src/lib/plugins/host-api';

/** Every model the catalog lists with the capability to fill a masked part of a photo, in the catalog's order. */
const removersIn = (models: readonly AiModelListing[]) =>
  models.filter(model => model.enabled && model.kinds.includes('inpaint'));

/** The remover to use: the one picked if the catalog still lists it, else the first. */
const removerFor = (models: readonly AiModelListing[], picked: string | null) => {
  const removers = removersIn(models);
  return removers.find(model => model.id === picked) ?? removers[0];
};

/** A loop was drawn around something to remove (see `AreaTarget`). It starts the circled area over, joins it, or takes part of it back out. */
function circleToRemove(loop: DrawnPoint[], mode: DrawMode) {
  const key = objectRemovalToolKey.value;
  const image = currentImage.value;
  const circled = key ? areaRings(areas.value, key) : [];
  // Taking away from nothing circled would circle the rest of the photo.
  if (!image || (mode === 'subtract' && circled.length === 0)) return;
  if (key) setArea(key, combineArea(circled, loop, mode, { width: image.width(), height: image.height() }));
}

/** The controls beneath the tool: what to do, and the button that removes what is circled. */
export function ObjectRemovalControls({ onRemove }: { onRemove: () => void }) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const drawn = useObservable(areas$, areas.value);
  const messages = useObservable(toolMessages$, toolMessages.value);
  const choices = useObservable(toolAiModelSelections$, toolAiModelSelections.value);
  const models = useObservable(aiModelList$, aiModelList.value);
  const toolKey = useObservable(objectRemovalToolKey$, objectRemovalToolKey.value);
  const removers = removersIn(models);
  const chosen = removerFor(models, toolKey ? choices[toolKey] ?? null : null);
  // The catalog is read when a screen needs it. Read it for this tool too, so the removers are here when it opens.
  useEffect(() => {
    if (removersIn(aiModelList.value).length === 0) refreshAiModels().catch(() => {});
  }, []);
  const circled = toolKey ? areaRings(drawn, toolKey).length > 0 : false;
  const said = toolKey ? messages[toolKey] : undefined;
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
              onPress={() => toolKey && setToolAiModel(toolKey, id)}
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
  const toolKey = useObservable(objectRemovalToolKey$, objectRemovalToolKey.value);
  // Circling is the whole of this tool, so it opens ready to draw.
  const target = useMemo(
    () =>
      focused && toolKey
        ? {
            key: toolKey,
            canDraw: true,
            startsDrawing: true,
            hint: {
              new: 'Circle what you want gone. Circling again starts over',
              add: 'Circle more to remove as well',
              subtract: 'Circle to take part of it back out',
            },
            opacity: 0.55,
            minShare: 0.0002,
            onClear: () => setArea(toolKey, []),
            onDrawn: circleToRemove,
          }
        : null,
    [focused, toolKey],
  );
  useAreaTarget('object-removal', target);

  useEffect(() => {
    let registration: { owner: string; role: string } | undefined;
    const prepare = (role: string) => {
      if (registration?.role === role) return;
      const owner = objectRemovalToolKey.value;
      if (!owner) return;
      app.tools.setMessage(owner, null);
      if (registration) app.ai.releaseModel(registration.owner, registration.role);
      const nextRegistration = { owner, role };
      registration = nextRegistration;
      app.ai.prepareModel(owner, role).catch(error => {
        if (registration === nextRegistration) {
          app.tools.setMessage(owner, `Could not load model: ${errorMessage(error)}`);
          app.ai.releaseModel(owner, role);
          registration = undefined;
        }
      });
    };

    const subscriptions = [
      isObjectRemovalFocused$.subscribe(focused => {
        if (focused) {
          const role = removerFor(aiModelList.value, toolKey ? toolAiModelSelections.value[toolKey] ?? null : null)?.id;
          if (role) prepare(role);
          return;
        }
        if (toolKey) {
          setArea(toolKey, []);
          app.tools.setMessage(toolKey, null);
        }
        if (registration) app.ai.releaseModel(registration.owner, registration.role);
        registration = undefined;
      }),
      toolAiModelSelections$.subscribe(selections => {
        if (focusedControlKey.value !== toolKey) return;
        const role = removerFor(aiModelList.value, toolKey ? selections[toolKey] ?? null : null)?.id;
        if (role) prepare(role);
      }),
    ];
    return () => {
      if (registration) app.ai.releaseModel(registration.owner, registration.role);
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
