import { AlakazamApi } from '../server/preload';
export { AlakazamApi } from '../server/preload';

declare global {
  interface Window {
    alakazam: AlakazamApi;
  }
}
