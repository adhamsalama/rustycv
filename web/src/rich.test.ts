import { describe, expect, it } from 'vitest'
import { getSchema } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { Node } from '@tiptap/pm/model'
import {
  docToValue,
  editorToRuns,
  fromBlocks,
  fromRuns,
  isEmpty,
  plainText,
  runsToEditor,
  toBlocks,
  toRuns,
  valueToDoc,
  type EditorNode,
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

  it('keeps a hard break as a newline rather than flattening it to a space', () => {
    // Shift+Enter. The renderer breaks the line on a newline inside a run, so
    // this is what makes the second half start where the user put it.
    const runs = editorToRuns([
      { type: 'text', text: 'one' },
      { type: 'hardBreak' },
      { type: 'text', text: 'two' },
    ])
    expect(plainText(fromRuns(runs))).toBe('one\ntwo')
    expect(fromRuns(runs)).toBe('one\ntwo')
  })

  it('collects text from unexpected wrappers', () => {
    const runs = editorToRuns([{ type: 'weird', content: [{ type: 'text', text: 'kept' }] }])
    expect(runs).toEqual([{ text: 'kept' }])
  })

  it('emits no marks array for unstyled runs', () => {
    expect(runsToEditor([{ text: 'a' }])).toEqual([{ type: 'text', text: 'a' }])
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

describe('blocks', () => {
  const cases: [string, RichText][] = [
    ['two paragraphs', [
      { kind: 'paragraph', runs: [{ text: 'Led the rewrite.' }] },
      { kind: 'paragraph', runs: [{ text: 'Mentored two juniors.' }] },
    ]],
    ['a bulleted list', [
      { kind: 'bullet', runs: [{ text: 'Cut p99 latency.' }] },
      { kind: 'bullet', runs: [{ text: 'Shipped retries.' }] },
    ]],
    ['a numbered list', [
      { kind: 'numbered', runs: [{ text: 'Migrate.' }] },
      { kind: 'numbered', runs: [{ text: 'Cut over.' }] },
    ]],
    ['a paragraph, a list and a formatted run', [
      { kind: 'paragraph', runs: [{ text: 'Led the ' }, { text: 'rewrite', bold: true }] },
      { kind: 'bullet', runs: [{ text: 'Cut p99 latency.' }] },
    ]],
    ['a soft break inside a list item', [
      { kind: 'bullet', runs: [{ text: 'One\nTwo' }] },
      { kind: 'bullet', runs: [{ text: 'Three' }] },
    ]],
  ]

  it.each(cases)('survives a trip through the editor: %s', (_name, value) => {
    expect(docToValue(valueToDoc(value))).toEqual(value)
  })

  it('gathers neighbouring items of one kind into a single list', () => {
    // Two separate lists would restart the numbering and break Enter, which
    // continues the list node the caret is in.
    const doc = valueToDoc([
      { kind: 'bullet', runs: [{ text: 'One' }] },
      { kind: 'bullet', runs: [{ text: 'Two' }] },
      { kind: 'numbered', runs: [{ text: 'Three' }] },
    ])
    expect(doc.content?.map((node) => node.type)).toEqual(['bulletList', 'orderedList'])
    expect(doc.content?.[0]?.content).toHaveLength(2)
  })

  it('reads a paragraph the editor wrote and a list it wrote', () => {
    const value = docToValue({
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Intro' }] },
        {
          type: 'bulletList',
          content: [
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'One' }] }] },
            { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Two' }] }] },
          ],
        },
      ],
    })
    expect(value).toEqual([
      { kind: 'paragraph', runs: [{ text: 'Intro' }] },
      { kind: 'bullet', runs: [{ text: 'One' }] },
      { kind: 'bullet', runs: [{ text: 'Two' }] },
    ])
  })

  it('keeps paragraphs apart instead of welding them together', () => {
    // Pasting two paragraphs used to concatenate them with no separator at
    // all, so the PDF read "...rewrite.Mentored...".
    const value = docToValue({
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'First para.' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'Second para.' }] },
      ],
    })
    expect(plainText(value)).toBe('First para.\nSecond para.')
  })

  it('folds a list item that holds more than one paragraph', () => {
    // Pasted content does this, and the format has one value per item.
    const value = docToValue({
      type: 'doc',
      content: [
        {
          type: 'bulletList',
          content: [
            {
              type: 'listItem',
              content: [
                { type: 'paragraph', content: [{ type: 'text', text: 'One' }] },
                { type: 'paragraph', content: [{ type: 'text', text: 'Two' }] },
              ],
            },
          ],
        },
      ],
    })
    expect(value).toEqual([{ kind: 'bullet', runs: [{ text: 'One\nTwo' }] }])
  })

  it('stores the smallest form that loses nothing', () => {
    expect(fromBlocks([{ kind: 'paragraph', runs: [{ text: 'Hello' }] }])).toBe('Hello')
    expect(fromBlocks([{ kind: 'paragraph', runs: [{ text: 'Hello', bold: true }] }])).toEqual([
      { text: 'Hello', bold: true },
    ])
    expect(fromBlocks([{ kind: 'bullet', runs: [{ text: 'Hello' }] }])).toEqual([
      { kind: 'bullet', runs: [{ text: 'Hello' }] },
    ])
    expect(fromBlocks([])).toBe('')
  })

  it('drops blocks that would render nothing', () => {
    expect(
      fromBlocks([
        { kind: 'paragraph', runs: [{ text: 'One' }] },
        { kind: 'paragraph', runs: [{ text: '' }] },
        { kind: 'bullet', runs: [] },
      ]),
    ).toBe('One')
  })

  it('reads the older stored forms as one paragraph', () => {
    expect(toBlocks('Hello')).toEqual([{ kind: 'paragraph', runs: [{ text: 'Hello' }] }])
    expect(toBlocks([{ text: 'Hello', bold: true }])).toEqual([
      { kind: 'paragraph', runs: [{ text: 'Hello', bold: true }] },
    ])
    expect(toBlocks('')).toEqual([])
    expect(toBlocks(null)).toEqual([])
  })

  it('gives an empty value a paragraph to type into', () => {
    // ProseMirror needs a textblock to put the caret in.
    expect(valueToDoc('').content).toEqual([{ type: 'paragraph', content: [] }])
  })
})

describe('against TipTap\'s own schema', () => {
  // These conversions are pure, but they have to agree with the real editor:
  // TipTap fills in node defaults of its own, so a document we build is not
  // byte-identical to the one it hands back. What has to survive is the stored
  // value — `RichText.tsx` compares on that for exactly this reason.
  const schema = getSchema([
    StarterKit.configure({
      heading: false,
      blockquote: false,
      codeBlock: false,
      code: false,
      strike: false,
      horizontalRule: false,
      link: { openOnClick: false },
    }),
  ])

  /** What the editor would hand back for a document we gave it. */
  const throughEditor = (doc: EditorNode): EditorNode =>
    Node.fromJSON(schema, doc as never).toJSON() as EditorNode

  const values: [string, RichText][] = [
    ['a plain string', 'Reduced latency by 50%.'],
    ['a soft break', 'One\nTwo'],
    ['marks', [{ text: 'Improved to ' }, { text: '98.5%', bold: true }]],
    ['a link', [{ text: 'docs', link: 'https://example.com' }]],
    ['paragraphs', [
      { kind: 'paragraph', runs: [{ text: 'One' }] },
      { kind: 'paragraph', runs: [{ text: 'Two' }] },
    ]],
    ['a bulleted list', [
      { kind: 'bullet', runs: [{ text: 'One' }] },
      { kind: 'bullet', runs: [{ text: 'Two' }] },
    ]],
    ['a numbered list', [
      { kind: 'numbered', runs: [{ text: 'One' }] },
      { kind: 'numbered', runs: [{ text: 'Two' }] },
    ]],
    ['both kinds of list', [
      { kind: 'paragraph', runs: [{ text: 'Intro' }] },
      { kind: 'bullet', runs: [{ text: 'One' }] },
      { kind: 'numbered', runs: [{ text: 'Two' }] },
    ]],
    ['nothing', ''],
  ]

  it.each(values)('survives the editor schema: %s', (_name, value) => {
    expect(docToValue(throughEditor(valueToDoc(value)))).toEqual(value)
  })

  it('accepts every node the editor can produce', () => {
    // A document the schema rejects would throw here rather than render.
    for (const [, value] of values) expect(() => throughEditor(valueToDoc(value))).not.toThrow()
    expect(() => throughEditor(valueToDoc('', 'bullet'))).not.toThrow()
  })

  it('opens an empty highlights field as a list the editor accepts', () => {
    // Highlights have always behaved this way: click in, type, get bullets.
    // An empty value cannot carry that itself, so the field asks for it.
    const doc = throughEditor(valueToDoc('', 'bullet'))
    expect(doc.content?.[0]?.type).toBe('bulletList')
    expect(docToValue(doc)).toBe('')
  })
})
