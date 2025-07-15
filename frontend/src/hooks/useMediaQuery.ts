import { useState, useEffect } from 'react'

export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState<boolean>(
    () => window.matchMedia(query).matches
  )

  useEffect(() => {
    const mediaQuery = window.matchMedia(query)
    
    // Update state when media query changes
    const handleChange = (event: MediaQueryListEvent) => {
      setMatches(event.matches)
    }

    // Add listener
    mediaQuery.addEventListener('change', handleChange)

    // Set initial value
    setMatches(mediaQuery.matches)

    // Cleanup
    return () => {
      mediaQuery.removeEventListener('change', handleChange)
    }
  }, [query])

  return matches
}

// Convenience hooks for common breakpoints
export function useIsSmallScreen(): boolean {
  // Tailwind's default sm breakpoint is 640px
  return !useMediaQuery('(min-width: 640px)')
}

export function useIsMediumScreen(): boolean {
  // Tailwind's default md breakpoint is 768px
  return useMediaQuery('(min-width: 768px)')
}

export function useIsLargeScreen(): boolean {
  // Tailwind's default lg breakpoint is 1024px
  return useMediaQuery('(min-width: 1024px)')
}