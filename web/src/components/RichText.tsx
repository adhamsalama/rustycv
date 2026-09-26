import { useEffect } from 'react'
import { EditorContent, useEditor, type Editor } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { docToValue, valueToDoc, type BlockKind, type EditorNode, type RichText } from '../rich'

/**
 * Only the marks and blocks the stored format can represent are enabled.
 * Anything else a user might paste in — headings, code blocks, tables — would be
 * silently dropped on save, so it is better not to offer it at all.
 */
const extensions = [
  StarterKit.configure({
    heading: false,
    blockquote: false,
    codeBlock: false,
    code: false,
    strike: false,
    horizontalRule: false,
    link: { openOnClick: false },
  }),
]

/** Two stored values, compared by content. */
const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b)

function Toolbar({ editor }: { editor: Editor }) {
  // Subscribing to selection changes is what keeps the active states honest;
  // `editor.isActive` is read at render time and React has no other reason to
  // re-render as the caret moves.
  const button = (
    label: string,
    title: string,
    isActive: boolean,
    run: () => void,
  ) => (
    <button
      type="button"
      className={isActive ? 'rt-button active' : 'rt-button'}
      title={title}
      aria-pressed={isActive}
      // Keep the editor's selection: a click would otherwise blur it first and
      // the command would apply to nothing.
      onMouseDown={(e) => e.preventDefault()}
      onClick={run}
    >
      {label}
    </button>
  )

  const toggleLink = () => {
    const existing = editor.getAttributes('link')['href']
    const href = window.prompt('Link URL', typeof existing === 'string' ? existing : 'https://')
    if (href === null) return
    if (href.trim() === '') {
      editor.chain().focus().unsetLink().run()
      return
    }
    editor.chain().focus().extendMarkRange('link').setLink({ href: href.trim() }).run()
  }

  return (
    <div className="rt-toolbar">
      {button('B', 'Bold (⌘B)', editor.isActive('bold'), () =>
        editor.chain().focus().toggleBold().run(),
      )}
      {button('I', 'Italic (⌘I)', editor.isActive('italic'), () =>
        editor.chain().focus().toggleItalic().run(),
      )}
      {button('U', 'Underline (⌘U)', editor.isActive('underline'), () =>
        editor.chain().focus().toggleUnderline().run(),
      )}
      {button('🔗', 'Link', editor.isActive('link'), toggleLink)}
      {button('•', 'Bulleted list', editor.isActive('bulletList'), () =>
        editor.chain().focus().toggleBulletList().run(),
      )}
      {button('1.', 'Numbered list', editor.isActive('orderedList'), () =>
        editor.chain().focus().toggleOrderedList().run(),
      )}
    </div>
  )
}

interface EditorShellProps {
  label: string
  doc: EditorNode
  /** Whether the editor already holds the value `doc` was built from. */
  holdsValue: (current: EditorNode) => boolean
  onChange: (doc: EditorNode) => void
  placeholder?: string
}

function EditorShell({ label, doc, holdsValue, onChange, placeholder }: EditorShellProps) {
  const editor = useEditor({
    extensions,
    content: doc,
    editorProps: {
      attributes: { class: 'rt-content', 'aria-label': label },
    },
    onUpdate: ({ editor }) => onChange(editor.getJSON() as EditorNode),
  })

  // Adopt content that changed underneath us — undo, a template switch, a
  // different CV loaded — without disturbing the caret during normal typing.
  useEffect(() => {
    if (!editor) return
    if (!holdsValue(editor.getJSON() as EditorNode)) {
      editor.commands.setContent(doc, { emitUpdate: false })
    }
    // The comparison is on the *stored value*, not on the two documents: TipTap
    // fills in node defaults of its own (an ordered list carries `start`), so a
    // shape comparison would differ forever and reset the caret on every
    // keystroke. Keying off `doc` identity alone would fight the user's typing
    // just as badly.
  }, [editor, doc, holdsValue])

  useEffect(() => () => editor?.destroy(), [editor])

  if (!editor) return null

  return (
    <div className="rt-editor">
      <Toolbar editor={editor} />
      <EditorContent editor={editor} />
      {editor.isEmpty && placeholder ? <div className="rt-placeholder">{placeholder}</div> : null}
    </div>
  )
}

/**
 * One rich value — a summary, a description, or an entry's highlights. Enter
 * starts a new paragraph or the next list item; Shift+Enter is a line break
 * inside the one being written.
 *
 * `emptyAs` opens an untouched field as a list rather than a paragraph, which
 * is what highlights want.
 */
export function RichTextField({
  label,
  value,
  onChange,
  placeholder,
  emptyAs,
}: {
  label: string
  value: RichText
  onChange: (value: RichText) => void
  placeholder?: string
  emptyAs?: BlockKind
}) {
  return (
    <div className="field">
      <span className="field-label">{label}</span>
      <EditorShell
        label={label}
        doc={valueToDoc(value, emptyAs)}
        holdsValue={(current) => same(docToValue(current), value)}
        placeholder={placeholder}
        onChange={(doc) => onChange(docToValue(doc))}
      />
    </div>
  )
}
