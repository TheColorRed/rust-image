import type { EditSection } from '@/src/lib/project/edit-sections';
import { REMOVE_TOOL_KEY } from '@/src/lib/project/edit-sections/keys';

/** Things a person wants gone from a photo, from a tiny spot to a circled object. */
export const remove: EditSection = {
  key: 'section-remove',
  label: 'Remove',
  controls: [{ kind: 'tool', key: REMOVE_TOOL_KEY, label: 'Remove Object' }],
};
