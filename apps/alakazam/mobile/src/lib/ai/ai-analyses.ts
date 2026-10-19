import { type SliderControl } from '@/src/lib/project/edit-sections';

/**
 * Something the AI models find for the open photo that controls use (the faces and hair, the skin and the people), and how to find it
 * again. Whatever makes one registers it where it is made, so the regenerate button knows nothing of any model: it asks which analyses
 * a control uses, and shows itself when one of their models gives masks (the catalog's `kind`).
 */
export type AiAnalysis = {
  /** Every model it uses, by role. The button is there when any of them gives masks. */
  roles: readonly string[];
  /** The models that have to be on the device for it to run. */
  required: readonly string[];
  /** Whether the control uses what it finds. */
  usedBy: (control: SliderControl) => boolean;
  /** Throws away what was found and finds it again. */
  regenerate: () => void;
};

const registered = new Map<string, AiAnalysis>();

/** Registers an analysis under a name. Registering the same name again (Fast Refresh) replaces it. */
export const registerAiAnalysis = (name: string, analysis: AiAnalysis) => registered.set(name, analysis);

/** The analyses a control uses. */
export const analysesUsedBy = (control: SliderControl): AiAnalysis[] =>
  [...registered.values()].filter(analysis => analysis.usedBy(control));

/** The kinds of result in the catalog that are masks, and so can be found again by the regenerate button. */
export const MASK_KINDS = ['mask', 'labels', 'instances'];
