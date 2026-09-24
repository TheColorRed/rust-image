import { AlakazamApi } from '../server/preload/alakazam';
import { AlakazamHistoryApi } from '../server/preload/alakazam-history';
import { AlakazamClipboardApi } from '../server/preload/clipboard';
import { AlakazamGizmosApi } from '../server/preload/gizmos';
import { AlakazamToolsApi } from '../server/preload/tools';

export { AlakazamApi } from '../server/preload/alakazam';
export { AlakazamHistoryApi } from '../server/preload/alakazam-history';
export { AlakazamGizmosApi } from '../server/preload/gizmos';
export { AlakazamToolsApi } from '../server/preload/tools';

declare global {
  interface Window {
    alakazam: AlakazamApi;
    alakazamHistory: AlakazamHistoryApi;
    clipboard: AlakazamClipboardApi;
    tools: AlakazamToolsApi;
    gizmos: AlakazamGizmosApi;
  }
}
