/**
 * Conversion between the stored rich-text format and TipTap's document JSON.
 *
 * The stored format is deliberately small — a flat list of blocks, each a list
 * of styled runs, with no nesting — so these functions are the only place that
 * knows about TipTap's shape. They take a structural type rather than importing
 * TipTap's own, which keeps them pure and unit-testable without booting an
 * editor. `crates/rustycv-core/src/rich.rs` writes the same three forms and has
 * to stay in step.
 */

export interface Run {
  /** A newline here is a soft line break within the block. */
  text: string
  bold?: boolean
  italic?: boolean
  underline?: boolean
  /** Empty or absent when the run is not a link. */
  link?: string
}

/**
 * What a block is. A list is not a node of its own: consecutive items of one
 * kind render as one list, which is what keeps the format flat and unnestable.
 */
export type BlockKind = 'paragraph' | 'bullet' | 'numbered'

export interface Block {
  kind: BlockKind
  runs: Run[]
}

/**
 * Three forms, and a value is stored in the smallest one that fits: a bare
 * string for one unstyled paragraph, runs for one styled paragraph, and blocks
 * only when there is more than one of them or a list among them.
 */
export type RichText = string | Run[] | Block[]

/** The subset of TipTap's JSON these conversions touch. */
export interface EditorNode {
  type?: string
  text?: string
  marks?: { type: string; attrs?: Record<string, unknown> }[]
  content?: EditorNode[]
  attrs?: Record<string, unknown>
}

const MARKS = ['bold', 'italic', 'underline'] as const

const LIST_NODE: Record<Exclude<BlockKind, 'paragraph'>, string> = {
  bullet: 'bulletList',
  numbered: 'orderedList',
}

const BLOCK_OF_LIST: Record<string, BlockKind> = {
  bulletList: 'bullet',
  orderedList: 'numbered',
}

/** Normalise any stored form to blocks. */
export function toBlocks(value: RichText | null | undefined): Block[] {
  if (value == null) return []
  if (typeof value === 'string') return value === '' ? [] : [{ kind: 'paragraph', runs: [{ text: value }] }]
  if (value.length === 0) return []
  // A block carries `runs`; a run carries `text`. That is the whole difference,
  // and it is what the Rust side keys off too.
  if (isBlock(value[0])) {
    return (value as Block[])
      .map((block) => ({ kind: block?.kind ?? 'paragraph', runs: cleanRuns(block?.runs) }))
      .filter((block) => block.runs.length > 0)
  }
  const runs = cleanRuns(value as Run[])
  return runs.length > 0 ? [{ kind: 'paragraph', runs }] : []
}

function isBlock(value: Run | Block | undefined): value is Block {
  return value != null && Array.isArray((value as Block).runs)
}

function cleanRuns(runs: Run[] | undefined): Run[] {
  if (!Array.isArray(runs)) return []
  return runs.filter((run) => run && typeof run.text === 'string' && run.text !== '')
}

/** Every run, whichever block it sits in. */
export function toRuns(value: RichText | null | undefined): Run[] {
  return toBlocks(value).flatMap((block) => block.runs)
}

/**
 * The plain text behind rich text — for summaries, counts and emptiness.
 * Blocks are separated by newlines, the same as a line break inside one.
 */
export function plainText(value: RichText | null | undefined): string {
  return toBlocks(value)
    .map((block) => block.runs.map((run) => run.text).join(''))
    .join('\n')
}

export function isEmpty(value: RichText | null | undefined): boolean {
  return plainText(value).trim() === ''
}

/**
 * Collapse blocks back to the compact form where nothing would be lost.
 *
 * Keeps unstyled documents storing plain strings, so exports stay readable and
 * saving a CV that was never formatted doesn't rewrite every line into objects.
 */
export function fromBlocks(blocks: Block[]): RichText {
  const clean = blocks
    .map((block) => ({ kind: block.kind, runs: cleanRuns(block.runs).map(cleanRun) }))
    .filter((block) => block.runs.length > 0)

  if (clean.length === 0) return ''
  const only = clean[0]!
  if (clean.length === 1 && only.kind === 'paragraph') {
    if (only.runs.length === 1 && isPlain(only.runs[0]!)) return only.runs[0]!.text
    return only.runs
  }
  return clean
}

/** One paragraph of runs, for the places that never had blocks to begin with. */
export function fromRuns(runs: Run[]): RichText {
  return fromBlocks([{ kind: 'paragraph', runs }])
}

function cleanRun(run: Run): Run {
  const out: Run = { text: run.text }
  for (const mark of MARKS) if (run[mark]) out[mark] = true
  if (run.link) out.link = run.link
  return out
}

function isPlain(run: Run): boolean {
  return !run.bold && !run.italic && !run.underline && !run.link
}

/**
 * Runs → the inline content of one TipTap paragraph.
 *
 * A newline in a run is a soft break, which TipTap models as a `hardBreak` node
 * between two text nodes rather than as a character.
 */
export function runsToEditor(runs: Run[]): EditorNode[] {
  const nodes: EditorNode[] = []
  for (const run of runs) {
    const marks: { type: string; attrs?: Record<string, unknown> }[] = []
    for (const mark of MARKS) if (run[mark]) marks.push({ type: mark })
    if (run.link) marks.push({ type: 'link', attrs: { href: run.link } })

    run.text.split('\n').forEach((part, i) => {
      if (i > 0) nodes.push({ type: 'hardBreak' })
      if (part === '') return
      nodes.push(marks.length > 0 ? { type: 'text', text: part, marks } : { type: 'text', text: part })
    })
  }
  return nodes
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
      runs.push({ text: '\n' })
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

/**
 * A rich value as a TipTap document: paragraphs, and runs of list items of one
 * kind gathered into a single list node — the shape the editor needs to let
 * Enter continue a list rather than start a new one.
 *
 * `emptyAs` is what an empty value opens as. Highlights pass `bullet` so that a
 * fresh entry behaves the way it always has: click in, type, get bullets. The
 * stored form cannot carry that on its own — an empty list has no blocks.
 */
export function valueToDoc(
  value: RichText | null | undefined,
  emptyAs: BlockKind = 'paragraph',
): EditorNode {
  const content: EditorNode[] = []
  for (const block of toBlocks(value)) {
    const item = { type: 'listItem', content: [{ type: 'paragraph', content: runsToEditor(block.runs) }] }
    if (block.kind === 'paragraph') {
      content.push({ type: 'paragraph', content: runsToEditor(block.runs) })
      continue
    }
    const node = LIST_NODE[block.kind]
    const last = content[content.length - 1]
    if (last?.type === node) last.content!.push(item)
    else content.push({ type: node, content: [item] })
  }
  // ProseMirror needs something to put the caret in.
  if (content.length === 0) {
    const empty = { type: 'paragraph', content: [] }
    content.push(
      emptyAs === 'paragraph' ? empty : { type: LIST_NODE[emptyAs], content: [{ type: 'listItem', content: [empty] }] },
    )
  }
  return { type: 'doc', content }
}

/** A TipTap document back to a stored value. */
export function docToValue(doc: EditorNode): RichText {
  const blocks: Block[] = []

  for (const node of doc.content ?? []) {
    const kind = node.type ? BLOCK_OF_LIST[node.type] : undefined
    if (kind) {
      for (const item of node.content ?? []) {
        blocks.push({ kind, runs: itemRuns(item) })
      }
    } else {
      blocks.push({ kind: 'paragraph', runs: editorToRuns(node.content) })
    }
  }
  return fromBlocks(blocks)
}

/**
 * One list item's runs. An item can hold several paragraphs — pasted content
 * does this — and the format has no room for that, so they fold into one item
 * separated by line breaks rather than being dropped.
 */
function itemRuns(item: EditorNode): Run[] {
  const paragraphs = (item.content ?? []).map((child) => editorToRuns(child.content))
  if (paragraphs.length === 0) return editorToRuns(item.content)
  return mergeAdjacent(
    paragraphs.flatMap((runs, i) => (i === 0 ? runs : [{ text: '\n' } as Run, ...runs])),
  )
}
