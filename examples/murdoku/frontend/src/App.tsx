import { useState } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { NavBar } from './components/NavBar';
import { HomePage } from './pages/HomePage';
import { PlayPage } from './pages/PlayPage';
import { CreatePage } from './pages/CreatePage';
import type { WalletState } from './types';

function App() {
  const [wallet, setWallet] = useState<WalletState>({ connected: false, address: null });

  function handleConnect() {
    setWallet({ connected: true, address: 'GBETG3K7QW4XNZPFVMMJJ5ZRQG2YDKN' });
  }

  function handleDisconnect() {
    setWallet({ connected: false, address: null });
  }

  return (
    <BrowserRouter>
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>

      <NavBar wallet={wallet} onConnect={handleConnect} onDisconnect={handleDisconnect} />

      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route
          path="/play/:puzzleId"
          element={<PlayPage walletConnected={wallet.connected} />}
        />
        <Route path="/create" element={<CreatePage />} />
        <Route path="*" element={
          <main id="main-content" style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '2rem', textAlign: 'center' }}>
            <div>
              <h1 style={{ fontFamily: 'var(--font-serif)', fontSize: '2rem', color: 'var(--accent-red)', marginBottom: '0.75rem' }}>404</h1>
              <p style={{ fontFamily: 'var(--font-serif)', fontStyle: 'italic', color: 'var(--noir-muted)' }}>
                This page vanished — like the alibi of a guilty man.
              </p>
            </div>
          </main>
        } />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
