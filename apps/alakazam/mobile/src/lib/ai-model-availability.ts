import { aiModelPath } from '@alakazam/mobile';

/**
 * The roles among `roles` whose AI model is not ready on this device: not downloaded, or downloaded without the manifest
 * that `AiModel.load` needs. Works offline.
 */
export const missingModels = (roles: readonly string[]): string[] =>
  roles.filter(role => {
    try {
      aiModelPath(role);
      return false;
    } catch {
      return true;
    }
  });
