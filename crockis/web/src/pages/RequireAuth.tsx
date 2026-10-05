import { Navigate, Outlet, useLocation } from 'react-router-dom'
import { useAuth } from '../auth/useAuth'

export function RequireAuth() {
  const { status, user } = useAuth()
  const location = useLocation()

  if (status === 'checking') {
    return (
      <p className="px-6 pt-16 text-center text-sm text-mist">Checking session…</p>
    )
  }

  if (!user) {
    return <Navigate to="/login" replace state={{ from: location.pathname }} />
  }

  return <Outlet />
}
