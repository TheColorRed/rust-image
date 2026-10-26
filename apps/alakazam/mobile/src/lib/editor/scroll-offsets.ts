const offsets = new Map<string, number>();

export function getScrollOffset(key: string): number {
  return offsets.get(key) ?? 0;
}

export function setScrollOffset(key: string, offset: number): void {
  offsets.set(key, offset);
}

export function clearScrollOffsets(): void {
  offsets.clear();
}

export function clearScrollOffset(key: string): void {
  offsets.delete(key);
}
