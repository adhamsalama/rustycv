import { describe, expect, it } from 'vitest'
import {
  bulletsToDoc,
  docToBullets,
  docToValue,
  editorToRuns,
  fromRuns,
  isEmpty,
  plainText,
  runsToEditor,
  toRuns,
  valueToDoc,
  type RichText,
} from './rich'

describe('the two stored forms', () => {
  it('reads a bare string as one unstyled run', () => {
    expect(toRuns('Reduced latency by 50%.')).toEqual([{ text: 'Reduced latency by 50%.' }])
    expect(toRuns('')).toEqual([])
    expect(toRuns(null)).toEqual([])
  })

  it('writes unstyled text back as a bare string', () => {
    // Matches what the Rust side emits, so a save never rewrites an unformatted
    // document into arrays of objects.
    expect(fromRuns([{ text: 'Hello' }])).toBe('Hello')
    expect(fromRuns([])).toBe('')
    expect(fromRuns([{ text: '' }])).toBe('')
  })

  it('keeps runs when a mark would otherwise be lost', () => {
    expect(fromRuns([{ text: 'Hello', bold: true }])).toEqual([{ text: 'Hello', bold: true }])
  })

  it('drops mark keys that are false rather than storing them', () => {
    expect(fromRuns([{ text: 'a', bold: false, link: '' }])).toBe('a')
  })
})

describe('editor round trips', () => {
  const cases: [string, RichText][] = [
    ['plain', 'Reduced latency by 50%.'],
    ['empty', ''],
    ['bold', [{ text: 'Improved to ' }, { text: '98.5%', bold: true }]],
    ['every mark', [{ text: 'a', bold: true, italic: true, underline: true }]],
    ['a link', [{ text: 'docs', link: 'https://example.com' }]],
    ['a formatted link', [{ text: 'docs', bold: true, link: 'https://example.com' }]],
  ]

  it.each(cases)('survives a trip through the editor: %s', (_name, value) => {
    expect(docToValue(valueToDoc(value))).toEqual(value)
  })

  it('merges runs that share every mark', () => {
    // TipTap splits text at edit boundaries; without merging, the stored
    // document fragments further with every keystroke.
    const runs = editorToRuns([
      { type: 'text', text: 'Impro' },
      { type: 'text', text: 'ved to ' },
      { type: 'text', text: '98', marks: [{ type: 'bold' }] },
      { type: 'text', text: '.5%', marks: [{ type: 'bold' }] },
    ])
    expect(runs).toEqual([{ text: 'Improved to ' }, { text: '98.5%', bold: true }])
  })

  it('keeps runs separate when their marks differ', () => {
    const runs = editorToRuns([
      { type: 'text', text: 'a', marks: [{ type: 'bold' }] },
      { type: 'text', text: 'b', marks: [{ type: 'italic' }] },
    ])
    expect(runs).toHaveLength(2)
  })

  it('ignores marks it does not store', () => {
    const runs = editorToRuns([{ type: 'text', text: 'a', marks: [{ type: 'strike' }] }])
    expect(runs).toEqual([{ text: 'a' }])
  })

  it('drops a link mark with no href instead of storing an empty link', () => {
    const runs = editorToRuns([{ type: 'text', text: 'a', marks: [{ type: 'link', attrs: {} }] }])
    expect(runs).toEqual([{ text: 'a' }])
  })

  it('turns a hard break into a space rather than losing the text after it', () => {
    const runs = editorToRuns([
      { type: 'text', text: 'one' },
      { type: 'hardBreak' },
      { type: 'text', text: 'two' },
    ])
    expect(plainText(fromRuns(runs))).toBe('one two')
  })

  it('collects text from unexpected wrappers', () => {
    const runs = editorToRuns([{ type: 'weird', content: [{ type: 'text', text: 'kept' }] }])
    expect(runs).toEqual([{ text: 'kept' }])
  })

  it('emits no marks array for unstyled runs', () => {
    expect(runsToEditor([{ text: 'a' }])).toEqual([{ type: 'text', text: 'a' }])
  })
})

describe('bullet lists', () => {
  it('round trips a list of bullets', () => {
    const bullets: RichText[] = ['First', [{ text: 'Second ' }, { text: 'bold', bold: true }]]
    expect(docToBullets(bulletsToDoc(bullets))).toEqual(bullets)
  })

  it('gives an empty list one bullet to type into', () => {
    // ProseMirror rejects a bulletList with no listItem, so the editor would
    // refuse to mount with an empty document.
    const doc = bulletsToDoc([])
    expect(doc.content?.[0]?.content).toHaveLength(1)
  })

  it('stores nothing when the only bullet is empty', () => {
    expect(docToBullets(bulletsToDoc([]))).toEqual([])
    expect(docToBullets(bulletsToDoc(['']))).toEqual([])
  })

  it('keeps a blank bullet in the middle of a list', () => {
    expect(docToBullets(bulletsToDoc(['a', '', 'b']))).toEqual(['a', '', 'b'])
  })
})

describe('emptiness', () => {
  it('treats whitespace as empty', () => {
    expect(isEmpty('   ')).toBe(true)
    expect(isEmpty('')).toBe(true)
    expect(isEmpty(null)).toBe(true)
    expect(isEmpty([{ text: ' ', bold: true }])).toBe(true)
    expect(isEmpty('x')).toBe(false)
  })

  it('reads plain text through formatting', () => {
    expect(plainText([{ text: 'a' }, { text: 'b', bold: true }])).toBe('ab')
  })
})
