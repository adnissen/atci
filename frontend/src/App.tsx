import { Routes, Route, Navigate } from 'react-router-dom'
import './App.css'
import { useEffect, useState } from 'react'
import { useLSState } from './hooks/useLSState'
import HomePage from './pages/HomePage'
import ConfigPage from './pages/ConfigPage'
import ConfigSetup from './components/ConfigSetup'

function App() {
  // Configuration state
  const [configComplete, setConfigComplete] = useState<boolean | null>(null) // null = loading, false = incomplete, true = complete
  
  // Theme toggle state
  const [isDarkMode, setIsDarkMode] = useLSState('isDarkMode', true)
  
  // Apply dark theme on initial load
  useEffect(() => {
    const htmlElement = document.documentElement
    const bodyElement = document.body
    
    if (isDarkMode) {
      htmlElement.classList.add('dark')
      bodyElement.classList.add('dark')
    } else {
      htmlElement.classList.remove('dark')
      bodyElement.classList.remove('dark')
    }
  }, [isDarkMode])

  // Theme toggle function
  const toggleTheme = () => {
    const newIsDarkMode = !isDarkMode
    setIsDarkMode(newIsDarkMode)
    
    // Update the HTML element class
    const htmlElement = document.documentElement
    const bodyElement = document.body
    
    if (newIsDarkMode) {
      htmlElement.classList.add('dark')
      bodyElement.classList.add('dark')
    } else {
      htmlElement.classList.remove('dark')
      bodyElement.classList.remove('dark')
    }
  }

  // Check configuration status on app load
  useEffect(() => {
    const checkConfiguration = async () => {
      try {
        const response = await fetch('/config', {
          headers: {
            'Accept': 'application/json',
            'Content-Type': 'application/json'
          }
        })
        if (response.ok) {
          const data = await response.json()
          // Check if we have the minimum required configuration
          const hasWatchDirectory = data.config?.watch_directory || 
                                   (data.config?.watch_directories && data.config.watch_directories.length > 0)
          const hasWhisperCli = data.config?.whispercli_path
          const hasModel = data.config?.model_path || data.config?.model_name
          const hasFFmpeg = data.config?.ffmpeg_path
          const hasFFprobe = data.config?.ffprobe_path
          
          if (hasWatchDirectory && hasWhisperCli && hasModel && hasFFmpeg && hasFFprobe) {
            setConfigComplete(true)
          } else {
            setConfigComplete(false)
          }
        } else {
          setConfigComplete(false)
        }
      } catch (error) {
        console.error('Error checking configuration:', error)
        setConfigComplete(false)
      }
    }
    
    checkConfiguration()
  }, [])

  // Show loading while checking configuration
  if (configComplete === null) {
    return (
      <div className={`min-h-screen bg-background ${isDarkMode ? 'dark' : ''} flex items-center justify-center`}>
        <div className="text-center">
          <div className="text-lg text-muted-foreground">Loading...</div>
        </div>
        
        {/* Theme Toggle Button */}
        <button
          onClick={toggleTheme}
          className="fixed bottom-6 left-6 z-50 p-3 bg-card border border-border rounded-full shadow-lg hover:shadow-xl transition-all duration-200 hover:scale-105 text-foreground hover:bg-accent"
          title={isDarkMode ? 'Switch to light mode' : 'Switch to dark mode'}
        >
          {isDarkMode ? (
            // Sun icon for light mode
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
            </svg>
          ) : (
            // Moon icon for dark mode
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
            </svg>
          )}
        </button>
      </div>
    )
  }

  // Show configuration setup if not complete
  if (configComplete === false) {
    return (
      <>
        <ConfigSetup onConfigComplete={() => setConfigComplete(true)} />
        
        {/* Theme Toggle Button */}
        <button
          onClick={toggleTheme}
          className="fixed bottom-6 left-6 z-50 p-3 bg-card border border-border rounded-full shadow-lg hover:shadow-xl transition-all duration-200 hover:scale-105 text-foreground hover:bg-accent"
          title={isDarkMode ? 'Switch to light mode' : 'Switch to dark mode'}
        >
          {isDarkMode ? (
            // Sun icon for light mode
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
            </svg>
          ) : (
            // Moon icon for dark mode
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
            </svg>
          )}
        </button>
      </>
    )
  }

  return (
    <div className={`min-h-screen bg-background ${isDarkMode ? 'dark' : ''}`}>
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/config" element={<ConfigPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>

      {/* Theme Toggle Button */}
      <button
        onClick={toggleTheme}
        className="fixed bottom-6 left-6 z-50 p-3 bg-card border border-border rounded-full shadow-lg hover:shadow-xl transition-all duration-200 hover:scale-105 text-foreground hover:bg-accent"
        title={isDarkMode ? 'Switch to light mode' : 'Switch to dark mode'}
      >
        {isDarkMode ? (
          // Sun icon for light mode
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
          </svg>
        ) : (
          // Moon icon for dark mode
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
          </svg>
        )}
      </button>
    </div>
  )
}

export default App
