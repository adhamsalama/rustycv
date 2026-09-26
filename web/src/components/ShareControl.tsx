import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import type { Cv } from '../types'

/**
 * The publish toggle plus the link once it exists.
 *
 * Publishing and unpublishing both come back with the full `Cv`, so the
 * mutation writes it straight into the `['cv', id]` cache rather than
 * refetching — the same document the editor already has, just with a newer
 * `publicId`/`published`.
 */
export function ShareControl({ cv }: { cv: Cv }) {
  const queryClient = useQueryClient()
  const [copied, setCopied] = useState(false)

  const onSettled = (updated: Cv | undefined) => {
    if (updated) queryClient.setQueryData(['cv', cv.id], updated)
  }
  const publish = useMutation({ mutationFn: () => api.publishCv(cv.id), onSuccess: onSettled })
  const unpublish = useMutation({ mutationFn: () => api.unpublishCv(cv.id), onSuccess: onSettled })

  const pending = publish.isPending || unpublish.isPending

  const copyLink = async () => {
    if (!cv.publicId) return
    await navigator.clipboard.writeText(api.shareUrl(cv.publicId))
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  return (
    <div className="share-control">
      {cv.published && cv.publicId ? (
        <>
          <a
            className="share-link"
            href={api.shareUrl(cv.publicId)}
            target="_blank"
            rel="noreferrer"
          >
            {api.shareUrl(cv.publicId)}
          </a>
          <button type="button" className="ghost" onClick={copyLink}>
            {copied ? 'Copied' : 'Copy link'}
          </button>
          <button
            type="button"
            className="ghost"
            disabled={pending}
            onClick={() => unpublish.mutate()}
          >
            Unpublish
          </button>
        </>
      ) : (
        <button type="button" className="ghost" disabled={pending} onClick={() => publish.mutate()}>
          Publish
        </button>
      )}
    </div>
  )
}
