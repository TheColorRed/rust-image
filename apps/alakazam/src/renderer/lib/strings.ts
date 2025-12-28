export function titleCase(str: string): string {
  return str
    .toLowerCase()
    .split(/\s|-|_/)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}
