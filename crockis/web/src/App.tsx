import { BrowserRouter, Navigate, Route, Routes, useLocation } from 'react-router-dom'
import { Layout } from './components/Layout'
import { AuthorizeDonePage, AuthorizePage } from './pages/AuthorizePage'
import { CollectionPage } from './pages/CollectionPage'
import { CollectionsPage } from './pages/CollectionsPage'
import { LoginPage } from './pages/LoginPage'
import { RequireAuth } from './pages/RequireAuth'
import { SettingsPage } from './pages/SettingsPage'

function HomeRedirect() {
  const location = useLocation()
  const params = new URLSearchParams(location.search)
  if (params.has('invite') || params.has('recover')) {
    return <Navigate to={{ pathname: '/login', search: location.search }} replace />
  }
  return <Navigate to="/collections" replace />
}

export function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/authorize" element={<AuthorizePage />} />
          <Route path="/authorize/done" element={<AuthorizeDonePage />} />
          <Route element={<RequireAuth />}>
            <Route path="/collections" element={<CollectionsPage />} />
            <Route path="/collections/:collectionId" element={<CollectionPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Route>
          <Route path="/" element={<HomeRedirect />} />
          <Route path="*" element={<HomeRedirect />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}
