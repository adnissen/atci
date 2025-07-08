import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from './components/ui/table'
import './App.css'
import TranscriptView from './components/TranscriptView'
import ConfigSetup from './components/ConfigSetup'
import EditDialog from './components/EditDialog'
import { useEffect, useState, useRef, useCallback } from 'react'
import { useLSState } from './hooks/useLSState'

function App() {
  // Sample data for the table
  type FileRow = {
    name: string
    base_name: string
    created_at: string
    transcript: boolean
    line_count?: number
    length?: string
    full_path?: string
    last_generated?: string
    model?: string
  }
  
  type TranscriptData = {
    text: string
    loading: boolean
    error: string | null
  }
  
  type SortColumn = 'created_at' | 'last_generated' | 'name' | 'line_count' | 'length' | 'model'
  type SortDirection = 'asc' | 'desc'
  
  // Configuration state
  const [configComplete, setConfigComplete] = useState<boolean | null>(null) // null = loading, false = incomplete, true = complete
  const [isConfigEditorOpen, setIsConfigEditorOpen] = useState(false)
  
  const [files, setFiles] = useState<FileRow[]>(window.autotranscript_files as FileRow[])
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set())
  const [searchTerm, setSearchTerm] = useState('')
  const [searchLineNumbers, setSearchLineNumbers] = useState<Record<string, number[]>>({})
  const [isSearching, setIsSearching] = useState(false)
  const [regeneratingFiles, setRegeneratingFiles] = useState<Set<string>>(new Set())
  type QueueItem = {
    video_path: string
    process_type: string
  }
  
  const [queue, setQueue] = useState<QueueItem[]>([])
  const [currentProcessingFile, setCurrentProcessingFile] = useState<QueueItem | null>(null)
  const [watchDirectory, setWatchDirectory] = useState<string>('')
  const [replacingFiles, setReplacingFiles] = useState<Set<string>>(new Set())
  const [transcriptData, setTranscriptData] = useState<Record<string, TranscriptData>>({})
  const [sortColumn, setSortColumn] = useLSState<SortColumn>('sortColumn', 'created_at')
  const [sortDirection, setSortDirection] = useLSState<SortDirection>('sortDirection', 'desc')
  
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
  }, []) // Empty dependency array means this runs once on mount
  
  // Replace transcript dialog state
  const [isReplaceDialogOpen, setIsReplaceDialogOpen] = useState(false)
  const [replaceTranscriptFilename, setReplaceTranscriptFilename] = useState('')
  const [replaceTranscriptInitialContent, setReplaceTranscriptInitialContent] = useState('')
  const [isReplacingTranscript, setIsReplacingTranscript] = useState(false)
  
  // State for tracking out-of-view expanded rows
  const [outOfViewExpandedFile, setOutOfViewExpandedFile] = useState<string | null>(null)
  const [flashingRow, setFlashingRow] = useState<string | null>(null)
  const fileRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const transcriptRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const observerRef = useRef<IntersectionObserver | null>(null)
  const transcriptObserverRef = useRef<IntersectionObserver | null>(null)

  // Helper function to check if a file is currently being processed
  const isFileBeingProcessed = (filename: string): boolean => {
    if (!currentProcessingFile) return false
    const currentFileName = currentProcessingFile.video_path.split('/').pop()?.replace(/\.(mp4|MP4)$/, '')
    return currentFileName === filename
  }

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

  // Calculate search results from search line numbers
  const searchResults = Object.keys(searchLineNumbers).filter(filename => 
    searchLineNumbers[filename] && (searchLineNumbers[filename].length > 0)
  )

  // Setup intersection observer to track expanded rows visibility
  const setupIntersectionObserver = useCallback(() => {
    // Clean up existing observers
    if (observerRef.current) {
      observerRef.current.disconnect()
    }
    if (transcriptObserverRef.current) {
      transcriptObserverRef.current.disconnect()
    }

    // Observer for file rows (sets single file when row goes off top)
    observerRef.current = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          const filename = entry.target.getAttribute('data-filename')
          if (filename && expandedFiles.has(filename)) {
            if (!entry.isIntersecting) {
              // Check if the element is above the viewport (off the top)
              const isAboveViewport = entry.boundingClientRect.bottom < (entry.rootBounds?.top || 0)
              if (isAboveViewport && !outOfViewExpandedFile) {
                // Only set as out of view if transcript is still visible
                const transcriptRow = transcriptRowRefs.current[filename]
                if (transcriptRow) {
                  const transcriptRect = transcriptRow.getBoundingClientRect()
                  const isTranscriptAboveViewport = transcriptRect.bottom < (entry.rootBounds?.top || 0)
                  if (!isTranscriptAboveViewport) {
                    setOutOfViewExpandedFile(filename)
                  }
                }
              }
            } else {
              // Remove if this file is currently the out-of-view file
              if (outOfViewExpandedFile === filename) {
                setOutOfViewExpandedFile(null)
              }
            }
          }
        })
      },
      {
        root: null,
        rootMargin: '-80px 0px 0px 0px', // Account for top bar height
        threshold: 0
      }
    )

    transcriptObserverRef.current = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          const filename = entry.target.getAttribute('data-filename')
          if (filename && expandedFiles.has(filename)) {
            if (!entry.isIntersecting) {
              // Check if the bottom of the transcript is above the viewport
              const isBottomAboveViewport = entry.boundingClientRect.bottom < (entry.rootBounds?.top || 0)
              if (isBottomAboveViewport && outOfViewExpandedFile === filename) {
                setOutOfViewExpandedFile(null)
              } 
            } else {
              // Check if the file row is above the viewport but transcript is visible
              const fileRow = fileRowRefs.current[filename]
              if (fileRow) {
                const fileRowRect = fileRow.getBoundingClientRect()
                const isFileRowAboveViewport = fileRowRect.bottom < (entry.rootBounds?.top || 0)
                if (isFileRowAboveViewport && !outOfViewExpandedFile) {
                  setOutOfViewExpandedFile(filename)
                }
              }
            }
          }
        })
      },
      {
        root: null,
        rootMargin: '-80px 0px 0px 0px', // Account for top bar height
        threshold: 0
      }
    )

    // Observe all expanded file rows
    expandedFiles.forEach(filename => {
      const rowElement = fileRowRefs.current[filename]
      if (rowElement && observerRef.current) {
        observerRef.current.observe(rowElement)
      }
      
      const transcriptRowElement = transcriptRowRefs.current[filename]
      if (transcriptRowElement && transcriptObserverRef.current) {
        transcriptObserverRef.current.observe(transcriptRowElement)
      }
    })
  }, [expandedFiles, outOfViewExpandedFile])

  // Handle scroll to top
  const handleScrollToTop = () => {
    if (outOfViewExpandedFile) {
      const topBarHeight = watchDirectory ? 64 : 0 // Approximate height of top bar
      const elementTop = fileRowRefs.current[outOfViewExpandedFile]?.offsetTop
      const scrollTop = elementTop ? elementTop - topBarHeight : 0
      window.scrollTo({ top: scrollTop, behavior: 'smooth' })
    }
  }

  // Handle collapse
  const handleCollapseExpanded = () => {
    if (outOfViewExpandedFile) {
      const targetFile = outOfViewExpandedFile
      
      // Collapse the row
      setExpandedFiles(prev => {
        const newSet = new Set(prev)
        newSet.delete(targetFile)
        return newSet
      })
      
      // Clear the out-of-view file
      setOutOfViewExpandedFile(null)
      
      // Scroll to the row and flash it
      setTimeout(() => {
        const rowElement = fileRowRefs.current[targetFile]
        if (rowElement) {
          const topBarHeight = watchDirectory ? 64 : 0
          const elementTop = rowElement.offsetTop
          const scrollTop = elementTop - topBarHeight
          window.scrollTo({ top: scrollTop, behavior: 'smooth' })
          
          // Flash the row
          setFlashingRow(targetFile)
          setTimeout(() => setFlashingRow(null), 1000) // Flash for 1 second
        }
      }, 100) // Small delay to ensure DOM updates
    }
  }

  // Sort files based on current sort column and direction
  const sortedFiles = [...files].sort((a, b) => {
    let aValue: any
    let bValue: any
    
    switch (sortColumn) {
      case 'created_at':
        aValue = a.created_at || ''
        bValue = b.created_at || ''
        break
      case 'last_generated':
        aValue = a.last_generated || ''
        bValue = b.last_generated || ''
        break
      case 'name':
        aValue = a.name || ''
        bValue = b.name || ''
        break
      case 'line_count':
        aValue = a.line_count || 0
        bValue = b.line_count || 0
        break
      case 'length':
        aValue = a.length || '0:00'
        bValue = b.length || '0:00'
        break
      case 'model':
        aValue = a.model || ''
        bValue = b.model || ''
        break
      default:
        return 0
    }
    
    // Handle date sorting
    if (sortColumn === 'created_at' || sortColumn === 'last_generated') {
      const dateA = new Date(aValue.replace(' ', 'T')).getTime()
      const dateB = new Date(bValue.replace(' ', 'T')).getTime()
      
      if (sortDirection === 'asc') {
        return dateA - dateB
      } else {
        return dateB - dateA
      }
    }
    
    // Handle string sorting
    if (typeof aValue === 'string' && typeof bValue === 'string') {
      if (sortDirection === 'asc') {
        return aValue.localeCompare(bValue)
      } else {
        return bValue.localeCompare(aValue)
      }
    }
    
    // Handle number sorting
    if (sortDirection === 'asc') {
      return aValue - bValue
    } else {
      return bValue - aValue
    }
  })

  const expandContext = (filename: string, direction: "up" | "down", line: number) => {
    // Get the line numbers for the file
    const fileLineNumbers = searchLineNumbers[filename]
    if (!fileLineNumbers || !fileLineNumbers.includes(line)) {
      return
    }

    const newLineNumbers = [...fileLineNumbers]

    if (direction === "up") {
      // Add 5 descending line numbers
      for (let i = 1; i <= 16; i++) {
        const prevLine = line - i
        if (prevLine > 0 && !newLineNumbers.includes(prevLine)) {
          newLineNumbers.push(prevLine)
        }
      }
    } else {
      // Add 5 ascending line numbers
      for (let i = 1; i <= 16; i++) {
        const nextLine = line + i
        if (!newLineNumbers.includes(nextLine)) {
          newLineNumbers.push(nextLine)
        }
      }
    }

    // Update the search line numbers
    setSearchLineNumbers(prev => ({
      ...prev,
      [filename]: newLineNumbers
    }))
  }

  const expandAll = (filename: string) => {
    // [-1] and [] are slightly different:
    // [] + a search term means there were no results at all in the file, so don't display the transcript at all
    // [-1] + a search term means there were results, and now we want to show the whole file
    setSearchLineNumbers(prev => ({
      ...prev,
      [filename]: [-1]
    }))
  }

  // Handle sort column click
  const handleSort = (column: SortColumn) => {
    if (sortColumn === column) {
      // Toggle direction if same column
      setSortDirection(prev => prev === 'asc' ? 'desc' : 'asc')
    } else {
      // Set new column with default direction
      setSortColumn(column)
      setSortDirection('asc')
    }
  }

  // Get sort indicator for header
  const getSortIndicator = (column: SortColumn) => {
    if (sortColumn !== column) {
      return (
        <svg className="w-4 h-4 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4" />
        </svg>
      )
    }
    
    if (sortDirection === 'asc') {
      return (
        <svg className="w-4 h-4 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
        </svg>
      )
    } else {
      return (
        <svg className="w-4 h-4 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      )
    }
  }

  // Get color for model chip based on model name
  const getModelChipColor = (model: string | undefined) => {
    if (!model) return 'bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-300'
    
    // Color palette for different model types
    const modelColors: Record<string, string> = {
      'tiny': 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
      'base': 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
      'small': 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
      'medium': 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200',
      'large': 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
    }
    
    // Determine model type from name
    for (const [key, color] of Object.entries(modelColors)) {
      if (model.includes(key)) {
        return color
      }
    }
    
    // Default color if no match
    return 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200'
  }

  // Function to format date from YYYY-MM-DD HH:MM:SS to MM-DD-YYYY x:xxpm
  const formatDate = (dateString: string): string => {
    if (!dateString || dateString === 'N/A') return 'N/A'
    
    try {
      const date = new Date(dateString.replace(' ', 'T'))
      if (isNaN(date.getTime())) return 'N/A'
      
      const month = (date.getMonth() + 1).toString().padStart(2, '0')
      const day = date.getDate().toString().padStart(2, '0')
      const year = date.getFullYear()
      
      let hours = date.getHours()
      const minutes = date.getMinutes().toString().padStart(2, '0')
      const ampm = hours >= 12 ? 'pm' : 'am'
      hours = hours % 12
      hours = hours ? hours : 12 // the hour '0' should be '12'
      
      return `${month}-${day}-${year} ${hours}:${minutes}${ampm}`
    } catch (error) {
      return 'N/A'
    }
  }

  // Fetch transcript for a specific file
  const fetchTranscript = async (filename: string) => {
    // Set loading state
    setTranscriptData(prev => ({
      ...prev,
      [filename]: { text: '', loading: true, error: null }
    }))

    try {
      const response = await fetch(`/transcripts/${encodeURIComponent(filename)}`)
      
      if (!response.ok) {
        throw new Error(`Failed to fetch transcript: ${response.status} ${response.statusText}`)
      }
      
      const transcriptContent = await response.text()
      
      // Set success state
      setTranscriptData(prev => ({
        ...prev,
        [filename]: { text: transcriptContent, loading: false, error: null }
      }))
    } catch (err) {
      // Set error state
      setTranscriptData(prev => ({
        ...prev,
        [filename]: { 
          text: '', 
          loading: false, 
          error: err instanceof Error ? err.message : 'An unknown error occurred' 
        }
      }))
    }
  }

  // Fetch transcripts for all expanded files
  const fetchExpandedTranscripts = async () => {
    const expandedArray = Array.from(expandedFiles)
    
    // Only fetch for files that don't already have data or are not currently loading
    const filesToFetch = expandedArray.filter(filename => {
      const currentData = transcriptData[filename]
      return !currentData || (!currentData.loading && !currentData.text && !currentData.error)
    })
    // Fetch transcripts for each file
    await Promise.all(filesToFetch.map(filename => fetchTranscript(filename)))
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
        setConfigComplete(data.is_complete)
        if (data.is_complete) {
          // Handle both old format (watch_directory) and new format (watch_directories)
          const directory = data.config.watch_directories?.[0] || data.config.watch_directory
          if (directory) {
            setWatchDirectory(directory)
          }
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

  // Fetch watch directory when config is complete
  useEffect(() => {
    if (configComplete) {
      const fetchWatchDirectory = async () => {
        try {
          const response = await fetch('/watch_directory')
          if (response.ok) {
            const data = await response.text()
            setWatchDirectory(data)
          }
        } catch (error) {
          console.error('Error fetching watch directory:', error)
        }
      }

      fetchWatchDirectory()
    }
  }, [configComplete])

  // Poll /queue endpoint every second (only when config is complete)
  useEffect(() => {
    if (!configComplete) return

    const interval = setInterval(async () => {
      try {
        const response = await fetch('/queue')
        if (response.ok) {
          const queueData = await response.json()
          if (queueData.queue && queueData.queue.length > 0) {
            setCurrentProcessingFile(queueData.current_file || null)
            setQueue(queueData.queue)
            // Extract video paths from queue items for regenerating files set
            const queueVideoPaths = queueData.queue.map((item: QueueItem) => {
              // Extract filename from full path
              const pathParts = item.video_path.split('/')
              const filename = pathParts[pathParts.length - 1]
              return filename.replace(/\.(mp4|MP4)$/, '')
            })
            setRegeneratingFiles(new Set(queueVideoPaths))
          } else {
            setQueue([])
            setCurrentProcessingFile(null)
            setRegeneratingFiles(new Set())
          }
        }
      } catch (error) {
        console.error('Error polling queue:', error)
      }
    }, 1000)

    return () => clearInterval(interval)
  }, [configComplete])

  useEffect(() => {
    refreshFiles()
    fetchExpandedTranscripts()
  }, [queue])

  const refreshFiles = async () => {
    try {
      const response = await fetch('/files')
      if (response.ok) {
        const filesData = await response.json()
        setFiles(filesData)
      }
    } catch (error) {
      console.error('Error fetching files:', error)
    }
  }

  useEffect(() => {
    // Refresh files list when queue changes or processing completes
    refreshFiles()
  }, [currentProcessingFile])

  // Fetch transcripts when expanded files change
  useEffect(() => {
    if (expandedFiles.size > 0) {
      fetchExpandedTranscripts()
    }
  }, [expandedFiles])

  // Setup intersection observer when expanded files change
  useEffect(() => {
    if (expandedFiles.size > 0) {
      setupIntersectionObserver()
    } else {
      setOutOfViewExpandedFile(null)
    }

    // Clear out-of-view file if it's no longer expanded
    if (outOfViewExpandedFile && !expandedFiles.has(outOfViewExpandedFile)) {
      setOutOfViewExpandedFile(null)
    }

    // Clear flashing state for files that are no longer expanded
    if (flashingRow && !expandedFiles.has(flashingRow)) {
      setFlashingRow(null)
    }

    // Cleanup observer on component unmount
    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect()
      }
      if (transcriptObserverRef.current) {
        transcriptObserverRef.current.disconnect()
      }
    }
  }, [expandedFiles, setupIntersectionObserver])

  // Find replacement when outOfViewExpandedFile becomes null
  useEffect(() => {
    if (!outOfViewExpandedFile && expandedFiles.size > 0) {
      // Check if there are any expanded rows where the transcript view is on screen but the file row is not
      for (const filename of expandedFiles) {
        const fileRow = fileRowRefs.current[filename]
        const transcriptRow = transcriptRowRefs.current[filename]
        
        if (fileRow && transcriptRow) {
          const fileRowRect = fileRow.getBoundingClientRect()
          const transcriptRowRect = transcriptRow.getBoundingClientRect()
          const topBarHeight = watchDirectory ? 64 : 0
          
          // Check if file row is above viewport (not visible)
          const isFileRowAboveViewport = fileRowRect.bottom < topBarHeight
          
          // Check if transcript row is at least partially visible
          const isTranscriptVisible = transcriptRowRect.bottom > topBarHeight && transcriptRowRect.top < window.innerHeight
          
          if (isFileRowAboveViewport && isTranscriptVisible) {
            setOutOfViewExpandedFile(filename)
            break
          }
        }
      }
    }
  }, [outOfViewExpandedFile, expandedFiles, watchDirectory])

  // Clear search results when search term is empty, or auto-search when 4+ characters
  useEffect(() => {
    if (!searchTerm.trim()) {
      setSearchLineNumbers({})
      setExpandedFiles(new Set())
    } else if (searchTerm.trim().length >= 4) {
      handleSearch()
    }
  }, [searchTerm])

  const handleSearch = async () => {
    if (!searchTerm.trim()) {
      setSearchLineNumbers({})
      setExpandedFiles(new Set())
      return
    }

    setIsSearching(true)
    try {
      // Get the line numbers for each file that contains the search term
      const lineNumbersResponse = await fetch(`/grep/${encodeURIComponent(searchTerm.trim())}`)
      if (lineNumbersResponse.ok) {
        const lineNumbersData = await lineNumbersResponse.json()
        setSearchLineNumbers(lineNumbersData)
        
        // Automatically expand files that contain search results
        const filesWithResults = Object.keys(lineNumbersData).filter(filename => 
          lineNumbersData[filename] && lineNumbersData[filename].length > 0
        )
        setExpandedFiles(new Set(filesWithResults))
      } else {
        setSearchLineNumbers({})
        setExpandedFiles(new Set())
      }
    } catch (error) {
      console.error('Search error:', error)
      setSearchLineNumbers({})
      setExpandedFiles(new Set())
    } finally {
      setIsSearching(false)
    }
  }

  const handleRegenerate = async (filename: string, e: React.MouseEvent) => {
    e.stopPropagation() // Prevent row expansion
    setRegeneratingFiles(prev => new Set(prev).add(filename))
    
    try {
      // Get CSRF token from meta tag
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content')
      
      const response = await fetch(`/transcripts/${encodeURIComponent(filename)}/regenerate`, {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json'
        }
      })
      if (response.ok) {
        // Reload the page after successful regeneration
        //window.location.reload()
      } else {
        console.error('Failed to regenerate transcript')
        setRegeneratingFiles(prev => {
          const newSet = new Set(prev)
          newSet.delete(filename)
          return newSet
        })
      }
    } catch (error) {
      console.error('Regeneration error:', error)
      setRegeneratingFiles(prev => {
        const newSet = new Set(prev)
        newSet.delete(filename)
        return newSet
      })
    }
  }

  const handleReplace = async (filename: string, e: React.MouseEvent) => {
    e.stopPropagation() // Prevent row expansion
    
    setReplacingFiles(prev => new Set(prev).add(filename))
    
    try {
      // Fetch the current transcript content
      const response = await fetch(`/transcripts/${encodeURIComponent(filename)}`)
      
      if (!response.ok) {
        throw new Error(`Failed to fetch transcript: ${response.status} ${response.statusText}`)
      }
      
      const transcriptContent = await response.text()
      
      // Set up the replace dialog
      setReplaceTranscriptFilename(filename)
      setReplaceTranscriptInitialContent(transcriptContent)
      setIsReplaceDialogOpen(true)
      
    } catch (error) {
      console.error('Error fetching transcript for replace:', error)
      alert('Error: Failed to load transcript. Please try again.')
    } finally {
      setReplacingFiles(prev => {
        const newSet = new Set(prev)
        newSet.delete(filename)
        return newSet
      })
    }
  }

  const handleReplaceTranscript = async (newText: string) => {
    if (!replaceTranscriptFilename || !newText.trim()) return;
    
    setIsReplacingTranscript(true);
    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');
      
      const response = await fetch(`/transcripts/${encodeURIComponent(replaceTranscriptFilename)}/replace`, {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ text: newText })
      });
      
      if (response.ok) {
        // Update the transcript data with the new content
        setTranscriptData(prev => ({
          ...prev,
          [replaceTranscriptFilename]: { text: newText, loading: false, error: null }
        }))
        
        // Force a refresh of the files list to update any metadata
        try {
          const filesResponse = await fetch('/files')
          if (filesResponse.ok) {
            const filesData = await filesResponse.json()
            setFiles(filesData)
          }
        } catch (error) {
          console.error('Error refreshing files after replace:', error)
        }
        
        // Close the dialog
        setIsReplaceDialogOpen(false)
        setReplaceTranscriptFilename('')
        setReplaceTranscriptInitialContent('')
      } else {
        const error = await response.json();
        alert(`Error: ${error.error || 'Failed to replace transcript'}`);
      }
    } catch (error) {
      console.error('Error replacing transcript:', error);
      alert('Error: Failed to replace transcript. Please try again.');
    } finally {
      setIsReplacingTranscript(false);
    }
  }

  const handleReplaceCancel = () => {
    setIsReplaceDialogOpen(false)
    setReplaceTranscriptFilename('')
    setReplaceTranscriptInitialContent('')
  }

  const handleRegenerateMeta = async (filename: string, e: React.MouseEvent) => {
    e.stopPropagation() // Prevent row expansion    
    try {
      // Get CSRF token from meta tag
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content')
      
      const response = await fetch(`/transcripts/${encodeURIComponent(filename)}/regenerate_meta`, {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json'
        }
      })
      
      if (response.ok) {
        // Refresh the files list to update the length value
        await refreshFiles()
      } else {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }))
        console.error('Failed to regenerate meta file:', errorData.error)
        alert(`Failed to regenerate meta file: ${errorData.error}`)
      }
    } catch (error) {
      console.error('Meta regeneration error:', error)
      alert('Error regenerating meta file. Please try again.')
    }
  }

  const handleConfigComplete = () => {
    setConfigComplete(true)
  }

  const handleConfigUpdate = async () => {
    // Refresh the watch directory after config update
    try {
      const response = await fetch('/watch_directory')
      if (response.ok) {
        const data = await response.text()
        setWatchDirectory(data)
      }
    } catch (error) {
      console.error('Error fetching updated watch directory:', error)
    }
    
    // Clear search state and transcript data from old directory
    setSearchTerm('')
    setSearchLineNumbers({})
    setExpandedFiles(new Set())
    setTranscriptData({})
    
    // Refresh the file list to show files from the new directory
    try {
      refreshFiles()
    } catch (error) {
      console.error('Error refreshing files from new directory:', error)
    }
    
    // Close the config editor
    setIsConfigEditorOpen(false)
  }

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
        <ConfigSetup onConfigComplete={handleConfigComplete} />
        
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

  // Show configuration editor if requested
  if (isConfigEditorOpen) {
    return (
      <>
        <ConfigSetup onConfigComplete={handleConfigUpdate} isEditMode={true} />
        
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
      {/* Watch Directory Bar - Fixed to top */}
      {watchDirectory && (
        <div className="fixed top-0 left-0 right-0 bg-muted/50 border-b border-border px-4 py-2 z-10 backdrop-blur-sm">
          <div className="container mx-auto">
            <div className="flex gap-6 justify-between items-center">
              <div className="flex gap-6 items-center flex-1">
                <div className="flex gap-2 items-center">
                  <div className="text-sm text-foreground font-medium text-left">
                    Directory: {watchDirectory}
                  </div>
                  <button
                    onClick={() => setIsConfigEditorOpen(true)}
                    className="p-1 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors"
                    title="Edit configuration"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                    </svg>
                  </button>
                </div>
                
                {/* Search Bar in Top Bar */}
                <div className="flex gap-2 items-center">
                  <input
                    type="text"
                    placeholder="Search transcripts..."
                    value={searchTerm}
                    onChange={(e) => setSearchTerm(e.target.value)}
                    className="px-2 py-1 text-sm border border-input bg-background text-foreground rounded focus:outline-none focus:ring-1 focus:ring-ring focus:border-transparent w-48"
                  />
                  <button
                    onClick={handleSearch}
                    disabled={isSearching}
                    className="px-2 py-1 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {isSearching ? 'Searching...' : 'Search'}
                  </button>
                </div>
              </div>
              
              {/* Scroll and Collapse Links */}
              {outOfViewExpandedFile && (
                <div className="flex gap-4 items-center">
                  <button
                    onClick={handleScrollToTop}
                    className="text-sm text-primary hover:text-primary/80 underline transition-colors"
                    title={`Scroll to ${outOfViewExpandedFile}`}
                  >
                    scroll to row
                  </button>
                  <button
                    onClick={handleCollapseExpanded}
                    className="text-sm text-primary hover:text-primary/80 underline transition-colors"
                    title={`Collapse ${outOfViewExpandedFile}`}
                  >
                    collapse
                  </button>
                </div>
              )}
              
              <div className="flex gap-4 items-center">
                {currentProcessingFile && (
                  <div className="text-sm text-primary font-medium">
                    Processing: {currentProcessingFile.video_path.split('/').pop()?.replace(/\.(mp4|MP4)$/, '')} ({currentProcessingFile.process_type})
                  </div>
                )}
                <div className="relative group">
                  <div className="text-sm text-foreground font-medium">
                    Queue: {queue.length}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Main content with top padding to account for fixed header */}
      <div className={`container mx-auto py-10 ${watchDirectory ? 'pt-16' : ''}`}>
        <h1 className="text-2xl font-bold mb-6 text-foreground">File List</h1>
        
        {/* File List - Full Width */}
        <div>
          {/* Search Results */}
          {searchResults.length > 0 && (
                    <div className="mb-6 p-4 bg-accent/10 border border-accent/20 rounded-md">
          <h3 className="text-sm font-medium text-accent-foreground mb-2">
                Found in {searchResults.length} file(s)
              </h3>
            </div>
          )}
          
          {searchTerm && searchResults.length === 0 && !isSearching && (
            <div className="mb-6 p-4 bg-muted border border-border rounded-md">
              <p className="text-sm text-muted-foreground">No files found containing "{searchTerm}"</p>
            </div>
          )}

          <Table className="table-fixed w-full">
            <TableHeader>
              <TableRow>
                <TableHead 
                  className="text-center w-[16%] cursor-pointer hover:bg-accent transition-colors"
                  onClick={() => handleSort('name')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Filename
                    {getSortIndicator('name')}
                  </div>
                  {searchTerm && searchResults.length > 0 && (
                                            <span className="text-xs text-primary ml-2">(Search Results)</span>
                  )}
                </TableHead>
                <TableHead 
                  className="text-center w-[14%] cursor-pointer hover:bg-accent transition-colors"
                  onClick={() => handleSort('created_at')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Date
                    {getSortIndicator('created_at')}
                  </div>
                </TableHead>
                <TableHead 
                  className="text-center w-[14%] pl-6 cursor-pointer hover:bg-accent transition-colors"
                  onClick={() => handleSort('last_generated')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Last Generated
                    {getSortIndicator('last_generated')}
                  </div>
                </TableHead>
                <TableHead 
                  className="text-center w-[10%] cursor-pointer hover:bg-accent transition-colors"
                  onClick={() => handleSort('line_count')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Lines
                    {getSortIndicator('line_count')}
                  </div>
                </TableHead>
                <TableHead 
                  className="text-center w-[10%] cursor-pointer hover:bg-accent transition-colors"
                  onClick={() => handleSort('length')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Length
                    {getSortIndicator('length')}
                  </div>
                </TableHead>
                <TableHead 
                  className="text-center w-[16%] cursor-pointer hover:bg-accent transition-colors"
                  onClick={() => handleSort('model')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Source
                    {getSortIndicator('model')}
                  </div>
                </TableHead>
                <TableHead className="text-center w-[20%]">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {sortedFiles.map((file, index) => {
                if (searchTerm != '' && !searchResults.includes(file.base_name)) {
                  return <></>;
                }
                
                const transcriptInfo = transcriptData[file.base_name] || { text: '', loading: false, error: null }
                
                return (
                <>
                <TableRow 
                  key={index} 
                  ref={(el) => { fileRowRefs.current[file.base_name] = el }}
                  data-filename={file.base_name}
                  className={flashingRow === file.base_name ? 'animate-pulse bg-primary/10' : ''}
                  onClick={() => {
                  // Only expand if transcript exists
                  if (file.transcript) {
                    setExpandedFiles(prev => {
                      const newSet = new Set(prev)
                      if (newSet.has(file.base_name)) {
                        newSet.delete(file.base_name)
                      } else {
                        newSet.add(file.base_name)
                      }
                      return newSet
                    })
                  }
                }}>
                  <TableCell className="font-medium w-[16%]">
                    <a 
                      href={`/player/${encodeURIComponent(file.base_name)}`}
                                                className="text-primary hover:text-primary/80 underline block truncate text-right"
                      onClick={(e) => e.stopPropagation()}
                      target="_blank"
                      rel="noopener noreferrer"
                      title={file.name}
                    >
                      <span className="inline-block w-full truncate" style={{ direction: 'rtl', textAlign: 'left' }}>
                        {file.name}
                      </span>
                    </a>
                  </TableCell>
                  <TableCell className="w-[14%] pr-10 text-foreground">{formatDate(file.created_at)}</TableCell>
                  <TableCell className="w-[14%] pl-10 text-foreground">{formatDate(file.last_generated || '')}</TableCell>
                  <TableCell className="w-[10%] text-foreground">{file.line_count || 0}</TableCell>
                  <TableCell className="w-[10%] text-foreground">
                    {file.length ? (
                      file.length
                    ) : (
                      <button
                        onClick={(e) => handleRegenerateMeta(file.base_name, e)}
                        disabled={regeneratingFiles.has(file.base_name)}
                        className="p-1 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                        title="Generate video length"
                      >
                        {regeneratingFiles.has(file.base_name) ? (
                          <svg className="w-4 h-4 animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                        ) : (
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                          </svg>
                        )}
                      </button>
                    )}
                  </TableCell>
                  <TableCell className="w-[16%] text-center">
                    {file.model ? (
                      <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${getModelChipColor(file.model)}`}>
                        {file.model}
                      </span>
                    ) : (
                      <span className="text-muted-foreground">-</span>
                    )}
                  </TableCell>
                  <TableCell className="w-[20%] text-center">
                    <div className="flex justify-center gap-2">
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          window.open(`/player/${encodeURIComponent(file.base_name)}`, '_blank')
                        }}
                                                    className="p-2 text-muted-foreground hover:text-primary hover:bg-primary/10 rounded-md transition-colors"
                        title="View file"
                      >
                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                        </svg>
                      </button>
                      
                      {/* Only show regenerate button if transcript exists */}
                      {file.transcript ? (
                        <>
                          <button
                            onClick={(e) => handleRegenerate(file.base_name, e)}
                            disabled={regeneratingFiles.has(file.base_name)}
                            className={`p-2 text-muted-foreground hover:text-primary hover:bg-accent rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${
                              isFileBeingProcessed(file.base_name) ? 'animate-reverse-spin' : ''
                            }`}
                            title="Regenerate transcript"
                          >
                            {regeneratingFiles.has(file.base_name) ? (
                              <svg className="w-5 h-5 animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                            </svg>
                            ) : (
                              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                              </svg>
                            )}
                          </button>
                          
                          <button
                            onClick={(e) => handleReplace(file.base_name, e)}
                            disabled={replacingFiles.has(file.base_name)}
                            className={`p-2 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed`}
                            title="Edit transcript"
                          >
                            {replacingFiles.has(file.base_name) ? (
                              <></>
                            ) : (
                              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                              </svg>
                            )}
                          </button>
                        </>
                      ) : isFileBeingProcessed(file.base_name) ? (
                        <button
                          disabled={true}
                          className="p-2 text-muted-foreground opacity-50 cursor-not-allowed rounded-md"
                          title="Processing transcript"
                        >
                          <svg className="w-5 h-5 animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                            </svg>
                        </button>
                      ) : null}
                    </div>
                  </TableCell>
                </TableRow>
                <TableRow
                  ref={(el) => { transcriptRowRefs.current[file.base_name] = el }}
                  data-filename={file.base_name}
                >
                  
                <TableCell colSpan={7} className="p-0">
                  <TranscriptView
                    visible={expandedFiles.has(file.base_name)}
                    name={file.base_name}
                    className="w-full"
                    searchTerm={searchTerm}
                    text={transcriptInfo.text}
                    loading={transcriptInfo.loading}
                    error={transcriptInfo.error}
                    visibleLines={searchLineNumbers[file.base_name] || []}
                    expandContext={expandContext}
                    expandAll={expandAll}
                    onEditSuccess={() => { fetchTranscript(file.base_name) }}
                  />
                </TableCell>
                </TableRow>
                </>
              )})}
            </TableBody>
          </Table>
        </div>
      </div>

      {/* Replace Transcript Dialog */}
      <EditDialog
        isOpen={isReplaceDialogOpen}
        title={`Replace Transcript - ${replaceTranscriptFilename}`}
        initialValue={replaceTranscriptInitialContent}
        onSave={handleReplaceTranscript}
        onCancel={handleReplaceCancel}
        isSubmitting={isReplacingTranscript}
        placeholder="Enter the complete transcript content..."
        isLargeMode={true}
      />

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
