/**
 * Splits a source doc comment into paragraphs.
 *
 * Rust `///` comments are hard-wrapped at around 80 columns, so rendering them
 * with `white-space: pre-wrap` would reproduce those breaks as a ragged column
 * in a narrow panel. Blank lines separate paragraphs and single newlines are
 * soft — the same reading Markdown gives them, which is why no Markdown
 * library is needed here.
 *
 * Everything else in a doc comment (`**bold**`, backticks, links) renders
 * literally. The generated Markdown document is the client that renders
 * Markdown as Markdown.
 */
export function docParagraphs(doc: string): string[] {
  return doc
    // Normalize first, so a source written on Windows splits the same way.
    .replace(/\r\n?/g, '\n')
    .split(/\n[ \t]*\n/)
    .map((paragraph) => paragraph.replace(/\s*\n\s*/g, ' ').trim())
    .filter(Boolean);
}
