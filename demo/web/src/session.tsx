import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from 'react'

export type AuthUser = {
  public_id: string
  email: string
  alias: string
  role: string
  created_at: string
  has_password: boolean
}

type SessionStatus = 'checking' | 'anonymous' | 'authenticated'

type SessionContextValue = {
  status: SessionStatus
  user: AuthUser | null
  establish: (user: AuthUser) => void
  logout: () => Promise<void>
  logoutAll: () => Promise<void>
  api: (path: string, init?: RequestInit) => Promise<Response>
}

const SessionContext = createContext<SessionContextValue | null>(null)

export function SessionProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<SessionStatus>('checking')
  const [user, setUser] = useState<AuthUser | null>(null)
  const epochRef = useRef(0)

  function clearSession() {
    epochRef.current += 1
    setUser(null)
    setStatus('anonymous')
  }

  function establish(nextUser: AuthUser) {
    epochRef.current += 1
    setUser(nextUser)
    setStatus('authenticated')
  }

  async function api(path: string, init: RequestInit = {}): Promise<Response> {
    const headers = new Headers(init.headers)
    if (init.body && !headers.has('Content-Type')) {
      headers.set('Content-Type', 'application/json')
    }
    const generation = epochRef.current
    const response = await fetch(path, { credentials: 'include', ...init, headers })
    if (response.status === 401 && generation === epochRef.current) {
      clearSession()
    }
    return response
  }

  async function logout() {
    await fetch('/api/auth/logout', { method: 'POST', credentials: 'include' })
    clearSession()
  }

  async function logoutAll() {
    const response = await api('/api/auth/logout_all', { method: 'POST' })
    if (response.ok) clearSession()
  }

  useEffect(() => {
    const generation = epochRef.current
    void (async () => {
      try {
        const response = await fetch('/api/auth/me', { credentials: 'include' })
        if (generation !== epochRef.current) return
        if (!response.ok) {
          setStatus('anonymous')
          return
        }
        setUser((await response.json()) as AuthUser)
        setStatus('authenticated')
      } catch {
        if (generation === epochRef.current) setStatus('anonymous')
      }
    })()
  }, [])

  return (
    <SessionContext.Provider value={{ status, user, establish, logout, logoutAll, api }}>
      {children}
    </SessionContext.Provider>
  )
}

export function useSession(): SessionContextValue {
  const value = useContext(SessionContext)
  if (!value) throw new Error('useSession must be used within SessionProvider')
  return value
}
