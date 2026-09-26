import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Route, Routes } from 'react-router-dom'
import { QueryCache, QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { AuthGate } from './components/AuthGate'
import { Board } from './pages/Board'
import { Dashboard } from './pages/Dashboard'
import { Editor } from './pages/Editor'
import { ME, shouldRecheckSession } from './session'
import './styles.css'

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false, retry: 1 } },
  // A session can expire while the app is open, so a 401 from any *other*
  // query means the one the gate is holding is stale too. See
  // `shouldRecheckSession` for why the gate's own 401 must not count.
  queryCache: new QueryCache({
    onError: (error, query) => {
      if (shouldRecheckSession(error, query.queryKey)) {
        void queryClient.invalidateQueries({ queryKey: [ME] })
      }
    },
  }),
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AuthGate>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/cv/:id" element={<Editor />} />
            <Route path="/jobs" element={<Board />} />
          </Routes>
        </AuthGate>
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
)
