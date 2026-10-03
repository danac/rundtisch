import { Outlet } from 'react-router-dom'

export function Layout() {
  return (
    <div className="min-h-dvh bg-void text-ink">
      <Outlet />
    </div>
  )
}
