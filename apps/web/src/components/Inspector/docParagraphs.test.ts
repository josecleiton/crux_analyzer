import { describe, expect, it } from 'vitest';
import { docParagraphs } from './docParagraphs';

describe('docParagraphs', () => {
  it('joins hand-wrapped lines into one paragraph', () => {
    // `///` comments are wrapped at ~80 columns; those breaks are not the
    // author asking for a line break in a 280px panel.
    expect(docParagraphs('One line\nwrapped by hand.')).toEqual(['One line wrapped by hand.']);
  });

  it('splits on a blank line', () => {
    expect(docParagraphs('First.\n\nSecond.')).toEqual(['First.', 'Second.']);
  });

  it('tolerates whitespace-only separators and CRLF', () => {
    expect(docParagraphs('First.\n   \nSecond.')).toEqual(['First.', 'Second.']);
    expect(docParagraphs('First.\r\n\r\nSecond.')).toEqual(['First.', 'Second.']);
  });

  it('drops empty and whitespace-only input', () => {
    expect(docParagraphs('')).toEqual([]);
    expect(docParagraphs('   \n\n  ')).toEqual([]);
  });

  it('leaves markdown syntax literal', () => {
    // No markdown renderer here on purpose: the generated document is the
    // client for that.
    expect(docParagraphs('Uses `AICore` and **bold**.')).toEqual([
      'Uses `AICore` and **bold**.',
    ]);
  });
});
