import { IconDefinition } from '@fortawesome/fontawesome-svg-core';

export function toSvgCursor(def: IconDefinition) {
  const data = def.icon[4];
  return `data:image/svg+xml;base64,${btoa(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="20" height="20" fill="currentColor">
        <path d="${data}" stroke="white" stroke-width="15" stroke-linejoin="round" stroke-linecap="round" />
      </svg>`)}`;
}
