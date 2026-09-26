import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import { ME } from '../session'

/**
 * Who you are, and the way out. Sits in the header of both top-level pages.
 *
 * Signing out clears the whole cache, not just the session: the CVs and the
 * board in it belong to the account that is leaving.
 */
export function AccountMenu() {
  const queryClient = useQueryClient()
  const { data: user } = useQuery({ queryKey: [ME], queryFn: api.me })

  const signOut = useMutation({
    mutationFn: api.logout,
    onSuccess: () => queryClient.clear(),
  })

  if (!user) return null

  return (
    <div className="account-menu">
      <span className="muted small account-email" title={user.email}>
        {user.email}
      </span>
      <button
        type="button"
        className="ghost"
        disabled={signOut.isPending}
        onClick={() => signOut.mutate()}
      >
        Sign out
      </button>
    </div>
  )
}
