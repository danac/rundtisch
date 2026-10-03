import { useCallback, useMemo, useState, type ReactNode } from 'react'
import { api, type Session } from '../api'
import { AuthContext, type AuthContextValue } from './context'
import { clearSession, readSession, writeSession } from './session'

export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<Session | null>(() => readSession())

  const login = useCallback(async (email: string, password: string) => {
    const next = await api.login({ email, password })
    writeSession(next)
    setSession(next)
  }, [])

  const logout = useCallback(async () => {
    if (session) {
      await api.logout(session.token)
    }
    clearSession()
    setSession(null)
  }, [session])

  const value = useMemo<AuthContextValue>(
    () => ({
      user: session?.user ?? null,
      token: session?.token ?? null,
      login,
      logout,
    }),
    [login, logout, session],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
