/** "VIFC Fintech Hub" → "vifc-fintech-hub": a workspace's folder name from its name (Vietnamese
 *  diacritics stripped; at most 40 characters, never ending in a hyphen). */
export function suggestSlug(name: string): string {
  return name
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/đ/g, "d")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+/, "")
    .slice(0, 40)
    .replace(/-+$/, "");
}
