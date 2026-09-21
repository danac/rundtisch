import { useEffect, useState, type FormEvent } from 'react'

type Role = 'User' | 'Admin'

type User = {
  id: number
  email: string
  alias: string
  role: Role
}

type ListResponse = { result: User[] }
type ErrorResponse = { error: string }

const panelClassName =
  'w-full rounded-2xl border border-ring/70 bg-paper/80 px-6 py-6 text-left shadow-[0_12px_40px_rgba(44,36,22,0.08)] backdrop-blur-[2px]'
const inputClassName =
  'mt-1.5 block w-full rounded-lg border border-ring/80 bg-paper px-3 py-2 text-ink outline-none transition focus:border-ink/40 focus:ring-2 focus:ring-ring/60'
const compactInputClassName =
  'block w-full min-w-0 rounded-lg border border-ring/80 bg-paper px-2.5 py-1.5 text-sm text-ink outline-none transition focus:border-ink/40 focus:ring-2 focus:ring-ring/60'
const primaryButtonClassName =
  'rounded-full bg-ink px-4 py-2.5 text-sm font-semibold tracking-wide text-paper transition hover:bg-ink/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50'
const secondaryButtonClassName =
  'rounded-full border border-ring/80 bg-paper px-3 py-1.5 text-sm font-semibold tracking-wide text-ink transition hover:bg-ring/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:opacity-50'

async function readError(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as ErrorResponse
    if (body.error) return body.error
  } catch {
    // ignore parse errors
  }
  return `Request failed (${response.status})`
}

export function UsersPanel() {
  const [users, setUsers] = useState<User[]>([])
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [email, setEmail] = useState('')
  const [alias, setAlias] = useState('')
  const [role, setRole] = useState<Role>('User')
  const [editDrafts, setEditDrafts] = useState<Record<number, string>>({})

  async function loadUsers() {
    setLoading(true)
    setError(null)
    try {
      const response = await fetch('/api/auth/users')
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      const body = (await response.json()) as ListResponse
      setUsers(body.result)
      setEditDrafts(Object.fromEntries(body.result.map((user) => [user.id, user.alias])))
    } catch {
      setError('Network error')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadUsers()
  }, [])

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setBusy(true)
    setError(null)
    try {
      const response = await fetch('/api/auth/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, alias, role }),
      })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      setEmail('')
      setAlias('')
      setRole('User')
      await loadUsers()
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  async function handleSaveAlias(id: number) {
    const nextAlias = editDrafts[id] ?? ''
    setBusy(true)
    setError(null)
    try {
      const response = await fetch(`/api/auth/users/${id}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ alias: nextAlias }),
      })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      await loadUsers()
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  async function handleDelete(id: number) {
    setBusy(true)
    setError(null)
    try {
      const response = await fetch(`/api/auth/users/${id}`, { method: 'DELETE' })
      if (!response.ok) {
        setError(await readError(response))
        return
      }
      await loadUsers()
    } catch {
      setError('Network error')
    } finally {
      setBusy(false)
    }
  }

  const actionsDisabled = loading || busy

  return (
    <section className={panelClassName} aria-labelledby="users-heading">
      <div className="flex items-center justify-between gap-3">
        <h2 id="users-heading" className="text-sm font-semibold tracking-wide text-ink">
          Users
        </h2>
        <button
          type="button"
          className={secondaryButtonClassName}
          onClick={() => void loadUsers()}
          disabled={actionsDisabled}
        >
          Refresh
        </button>
      </div>

      {error ? (
        <p className="mt-3 text-sm text-ink" role="alert">
          {error}
        </p>
      ) : null}

      <div className="mt-4">
        {loading ? (
          <p className="text-sm text-ink-muted">Loading…</p>
        ) : users.length === 0 ? (
          <p className="text-sm text-ink-muted">No users yet.</p>
        ) : (
          <ul className="divide-y divide-ring/50">
            {users.map((user) => {
              const draft = editDrafts[user.id] ?? user.alias
              const dirty = draft.trim() !== user.alias
              return (
                <li key={user.id} className="flex flex-col gap-2 py-3 first:pt-0 last:pb-0">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0 flex-1">
                      <label className="block text-xs text-ink-muted">
                        Alias
                        <input
                          type="text"
                          value={draft}
                          onChange={(event) =>
                            setEditDrafts((prev) => ({ ...prev, [user.id]: event.target.value }))
                          }
                          className={`mt-1 ${compactInputClassName}`}
                          disabled={actionsDisabled}
                          aria-label={`Alias for ${user.email}`}
                        />
                      </label>
                      <p className="mt-1.5 truncate text-sm text-ink-muted">{user.email}</p>
                      <p className="mt-0.5 text-xs tracking-wide text-ink-muted">{user.role}</p>
                    </div>
                    <div className="flex shrink-0 flex-col gap-2">
                      <button
                        type="button"
                        className={secondaryButtonClassName}
                        onClick={() => void handleSaveAlias(user.id)}
                        disabled={actionsDisabled || !dirty}
                      >
                        Save
                      </button>
                      <button
                        type="button"
                        className={secondaryButtonClassName}
                        onClick={() => void handleDelete(user.id)}
                        disabled={actionsDisabled}
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                </li>
              )
            })}
          </ul>
        )}
      </div>

      <form onSubmit={handleCreate} className="mt-6 border-t border-ring/50 pt-5">
        <h3 className="text-sm font-semibold tracking-wide text-ink">Add user</h3>
        <div className="mt-4 space-y-4">
          <label className="block text-sm text-ink-muted">
            Email
            <input
              type="email"
              name="email"
              required
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              className={inputClassName}
              disabled={actionsDisabled}
            />
          </label>
          <label className="block text-sm text-ink-muted">
            Alias
            <input
              type="text"
              name="alias"
              required
              value={alias}
              onChange={(event) => setAlias(event.target.value)}
              className={inputClassName}
              disabled={actionsDisabled}
            />
          </label>
          <label className="block text-sm text-ink-muted">
            Role
            <select
              name="role"
              value={role}
              onChange={(event) => setRole(event.target.value as Role)}
              className={inputClassName}
              disabled={actionsDisabled}
            >
              <option value="User">User</option>
              <option value="Admin">Admin</option>
            </select>
          </label>
        </div>
        <button type="submit" className={`mt-6 w-full ${primaryButtonClassName}`} disabled={actionsDisabled}>
          Add user
        </button>
      </form>
    </section>
  )
}
