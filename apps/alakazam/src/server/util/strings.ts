export const toFileName = (filePath: string) => {
  const parts = filePath?.split(/[/\\]/) ?? '';
  return parts[parts.length - 1];
};
