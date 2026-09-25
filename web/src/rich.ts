/**
 * Conversion between the stored rich-text format and TipTap's document JSON.
 *
 * The stored format is deliberately small — a flat list of styled runs, no
 * nesting — so these functions are the only place that knows about TipTap's
 * shape. They take a structural type rather than importing TipTap's own, which
 * keeps them pure and unit-testable without booting an editor.
 */

export interface Run {
  text: string
  bold?: boolean
  italic?: boolean
  underline?: boolean
  /** Empty or absent when the run is not a link. */
  link?: string
}

/**
 * Unstyled text is stored as a bare string and formatted text as runs — see
 * `crates/rustycv-core/src/rich.rs`, which writes the same two forms.
 */
export type RichText = string | Run[]

/** The subset of TipTap's JSON these conversions touch. */
export interface EditorNode {
  type?: string
  text?: string
  marks?: { type: string; attrs?: Record<string, unknown> }[]
  content?: EditorNode[]
  attrs?: Record<string, unknown>
}

const MARKS = ['bold', 'italic', 'underline'] as const

/** Normalise either stored form to runs. */
export function toRuns(value: RichText | null | undefined): Run[] {
  if (value == null) return []
  if (typeof value === 'string') return value === '' ? [] : [{ text: value }]
  return value.filter((run) => run && typeof run.text === 'string' && run.text !== '')
}

/** The plain text behind rich text — for summaries, counts and emptiness. */
export function plainText(value: RichText | null | undefined): string {
  return toRuns(value)
    .map((run) => run.text)
    .join('')
}

export function isEmpty(value: RichText | null | undefined): boolean {
  return plainText(value).trim() === ''
}

/**
 * Collapse runs back to the compact form where nothing would be lost.
 *
 * Keeps unstyled documents storing plain strings, so exports stay readable and
 * saving a CV that was never formatted doesn't rewrite every line into objects.
 */
export function fromRuns(runs: Run[]): RichText {
  const clean = runs
    .map((run) => {
      const out: Run = { text: run.text }
      for (const mark of MARKS) if (run[mark]) out[mark] = true
      if (run.link) out.link = run.link
      return out
    })
    .filter((run) => run.text !== '')

  if (clean.length === 0) return ''
  if (clean.length === 1 && isPlain(clean[0]!)) return clean[0]!.text
  return clean
}

function isPlain(run: Run): boolean {
  return !run.bold && !run.italic && !run.underline && !run.link
}

/** Runs → the inline content of one TipTap paragraph. */
export function runsToEditor(runs: Run[]): EditorNode[] {
  return runs.map((run) => {
    const marks: { type: string; attrs?: Record<string, unknown> }[] = []
    for (const mark of MARKS) if (run[mark]) marks.push({ type: mark })
    if (run.link) marks.push({ type: 'link', attrs: { href: run.link } })
    return marks.length > 0 ? { type: 'text', text: run.text, marks } : { type: 'text', text: run.text }
  })
}

/**
 * TipTap inline content → runs.
 *
 * Walks nested nodes so a hard break or an unexpected wrapper contributes its
 * text rather than silently dropping it.
 */
export function editorToRuns(content: EditorNode[] | undefined): Run[] {
  const runs: Run[] = []

  const visit = (node: EditorNode) => {
    if (node.type === 'hardBreak') {
      runs.push({ text: ' ' })
      return
    }
    if (typeof node.text === 'string') {
      const run: Run = { text: node.text }
      for (const mark of node.marks ?? []) {
        if (mark.type === 'link') {
          const href = mark.attrs?.['href']
          if (typeof href === 'string' && href !== '') run.link = href
        } else if ((MARKS as readonly string[]).includes(mark.type)) {
          run[mark.type as (typeof MARKS)[number]] = true
        }
      }
      runs.push(run)
      return
    }
    for (const child of node.content ?? []) visit(child)
  }

  for (const node of content ?? []) visit(node)
  return mergeAdjacent(runs)
}

/**
 * Fold neighbouring runs that share every mark.
 *
 * TipTap splits text at every edit boundary, so typing into the middle of a
 * word yields several identical-looking runs. Merging keeps the stored document
 * from fragmenting a little more with each keystroke.
 */
function mergeAdjacent(runs: Run[]): Run[] {
  const out: Run[] = []
  for (const run of runs) {
    const last = out[out.length - 1]
    if (
      last &&
      !!last.bold === !!run.bold &&
      !!last.italic === !!run.italic &&
      !!last.underline === !!run.underline &&
      (last.link ?? '') === (run.link ?? '')
    ) {
      last.text += run.text
    } else {
      out.push({ ...run })
    }
  }
  return out.filter((run) => run.text !== '')
}

// ----------------------------------------------------- whole documents

/** A single rich value as a one-paragraph TipTap document. */
export function valueToDoc(value: RichText | null | undefined): EditorNode {
  return { type: 'doc', content: [{ type: 'paragraph', content: runsToEditor(toRuns(value)) }] }
}

/** Every paragraph of a TipTap document, flattened back to one value. */
export function docToValue(doc: EditorNode): RichText {
  return fromRuns(editorToRuns(doc.content))
}

/** A list of rich values as a TipTap document containing one bullet list. */
export function bulletsToDoc(values: RichText[]): EditorNode {
  const items = values.length > 0 ? values : ['']
  return {
    type: 'doc',
    content: [
      {
        type: 'bulletList',
        content: items.map((value) => ({
          type: 'listItem',
          content: [{ type: 'paragraph', content: runsToEditor(toRuns(value)) }],
        })),
      },
    ],
  }
}

/** A TipTap bullet-list document back to one rich value per bullet. */
export function docToBullets(doc: EditorNode): RichText[] {
  const bullets: RichText[] = []

  const visit = (node: EditorNode) => {
    if (node.type === 'listItem') {
      bullets.push(fromRuns(editorToRuns(node.content)))
      return
    }
    for (const child of node.content ?? []) visit(child)
  }

  for (const node of doc.content ?? []) visit(node)
  // A lone empty bullet is what an untouched editor looks like; store nothing.
  return bullets.length === 1 && isEmpty(bullets[0]!) ? [] : bullets
}
