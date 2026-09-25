import { useEffect } from 'react'
import { EditorContent, useEditor, type Editor } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import {
  bulletsToDoc,
  docToBullets,
  docToValue,
  valueToDoc,
  type EditorNode,
  type RichText,
} from '../rich'

/**
 * Only the marks the stored format can represent are enabled. Anything else a
 * user might paste in — headings, code blocks, tables — would be silently
 * dropped on save, so it is better not to offer it at all.
 */
const extensions = (withLists: boolean) => [
  StarterKit.configure({
    heading: false,
    blockquote: false,
    codeBlock: false,
    code: false,
    strike: false,
    horizontalRule: false,
    orderedList: false,
    bulletList: withLists ? {} : false,
    listItem: withLists ? {} : false,
    listKeymap: withLists ? {} : false,
    link: { openOnClick: false },
  }),
]

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
    </div>
  )
}

interface EditorShellProps {
  label: string
  doc: EditorNode
  withLists: boolean
  /** Reject Enter, for fields that hold a single value. */
  singleParagraph?: boolean
  onChange: (doc: EditorNode) => void
  placeholder?: string
}

function EditorShell({
  label,
  doc,
  withLists,
  singleParagraph,
  onChange,
  placeholder,
}: EditorShellProps) {
  const editor = useEditor({
    extensions: extensions(withLists),
    content: doc,
    editorProps: {
      attributes: { class: 'rt-content', 'aria-label': label },
      handleKeyDown: (_view, event) =>
        // Returning true swallows the key. A one-value field has nowhere to put
        // a second paragraph, so Enter would create content that is lost on save.
        Boolean(singleParagraph) && event.key === 'Enter' && !event.shiftKey,
    },
    onUpdate: ({ editor }) => onChange(editor.getJSON() as EditorNode),
  })

  // Adopt content that changed underneath us — undo, a template switch, a
  // different CV loaded — without disturbing the caret during normal typing.
  useEffect(() => {
    if (!editor) return
    const incoming = JSON.stringify(doc)
    if (incoming !== JSON.stringify(editor.getJSON())) {
      editor.commands.setContent(doc, { emitUpdate: false })
    }
    // Comparing serialized documents is what makes this safe to run on every
    // render; keying off `doc` identity alone would fight the user's typing.
  }, [editor, doc])

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

/** One rich value — a summary, or an entry's description. */
export function RichTextField({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string
  value: RichText
  onChange: (value: RichText) => void
  placeholder?: string
}) {
  return (
    <div className="field">
      <span className="field-label">{label}</span>
      <EditorShell
        label={label}
        doc={valueToDoc(value)}
        withLists={false}
        singleParagraph
        placeholder={placeholder}
        onChange={(doc) => onChange(docToValue(doc))}
      />
    </div>
  )
}

/**
 * An entry's highlights: one editor holding a bullet list, so Enter starts the
 * next bullet the way it does in a document.
 */
export function RichBulletsField({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string
  value: RichText[]
  onChange: (value: RichText[]) => void
  placeholder?: string
}) {
  return (
    <div className="field">
      <span className="field-label">{label}</span>
      <EditorShell
        label={label}
        doc={bulletsToDoc(value)}
        withLists
        placeholder={placeholder}
        onChange={(doc) => onChange(docToBullets(doc))}
      />
    </div>
  )
}
