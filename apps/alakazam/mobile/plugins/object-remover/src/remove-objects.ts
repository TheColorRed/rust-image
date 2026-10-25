import { app, areaMask, owning, setAiPendingNotice, toolKey, type EditControl } from '@alakazam/plugin-toolkit';

const TOOL_KEY = toolKey('remove-objects');
const INPAINT = 'inpaint';

const run = async () => {
  const image = app.editor.currentImage();
  const area = app.editor.area(TOOL_KEY);
  if (!image || !app.editor.hasHistory() || area.length === 0) return;
  const models = app.ai.models().filter(model => model.enabled && model.kinds.includes(INPAINT));
  const selected = app.ai.selectedModel(TOOL_KEY);
  const model = models.find(item => item.id === selected) ?? models[0];
  if (!model) {
    app.tools.setMessage(TOOL_KEY, 'No object removal model is listed. Check your connection and try again.');
    return;
  }
  if (app.ai.missing([model.id]).length > 0) {
    app.ai.requestModels('Object removal', [model.id], () => void run());
    return;
  }

  app.editor.setBusy(true);
  app.tools.setMessage(TOOL_KEY, null);
  setAiPendingNotice({ text: 'Removing Object(s)', subText: '', detail: model.title });
  try {
    await app.ai.withProgress('object-removal', () =>
      owning(async keep => {
        const mask = areaMask(area, image.width(), image.height(), keep);
        const answer = await app.ai.withModel(TOOL_KEY, model.id, loaded => app.ai.runGuided(loaded, image, mask));
        app.editor.replaceImage(image, keep(answer.image()));
      }),
    );
    if (app.editor.currentImage() !== image) return;
    const spots = area.map(ring => {
      const xs = ring.map(([x]) => x * image.width());
      const ys = ring.map(([, y]) => y * image.height());
      const [left, right, top, bottom] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
      return { x: (left + right) / 2, y: (top + bottom) / 2, radius: Math.hypot(right - left, bottom - top) / 2 };
    });
    app.editor.setArea(TOOL_KEY, []);
    app.history.commitRetouch({ type: 'object-removal', area, spots });
    await app.history.save();
  } catch (error) {
    app.tools.setMessage(TOOL_KEY, `Could not remove it: ${error instanceof Error ? error.message : String(error)}`);
  } finally {
    setAiPendingNotice(null);
    app.editor.setBusy(false);
  }
};

/** The object-removal tool entry in the Cleanup section. */
export default (): EditControl => ({
  kind: 'tool',
  key: TOOL_KEY,
  panel: 'object-removal',
  label: 'Remove Objects',
  onApply: () => void run(),
});
