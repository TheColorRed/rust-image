let selection: Map<string, [number, number][]> = new Map();
let feather: Map<string, number> = new Map();

/**
 * Sets the selection area for a project.
 * @param projectId The ID of the project.
 * @param area Array of [x, y] coordinates defining the selection area.
 */
export function setSelectionArea(projectId: string, area: [number, number][]) {
  selection.set(projectId, area);
}
/**
 * Gets the selection area for a project.
 * @param projectId The ID of the project.
 * @returns Array of [x, y] coordinates defining the selection area.
 */
export function getSelectionArea(projectId: string): [number, number][] {
  return selection.get(projectId) ?? [];
}
/**
 * Sets the feather amount for a project's selection.
 * @param projectId The ID of the project.
 * @param value The feather amount.
 */
export function setSelectionFeather(projectId: string, value: number) {
  feather.set(projectId, value);
}
/**
 * Gets the feather amount for a project's selection.
 * @param projectId The ID of the project.
 * @returns The feather amount.
 */
export function getSelectionFeather(projectId: string): number {
  return feather.get(projectId) ?? 0;
}
