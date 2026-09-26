import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react'

export type AuthUser = {
  public_id: string
  email: string
  alias: string
  role: string
}

type SessionStatus = 'checking' | 'anonymous' | 'authenticated'

type SessionContextValue = {
  status: SessionStatus
  accessToken: string | null
  user: AuthUser | null
  establish: (accessToken: string, user: AuthUser) => void
  logout: () => Promise<void>
  api: (path: string, init?: RequestInit) => Promise<Response>
}

const SessionContext = createContext<SessionContextValue | null>(null)

const PUBLIC_PATHS = new Set(['/api/auth/register', '/api/auth/activate', '/api/auth/login'])

type RefreshResult = { accessToken: string; user: AuthUser } | null

let refreshInflight: Promise<RefreshResult> | null = null

function requestRefresh(): Promise<RefreshResult> {
  if (!refreshInflight) {
    refreshInflight = (async () => {
      const response = await fetch('/api/auth/refresh', {
        method: 'POST',
        credentials: 'include',
      })
      if (!response.ok) return null
      const body = (await response.json()) as { access_token: string; user: AuthUser }
      return { accessToken: body.access_token, user: body.user }
    })().finally(() => {
      refreshInflight = null
    })
  }
  return refreshInflight
}

function decodeBase64Url(value: string): string {
  const padded = value.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - (value.length % 4)) % 4)
  const bytes = Uint8Array.from(atob(padded), (char) => char.charCodeAt(0))
  return new TextDecoder().decode(bytes)
}

export function decodeJwt(token: string): { header: unknown; payload: unknown } {
  const [header, payload] = token.split('.')
  if (!header || !payload) {
    throw new Error('malformed jwt')
  }
  return {
    header: JSON.parse(decodeBase64Url(header)),
    payload: JSON.parse(decodeBase64Url(payload)),
  }
}

function accessTokenExpired(token: string): boolean {
  try {
    const { payload } = decodeJwt(token)
    if (!payload || typeof payload !== 'object' || !('exp' in payload)) return true
    const exp = (payload as { exp?: unknown }).exp
    return typeof exp !== 'number' || exp * 1000 <= Date.now()
  } catch {
    return true
  }
}

export function SessionProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<SessionStatus>('checking')
  const [accessToken, setAccessToken] = useState<string | null>(null)
  const [user, setUser] = useState<AuthUser | null>(null)
  const tokenRef = useRef<string | null>(null)
  const epochRef = useRef(0)

  function clearSession() {
    epochRef.current += 1
    tokenRef.current = null
    setAccessToken(null)
    setUser(null)
    setStatus('anonymous')
  }

  function establish(nextToken: string, nextUser: AuthUser) {
    tokenRef.current = nextToken
    setAccessToken(nextToken)
    setUser(nextUser)
    setStatus('authenticated')
  }

  async function refreshAccess(): Promise<string | null> {
    const generation = epochRef.current
    const result = await requestRefresh()
    if (generation !== epochRef.current) return tokenRef.current
    if (!result) {
      tokenRef.current = null
      setAccessToken(null)
      setUser(null)
      setStatus('anonymous')
      return null
    }
    tokenRef.current = result.accessToken
    setAccessToken(result.accessToken)
    setUser(result.user)
    setStatus('authenticated')
    return result.accessToken
  }

  async function api(path: string, init: RequestInit = {}): Promise<Response> {
    const headers = new Headers(init.headers)
    if (init.body && !headers.has('Content-Type')) {
      headers.set('Content-Type', 'application/json')
    }
    const anonymousCall = PUBLIC_PATHS.has(path)
    if (!anonymousCall) {
      let token = tokenRef.current
      if (!token || accessTokenExpired(token)) {
        token = await refreshAccess()
        if (!token) {
          return new Response(JSON.stringify({ error: 'invalid_token' }), {
            status: 401,
            headers: { 'Content-Type': 'application/json' },
          })
        }
      }
      headers.set('Authorization', `Bearer ${token}`)
    }
    let response = await fetch(path, { credentials: 'include', ...init, headers })
    if (!anonymousCall && response.status === 401) {
      const token = await refreshAccess()
      if (!token) return response
      headers.set('Authorization', `Bearer ${token}`)
      response = await fetch(path, { credentials: 'include', ...init, headers })
      if (response.status === 401) clearSession()
    }
    return response
  }

  async function logout() {
    clearSession()
    await fetch('/api/auth/logout', { method: 'POST', credentials: 'include' })
  }

  useEffect(() => {
    void refreshAccess()
  }, [])

  return (
    <SessionContext.Provider value={{ status, accessToken, user, establish, logout, api }}>
      {children}
    </SessionContext.Provider>
  )
}

export function useSession(): SessionContextValue {
  const value = useContext(SessionContext)
  if (!value) throw new Error('useSession must be used within SessionProvider')
  return value
}
