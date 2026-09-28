import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { Cv, CvComment } from '../types'

/** Mirrors `comments::MAX_AUTHOR` / `MAX_BODY` on the server. */
const MAX_AUTHOR = 80
const MAX_BODY = 2000

function CommentList({
  comments,
  onDelete,
}: {
  comments: CvComment[]
  onDelete?: (comment: CvComment) => void
}) {
  if (comments.length === 0) return <p className="muted small">No comments yet.</p>
  return (
    <ul className="comment-list">
      {comments.map((comment) => (
        <li key={comment.id} className="comment">
          <div className="comment-meta">
            <strong>{comment.author || 'Anonymous'}</strong>
            <time className="muted small" dateTime={comment.createdAt}>
              {new Date(comment.createdAt).toLocaleDateString()}
            </time>
            {onDelete ? (
              <button type="button" className="ghost small" onClick={() => onDelete(comment)}>
                Delete
              </button>
            ) : null}
          </div>
          {/* Plain text: rendered as a string, never as markup. */}
          <p className="comment-body">{comment.body}</p>
        </li>
      ))}
    </ul>
  )
}

/**
 * What a visitor to a share link sees under the PDF: a way to send the owner a
 * note, and nothing else. Comments are private to the owner, so nobody else's
 * are shown here — not even the one just sent.
 */
export function PublicComments({ publicId }: { publicId: string }) {
  const [author, setAuthor] = useState('')
  const [body, setBody] = useState('')

  const post = useMutation({
    mutationFn: () => api.postComment(publicId, author, body),
    onSuccess: () => setBody(''),
  })

  return (
    <section className="share-comments" aria-label="Leave a comment">
      <h2>Leave a comment</h2>
      <p className="muted small">Only the owner of this CV will see it.</p>
      <form
        className="comment-form"
        onSubmit={(e) => {
          e.preventDefault()
          if (body.trim()) post.mutate()
        }}
      >
        <input
          placeholder="Your name (optional)"
          maxLength={MAX_AUTHOR}
          value={author}
          onChange={(e) => setAuthor(e.target.value)}
        />
        <textarea
          placeholder="Your comment"
          required
          maxLength={MAX_BODY}
          rows={3}
          value={body}
          onChange={(e) => {
            setBody(e.target.value)
            if (post.isSuccess) post.reset()
          }}
        />
        {post.isError ? (
          <p className="auth-error" role="alert">
            {(post.error as Error).message}
          </p>
        ) : null}
        {post.isSuccess ? (
          <p className="muted small" role="status">
            Sent. Thanks!
          </p>
        ) : null}
        <div>
          <button type="submit" className="primary" disabled={post.isPending || !body.trim()}>
            {post.isPending ? 'Sending…' : 'Send comment'}
          </button>
        </div>
      </form>
    </section>
  )
}

/** The owner's view of what visitors have left, with a way to remove it. */
export function CommentsDialog({ cv, onClose }: { cv: Cv; onClose: () => void }) {
  const cvId = cv.id
  const queryClient = useQueryClient()
  // Comes back as the full `Cv`, like publishing does, so it goes straight
  // into the editor's cache rather than being refetched.
  const toggle = useMutation({
    mutationFn: (enabled: boolean) => api.setCommentsEnabled(cvId, enabled),
    onSuccess: (updated) => queryClient.setQueryData(['cv', cvId], updated),
  })
  const key = ['cv-comments', cvId]
  const { data: comments, isLoading } = useQuery({
    queryKey: key,
    queryFn: () => api.listCvComments(cvId),
  })

  const remove = useMutation({
    mutationFn: (comment: CvComment) => api.deleteCvComment(cvId, comment.id),
    onSuccess: (_, comment) =>
      queryClient.setQueryData<CvComment[]>(key, (old = []) =>
        old.filter((c) => c.id !== comment.id),
      ),
  })

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  return createPortal(
    <div
      className="scrim"
      role="dialog"
      aria-modal="true"
      aria-label="Comments"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose()
      }}
    >
      <div className="dialog-card comments-dialog">
        <h2>Comments</h2>
        <label className="comments-toggle">
          <input
            type="checkbox"
            checked={cv.commentsEnabled}
            disabled={toggle.isPending}
            onChange={(e) => toggle.mutate(e.target.checked)}
          />
          <span>Let visitors to the share link send you comments</span>
        </label>
        {cv.commentsEnabled ? null : (
          <p className="muted small">
            Comments are off: visitors cannot send new ones. These are kept.
          </p>
        )}
        {isLoading ? (
          <p className="muted">Loading…</p>
        ) : (
          <CommentList comments={comments ?? []} onDelete={(c) => remove.mutate(c)} />
        )}
        {remove.isError || toggle.isError ? (
          <p className="auth-error" role="alert">
            {((remove.error ?? toggle.error) as Error).message}
          </p>
        ) : null}
        <div className="dialog-actions">
          <button type="button" className="primary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>,
    window.document.body,
  )
}
