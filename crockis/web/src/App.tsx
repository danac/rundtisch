import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { Layout } from './components/Layout'
import { CollectionPage } from './pages/CollectionPage'
import { CollectionsPage } from './pages/CollectionsPage'
import { LoginPage } from './pages/LoginPage'
import { RequireAuth } from './pages/RequireAuth'

export function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/login" element={<LoginPage />} />
          <Route element={<RequireAuth />}>
            <Route path="/collections" element={<CollectionsPage />} />
            <Route path="/collections/:collectionId" element={<CollectionPage />} />
          </Route>
          <Route path="/" element={<Navigate to="/collections" replace />} />
          <Route path="*" element={<Navigate to="/collections" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}
