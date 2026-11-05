import { app, areaMask, areaSpots, owning, toolKey, type EditControl } from '@alakazam/sdk';

const TOOL_KEY = toolKey('remove-objects');

const run = async () => {
  const image = app.editor.currentImage();
  const area = app.editor.area(TOOL_KEY);
  if (!image || !app.editor.hasHistory() || area.length === 0) return;
  const model = app.ai.ensureModel(TOOL_KEY, 'inpaint', 'Object removal', () => void run());
  if (!model) return;

  const done = await app.ai.job(
    { key: TOOL_KEY, progress: 'object-removal', notice: { text: 'Removing Object(s)', subText: '', detail: model.title }, failure: 'Could not remove it' },
    () =>
      owning(async keep => {
        const mask = areaMask(area, image.width(), image.height(), keep);
        const answer = await app.ai.withModel(TOOL_KEY, model.id, loaded => app.ai.runGuided(loaded, image, mask));
        app.editor.replaceImage(image, keep(answer.image()));
      }),
  );
  if (!done || app.editor.currentImage() !== image) return;
  const spots = areaSpots(area, image.width(), image.height());
  app.editor.setArea(TOOL_KEY, []);
  app.history.commitRetouch({ type: 'object-removal', area, spots });
  try {
    await app.history.save();
  } catch (error) {
    app.tools.setMessage(TOOL_KEY, `Could not remove it: ${error instanceof Error ? error.message : String(error)}`);
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
