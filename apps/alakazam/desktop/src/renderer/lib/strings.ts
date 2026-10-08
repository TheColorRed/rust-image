/**
 * Converts a string to title case, capitalizing the first letter of each word and converting the rest to lowercase.
 * Words are defined as sequences of characters separated by spaces, hyphens, or underscores.
 * @param str The input string to be converted.
 * @returns The title-cased string.
 */
export function titleCase(str: string): string {
  return str
    .toLowerCase()
    .split(/\s|-|_/)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}
/**
 * A fuzzy search function that validates if the query string matches the text string. The characters in the query must appear in order within the text, but they do not need to be contiguous.
 * @param query The search query string.
 * @param text The text string to search within.
 * @returns True if the query matches the text in a fuzzy manner, false otherwise.
 */
export function fuzzySearch(query: string, text: string): boolean {
  const queryLower = query.toLowerCase();
  const textLower = text.toLowerCase();

  let queryIndex = 0;
  let textIndex = 0;

  while (queryIndex < queryLower.length && textIndex < textLower.length) {
    if (queryLower[queryIndex] === textLower[textIndex]) {
      queryIndex++;
    }
    textIndex++;
  }

  return queryIndex === queryLower.length;
}
