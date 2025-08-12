import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuCheckboxItem,
  DropdownMenuSeparator,
  DropdownMenuLabel,
  DropdownMenuItem,
} from '../components/ui/dropdown-menu'
import TranscriptView from '../components/TranscriptView'
import DualEditDialog from '../components/DualEditDialog'
import RightPanePlaceholder from '../components/RightPanePlaceholder'
import MobileTranscriptList from '../components/MobileTranscriptList'
import { useEffect, useState, useRef, useCallback } from 'react'
import { useLSState } from '../hooks/useLSState'
import { useIsSmallScreen } from '../hooks/useMediaQuery'
import { useNavigate } from 'react-router-dom'
import { addTimestamp } from '../lib/utils'

export default function HomePage() {
  const navigate = useNavigate()
  const isSmallScreen = useIsSmallScreen()
  
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
  
  const [files, setFiles] = useState<FileRow[]>(window.autotranscript_files as FileRow[])
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set())
  const [searchTerm, setSearchTerm] = useState('')
  const [activeSearchTerm, setActiveSearchTerm] = useState('')
  const [searchLineNumbers, setSearchLineNumbers] = useState<Record<string, number[]>>({})
  const [isSearching, setIsSearching] = useState(false)
  const [regeneratingFiles, setRegeneratingFiles] = useState<Set<string>>(new Set())
  const [isBulkRegenerating, setIsBulkRegenerating] = useState(false)
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
  const [selectedWatchDirs, setSelectedWatchDirs] = useLSState<string[]>('selectedWatchDirs', [])
  const [availableWatchDirs, setAvailableWatchDirs] = useState<string[]>([])
  const [selectedSources, setSelectedSources] = useLSState<string[]>('selectedSources', [])
  const [availableSources, setAvailableSources] = useState<string[]>([])
  
  // Replace transcript dialog state
  const [isReplaceDialogOpen, setIsReplaceDialogOpen] = useState(false)
  const [replaceTranscriptFilename, setReplaceTranscriptFilename] = useState('')
  const [replaceTranscriptInitialContent, setReplaceTranscriptInitialContent] = useState('')
  const [isReplacingTranscript, setIsReplacingTranscript] = useState(false)
  
  // Rename dialog state
  const [isRenameDialogOpen, setIsRenameDialogOpen] = useState(false)
  const [renameFilename, setRenameFilename] = useState('')
  const [newFilename, setNewFilename] = useState('')
  const [isRenaming, setIsRenaming] = useState(false)
  const [renameError, setRenameError] = useState('')
  
  // State for tracking out-of-view expanded rows
  const [outOfViewExpandedFile, setOutOfViewExpandedFile] = useState<string | null>(null)
  const [flashingRow, setFlashingRow] = useState<string | null>(null)
  
  // Right pane URL state
  const [rightPaneUrl, setRightPaneUrl] = useState<string>('')
  const [leftPaneWidth, setLeftPaneWidth] = useState<number>(0)
  const [isLeftPaneWidthMeasured, setIsLeftPaneWidthMeasured] = useState<boolean>(false)
  
  // Clip state variables
  const [clipStart, setClipStart] = useState<number | null>(null)
  const [clipEnd, setClipEnd] = useState<number | null>(null)
  const [clipTranscript, setClipTranscript] = useState<string | null>(null)
  const fileRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const transcriptRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const observerRef = useRef<IntersectionObserver | null>(null)
  const transcriptObserverRef = useRef<IntersectionObserver | null>(null)
  const leftPaneRef = useRef<HTMLDivElement | null>(null)

  // Helper function to check if a file is currently being processed
  const isFileBeingProcessed = (filename: string): boolean => {
    if (!currentProcessingFile) return false
    const currentFileName = currentProcessingFile.video_path.split('/').pop()?.replace(/\.(mp4|MP4)$/, '')
    return currentFileName === filename
  }

  // Calculate search results from search line numbers
  const searchResults = Object.keys(searchLineNumbers).filter(filename => 
    searchLineNumbers[filename] && (searchLineNumbers[filename].length > 0)
  )

  // Determine if we should use mobile view based on left pane width or screen size
  const shouldUseMobileView = isSmallScreen || (isLeftPaneWidthMeasured && leftPaneWidth < 753)

  // Fetch configured watch directories from the API
  useEffect(() => {
    const fetchWatchDirectories = async () => {
      try {
        const response = await fetch(addTimestamp('/watch_directories'))
        if (response.ok) {
          const dirs = await response.json()
          setAvailableWatchDirs(dirs || [])
        }
      } catch (error) {
        console.error('Error fetching watch directories:', error)
      }
    }
    
    fetchWatchDirectories()
  }, [])

  // Fetch available sources from the API
  useEffect(() => {
    const fetchSources = async () => {
      try {
        const response = await fetch(addTimestamp('/sources'))
        if (response.ok) {
          const sources = await response.json()
          setAvailableSources(sources || [])
        }
      } catch (error) {
        console.error('Error fetching sources:', error)
      }
    }
    
    fetchSources()
  }, [])

  // Initial refresh when component mounts and filters are ready
  useEffect(() => {
    const hasInitializedWatchDirs = availableWatchDirs.length === 0 || selectedWatchDirs.length > 0
    const hasInitializedSources = availableSources.length === 0 || selectedSources.length > 0
    
    if (hasInitializedWatchDirs && hasInitializedSources) {
      refreshFiles()
      refreshQueue()
    }
  }, [availableWatchDirs, availableSources, selectedWatchDirs, selectedSources])

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
      window.scrollTo({ top: scrollTop })
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
      'subtitles': 'bg-teal-100 text-teal-800 dark:bg-teal-900 dark:text-teal-200',
      'manual': 'bg-pink-100 text-pink-800 dark:bg-pink-900 dark:text-pink-200',
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
  const formatDate = (dateString: string, includeTime: boolean = true): string => {
    if (!dateString || dateString === 'N/A') return 'N/A'
    
    try {
      const date = new Date(dateString.replace(' ', 'T'))
      if (isNaN(date.getTime())) return 'N/A'
      
      const month = (date.getMonth() + 1).toString().padStart(2, '0')
      const day = date.getDate().toString().padStart(2, '0')
      const year = date.getFullYear()
      
      if (!includeTime) {
        return `${month}-${day}-${year}`
      }
      
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
      const response = await fetch(addTimestamp(`/transcripts/${encodeURIComponent(filename)}`))
      
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

  // Fetch watch directory on component mount
  useEffect(() => {
    const fetchWatchDirectory = async () => {
      try {
        const response = await fetch(addTimestamp('/watch_directory'))
        if (response.ok) {
          const data = await response.text()
          setWatchDirectory(data)
        }
      } catch (error) {
        console.error('Error fetching watch directory:', error)
      }
    }
    
    fetchWatchDirectory()
  }, [])

  // Set up intersection observer when expanded files change
  useEffect(() => {
    setupIntersectionObserver()
    
    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect()
      }
      if (transcriptObserverRef.current) {
        transcriptObserverRef.current.disconnect()
      }
    }
  }, [setupIntersectionObserver])

  // Set up ResizeObserver to track left pane width
  useEffect(() => {
    if (!leftPaneRef.current) return

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setLeftPaneWidth(entry.contentRect.width)
        setIsLeftPaneWidthMeasured(true)
      }
    })

    resizeObserver.observe(leftPaneRef.current)

    return () => {
      resizeObserver.disconnect()
    }
  }, [])

  // Fetch transcripts when files are expanded
  useEffect(() => {
    fetchExpandedTranscripts()
  }, [expandedFiles])

  // Refresh files when selectedWatchDirs changes
  useEffect(() => {
    refreshFiles()
  }, [selectedWatchDirs])

  // Refresh files when selectedSources changes
  useEffect(() => {
    refreshFiles()
  }, [selectedSources])

  const refreshFiles = async () => {
    try {
      const params = new URLSearchParams()
      params.append('watch_directories', selectedWatchDirs.join(','))
      params.append('sources', selectedSources.join(','))

      
      const queryString = params.toString()
      const url = queryString ? `/files?${queryString}` : '/files'
      
      const response = await fetch(addTimestamp(url))
      if (response.ok) {
        const data = await response.json()
        setFiles(data || [])
      }
    } catch (error) {
      console.error('Error refreshing files:', error)
    }
  }

  const refreshQueue = async () => {
    try {
      const response = await fetch(addTimestamp('/queue'))
      if (response.ok) {
        const data = await response.json()
        setQueue(data.queue || [])
        setCurrentProcessingFile(data.current_file || null)
        
        // Update regeneratingFiles based on queue
        const filesInQueue = new Set<string>(
          data.queue
            .filter((item: QueueItem) => item.process_type === 'regenerate')
            .map((item: QueueItem) => item.video_path.split('/').pop()?.replace(/\.(mp4|MP4)$/, '') || '')
        )
        
        // Add current processing file if it's a regenerate
        if (data.current_processing && data.current_processing.process_type === 'regenerate') {
          const currentFileName = data.current_processing.video_path.split('/').pop()?.replace(/\.(mp4|MP4)$/, '')
          if (currentFileName) {
            filesInQueue.add(currentFileName)
          }
        }
        
        setRegeneratingFiles(filesInQueue)
      }
    } catch (error) {
      console.error('Error refreshing queue:', error)
    }
  }

  // Poll for updates every 3 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      refreshFiles()
      refreshQueue()
    }, 3000)
    
    return () => clearInterval(interval)
  }, [availableWatchDirs, availableSources, selectedWatchDirs, selectedSources])



  // useEffect for right pane URL changes
  useEffect(() => {
    // This effect will trigger when rightPaneUrl changes
    // The layout change will be handled in the render section
  }, [rightPaneUrl])

  const handleSearch = async () => {
    if (!searchTerm.trim()) {
      setSearchLineNumbers({})
      setActiveSearchTerm('')
      return
    }

    setIsSearching(true)

    try {
      const response = await fetch(addTimestamp(`/grep/${encodeURIComponent(searchTerm)}`))
      if (response.ok) {
        const data = await response.json()
        setSearchLineNumbers(data || {})
        setActiveSearchTerm(searchTerm)
        
        // Expand all files that have search results
        const filesWithResults = Object.keys(data || {}).filter(filename => 
          data[filename] && data[filename].length > 0
        )
        setExpandedFiles(new Set(filesWithResults))
      }
    } catch (error) {
      console.error('Error searching:', error)
    } finally {
      setIsSearching(false)
    }
  }

  const handleClearSearch = () => {
    setSearchTerm('')
    setActiveSearchTerm('')
    setSearchLineNumbers({})
    setExpandedFiles(new Set()) // Collapse all expanded files
  }

  const handleRegenerate = async (filename: string, e: React.MouseEvent) => {
    e.stopPropagation()
    
    // Add to regenerating set immediately for UI feedback
    setRegeneratingFiles(prev => new Set(prev).add(filename))
    const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');

    try {
      const response = await fetch(addTimestamp(`/regenerate/${encodeURIComponent(filename)}`), {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
      })
      
      if (!response.ok) {
        // Remove from regenerating set if request failed
        setRegeneratingFiles(prev => {
          const newSet = new Set(prev)
          newSet.delete(filename)
          return newSet
        })
        
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }))
        console.error('Failed to regenerate transcript:', errorData.error)
        alert(`Failed to regenerate transcript: ${errorData.error}`)
      }
      // If successful, the file will remain in regeneratingFiles until the queue refresh removes it
    } catch (error) {
      // Remove from regenerating set if request failed
      setRegeneratingFiles(prev => {
        const newSet = new Set(prev)
        newSet.delete(filename)
        return newSet
      })
      console.error('Regeneration error:', error)
      alert('Error regenerating transcript. Please try again.')
    }
  }

  const handleReplace = async (filename: string, e: React.MouseEvent) => {
    e.stopPropagation()
    
    // Add to replacing set immediately for UI feedback
    setReplacingFiles(prev => new Set(prev).add(filename))
    
    try {
      // Fetch the current transcript
      const response = await fetch(addTimestamp(`/transcripts/${encodeURIComponent(filename)}`))
      if (response.ok) {
        const transcriptContent = await response.text()
        setReplaceTranscriptFilename(filename)
        setReplaceTranscriptInitialContent(transcriptContent)
        setIsReplaceDialogOpen(true)
      } else {
        throw new Error('Failed to fetch transcript')
      }
    } catch (error) {
      console.error('Error fetching transcript for replacement:', error)
      alert('Error loading transcript. Please try again.')
    } finally {
      // Remove from replacing set
      setReplacingFiles(prev => {
        const newSet = new Set(prev)
        newSet.delete(filename)
        return newSet
      })
    }
  }

  const handleReplaceTranscript = async (newText: string) => {
    setIsReplacingTranscript(true)
    
    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');

      const response = await fetch(addTimestamp(`/transcripts/${encodeURIComponent(replaceTranscriptFilename)}/replace`), {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ text: newText })
      })
      
      if (response.ok) {
        // Close the dialog
        setIsReplaceDialogOpen(false)
        setReplaceTranscriptFilename('')
        setReplaceTranscriptInitialContent('')
        
        // If the file is expanded, refresh its transcript
        if (expandedFiles.has(replaceTranscriptFilename)) {
          await fetchTranscript(replaceTranscriptFilename)
        }
        
        // Refresh the file list to update line count
        await refreshFiles()
      } else {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }))
        console.error('Failed to replace transcript:', errorData.error)
        alert(`Failed to replace transcript: ${errorData.error}`)
      }
    } catch (error) {
      console.error('Replace transcript error:', error)
      alert('Error replacing transcript. Please try again.')
    } finally {
      setIsReplacingTranscript(false)
    }
  }

  const handleReplaceCancel = () => {
    setIsReplaceDialogOpen(false)
    setReplaceTranscriptFilename('')
    setReplaceTranscriptInitialContent('')
  }

  const handleRename = async (filename: string, e: React.MouseEvent) => {
    e.stopPropagation()
    
    // Extract just the filename part (remove any folder path)
    const baseFilename = filename.split('/').pop()?.split('\\').pop() || filename
    
    setRenameFilename(filename) // Keep original for API call
    setNewFilename(baseFilename) // Pre-populate with just the filename
    setRenameError('')
    setIsRenameDialogOpen(true)
    
    // Select all text in the input field after a brief delay to ensure the dialog is rendered
    setTimeout(() => {
      const input = document.getElementById('new-filename') as HTMLInputElement
      if (input) {
        input.select()
      }
    }, 100)
  }

  const handleRenameSubmit = async () => {
    if (!newFilename.trim()) {
      setRenameError('Filename cannot be empty')
      return
    }

    const baseFilename = renameFilename.split('/').pop()?.split('\\').pop() || renameFilename
    if (newFilename === baseFilename) {
      setRenameError('New filename must be different from current filename')
      return
    }

    if (/[\/\\]/.test(newFilename)) {
      setRenameError('Filename cannot contain path separators')
      return
    }

    setIsRenaming(true)
    setRenameError('')

    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');

      const response = await fetch(addTimestamp(`/transcripts/${encodeURIComponent(renameFilename)}/rename`), {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          new_filename: newFilename.trim()
        })
      })

      if (!response.ok) {
        const errorData = await response.json()
        throw new Error(errorData.error || `Failed to rename file: ${response.status} ${response.statusText}`)
      }

      // Success - close dialog and refresh files
      setIsRenameDialogOpen(false)
      setRenameFilename('')
      setNewFilename('')
      await refreshFiles()

    } catch (err) {
      console.error('Error renaming file:', err)
      setRenameError(err instanceof Error ? err.message : 'An error occurred while renaming the file')
    } finally {
      setIsRenaming(false)
    }
  }

  const handleRenameCancel = () => {
    setIsRenameDialogOpen(false)
    setRenameFilename('')
    setNewFilename('')
    setRenameError('')
  }

  const handleRegenerateMeta = async (filename: string, e: React.MouseEvent) => {
    e.stopPropagation()
    
    // Add to regenerating set for UI feedback
    setRegeneratingFiles(prev => new Set(prev).add(filename))
    const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');

    try {
      const response = await fetch(addTimestamp(`/regenerate-meta/${encodeURIComponent(filename)}`), {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
      })
      
      if (response.ok) {
        // Refresh files to show updated meta
        await refreshFiles()
      } else {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }))
        console.error('Failed to regenerate meta file:', errorData.error)
        alert(`Failed to regenerate meta file: ${errorData.error}`)
      }
    } catch (error) {
      console.error('Meta regeneration error:', error)
      alert('Error regenerating meta file. Please try again.')
    } finally {
      // Remove from regenerating set
      setRegeneratingFiles(prev => {
        const newSet = new Set(prev)
        newSet.delete(filename)
        return newSet
      })
    }
  }

  const handleWatchDirToggle = (dir: string) => {
    console.log("handleWatchDirToggle", dir)
    setSelectedWatchDirs(prev => {
      const newSelection = prev.includes(dir) 
        ? prev.filter(d => d !== dir)
        : [...prev, dir]
      return newSelection
    })
  }

  const handleSelectAllWatchDirs = () => {
    console.log("handleSelectAllWatchDirs", availableWatchDirs)
    setSelectedWatchDirs(availableWatchDirs)
  }

  const handleDeselectAllWatchDirs = () => {
    console.log("handleDeselectAllWatchDirs")
    setSelectedWatchDirs([])
  }

  const handleSourceToggle = (source: string) => {
    setSelectedSources(prev => {
      const newSelection = prev.includes(source) 
        ? prev.filter(s => s !== source)
        : [...prev, source]
      return newSelection
    })
  }

  const handleSelectAllSources = () => {
    setSelectedSources(availableSources)
  }

  const handleDeselectAllSources = () => {
    setSelectedSources([])
  }

  const handleSetRightPaneUrl = useCallback((url: string) => {
    if (isSmallScreen) {
      // On mobile, open URL in new browser window
      window.open(url, '_blank')
    } else {
      // On desktop, set the state variable
      setRightPaneUrl(url)
    }
  }, [isSmallScreen])

  // Clip management methods
  const handleSetClipStart = (time: number, transcript: string) => {
    if (clipTranscript && clipTranscript !== transcript) {
      // Different transcript - clear existing clip and set new start
      setClipStart(time)
      setClipEnd(null)
      setClipTranscript(transcript)
    } else {
      // Same transcript or no existing clip
      setClipStart(time)
      setClipTranscript(transcript)
    }
  }

  const handleSetClipEnd = (time: number, transcript: string) => {
    if (clipTranscript && clipTranscript !== transcript) {
      // Different transcript - clear existing clip and set new end
      setClipStart(null)
      setClipEnd(time)
      setClipTranscript(transcript)
    } else {
      // Same transcript or no existing clip
      setClipEnd(time)
      setClipTranscript(transcript)
    }
  }

  const handleClearClip = () => {
    setClipStart(null)
    setClipEnd(null)
    setClipTranscript(null)
    // Clear right pane to show placeholder
    setRightPaneUrl('')
  }

  const handleClipBlock = (startTime: number, endTime: number, transcript: string) => {
    setClipStart(startTime)
    setClipEnd(endTime)
    setClipTranscript(transcript)
  }



  // Auto-update right pane when both clip start and end are set, or clear when incomplete
  useEffect(() => {
    if (clipStart !== null && clipEnd !== null && clipTranscript) {
      const url = `/clip_player/${encodeURIComponent(clipTranscript)}?start_time=${clipStart}&end_time=${clipEnd}&display_text=false`
      handleSetRightPaneUrl(url)
    } else if (clipStart !== null || clipEnd !== null) {
      // If we have partial clip data, clear the right pane to show placeholder
      setRightPaneUrl('')
    }
  }, [clipStart, clipEnd, clipTranscript, handleSetRightPaneUrl])

  // Helper function to handle file expansion
  const handleExpandFile = (filename: string) => {
    setExpandedFiles(prev => {
      const newSet = new Set(prev)
      if (newSet.has(filename)) {
        newSet.delete(filename)
      } else {
        newSet.add(filename)
      }
      return newSet
    })
  }

  // Make the function available globally for testing
  useEffect(() => {
    if (typeof window !== 'undefined') {
      (window as any).handleSetRightPaneUrl = handleSetRightPaneUrl
    }
  }, [handleSetRightPaneUrl])

  const handleBulkRegenerate = async () => {
    // Get all files that are currently displayed
    const displayedFiles = sortedFiles.filter(file => {
      // Filter out files that don't match search if there's an active search
      if (activeSearchTerm && !searchResults.includes(file.base_name)) {
        return false
      }
      // Only include files that have transcripts
      return file.transcript
    })

    if (displayedFiles.length === 0) {
      alert('No files with transcripts to regenerate')
      return
    }

    const selectedSource = selectedSources[0]
    const fileCount = displayedFiles.length
    const confirmMessage = `Regenerate ${fileCount} file${fileCount === 1 ? '' : 's'} using "${selectedSource}" source?`
    
    if (!confirm(confirmMessage)) {
      return
    }

    setIsBulkRegenerating(true)
    const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content')

    try {
      let successCount = 0
      let errorCount = 0
      
      // Process files in parallel
      const results = await Promise.allSettled(
        displayedFiles.map(async (file) => {
          const response = await fetch(addTimestamp(`/regenerate/${encodeURIComponent(file.base_name)}`), {
            method: 'POST',
            headers: {
              'X-CSRF-Token': csrfToken || '',
              'Content-Type': 'application/json',
            },
          })
          
          if (!response.ok) {
            const errorData = await response.json().catch(() => ({ error: 'Unknown error' }))
            throw new Error(`${file.base_name}: ${errorData.error}`)
          }
          
          return file.base_name
        })
      )
      
      // Count successes and failures
      results.forEach((result) => {
        if (result.status === 'fulfilled') {
          successCount++
        } else {
          errorCount++
          console.error('Bulk regeneration error:', result.reason)
        }
      })
      
      if (errorCount > 0) {
        alert(`Bulk regeneration completed with ${successCount} successes and ${errorCount} errors. Check console for details.`)
      } else {
        alert(`Successfully queued ${successCount} files for regeneration`)
      }
    } catch (error) {
      console.error('Bulk regeneration error:', error)
      alert('Error during bulk regeneration. Please try again.')
    } finally {
      setIsBulkRegenerating(false)
    }
  }

  return (
    <>
      {/* Watch Directory Bar - Fixed to top */}
      {watchDirectory && (
        <div className="fixed top-0 left-0 right-0 bg-muted/50 border-b border-border px-2 sm:px-4 py-2 z-10 backdrop-blur-sm">
          <div className="w-full">
            <div className="flex gap-2 sm:gap-6 justify-between items-center">
              <div className="flex gap-2 sm:gap-6 items-center flex-1 min-w-0">
                <div className="flex gap-2 items-center flex-shrink-0">
                  <button
                    onClick={() => navigate('/config')}
                    className="p-1 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors group"
                    title="Edit configuration"
                  >
                    <svg className="w-4 h-4 group-hover:animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 11a2 2 0 100-4 2 2 0 000 4z" />
                    </svg>
                  </button>
                  <div className="relative">
                    <button
                      onClick={() => navigate('/queue')}
                      className="p-1 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors"
                      title="View processing queue"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 10h16M4 14h16M4 18h16" />
                      </svg>
                    </button>
                    {queue.length > 0 && (
                      <div className="absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full min-w-[18px] h-[18px] flex items-center justify-center px-1">
                        {queue.length}
                      </div>
                    )}
                  </div>
                  
                  {/* Search Bar in Top Bar - Left justified with nav buttons */}
                  <div className="flex gap-1 items-center">
                    <div className="relative">
                      <input
                        type="text"
                        placeholder="Search transcripts..."
                        value={searchTerm}
                        onChange={(e) => {
                          const newValue = e.target.value
                          setSearchTerm(newValue)
                          // If user deletes all text, clear the filtering
                          if (newValue.trim() === '') {
                            setActiveSearchTerm('')
                            setSearchLineNumbers({})
                            setExpandedFiles(new Set())
                          }
                        }}
                        onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                        className="px-2 py-1 pr-8 text-sm border border-input bg-background text-foreground rounded focus:outline-none focus:ring-1 focus:ring-ring focus:border-transparent w-48 min-w-0"
                      />
                      {(searchTerm || activeSearchTerm) && (
                        <button
                          onClick={handleClearSearch}
                          className="absolute right-2 top-1/2 transform -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
                          title="Clear search"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                          </svg>
                        </button>
                      )}
                    </div>
                    <button
                      onClick={handleSearch}
                      disabled={isSearching}
                      className="px-2 py-1 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50 disabled:cursor-not-allowed flex-shrink-0"
                    >
                      {isSearching ? 'Searching...' : 'Search'}
                    </button>
                  </div>
                </div>
                
                {/* Spacer to push right content to the right */}
                <div className="flex-1"></div>
              </div>
              
              {/* Scroll and Collapse Links */}
              {outOfViewExpandedFile && (
                <div className="flex gap-2 sm:gap-4 items-center flex-shrink-0">
                  <button
                    onClick={handleScrollToTop}
                    className="text-xs sm:text-sm text-primary hover:text-primary/80 underline transition-colors"
                    title={`Scroll to ${outOfViewExpandedFile}`}
                  >
                    scroll to row
                  </button>
                  <button
                    onClick={handleCollapseExpanded}
                    className="text-xs sm:text-sm text-primary hover:text-primary/80 underline transition-colors"
                    title={`Collapse ${outOfViewExpandedFile}`}
                  >
                    collapse
                  </button>
                </div>
              )}
              
              <div className="flex gap-2 sm:gap-4 items-center flex-shrink-0">
                {currentProcessingFile && (
                  <div className="text-xs sm:text-sm text-primary font-medium hidden sm:block">
                    Processing: {currentProcessingFile.video_path.split('/').pop()} ({currentProcessingFile.process_type})
                  </div>
                )}

              </div>
            </div>
          </div>
        </div>
      )}

      {/* Main content with top padding to account for fixed header */}
      <div className={`${!isSmallScreen ? 'flex h-screen' : 'px-0 py-4'}`}>
        {/* Left Pane - File List and Filters */}
        <div 
          ref={leftPaneRef}
          className={`${!isSmallScreen ? `w-1/2 overflow-y-auto scrollbar-hide px-2 sm:px-4 ${watchDirectory ? 'pt-20 pb-4' : 'py-10'}` : 'w-full'} relative`}
        >  
          {/* Filters */}
        {(availableWatchDirs.length > 1 || availableSources.length > 1) && (
          <div className={`mb-6 flex items-center gap-4 ${shouldUseMobileView ? 'px-4' : ''}`}>
            {/* Watch Directory Filter */}
            {availableWatchDirs.length > 1 && (
              <DropdownMenu modal={false}>
                <DropdownMenuTrigger asChild>
                  <button className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50">
                    {selectedWatchDirs.length === availableWatchDirs.length 
                      ? "All Directories" 
                      : selectedWatchDirs.length === 0 
                      ? "No Directories" 
                      : `${selectedWatchDirs.length} Director${selectedWatchDirs.length === 1 ? 'y' : 'ies'}`}
                    <svg className="ml-2 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                    </svg>
                  </button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="start" className="w-80">
                  <DropdownMenuLabel>Watch Directories</DropdownMenuLabel>
                  <DropdownMenuSeparator />
                  <div className="grid grid-cols-1 gap-1 p-2">
                    <DropdownMenuCheckboxItem
                      checked={selectedWatchDirs.length === availableWatchDirs.length}
                      onCheckedChange={(checked) => {
                        if (checked) {
                          handleSelectAllWatchDirs()
                        } else {
                          handleDeselectAllWatchDirs()
                        }
                      }}
                      className="font-medium"
                    >
                      Select All
                    </DropdownMenuCheckboxItem>
                    <DropdownMenuSeparator />
                    {availableWatchDirs.map(dir => (
                      <DropdownMenuCheckboxItem
                        key={dir}
                        checked={selectedWatchDirs.includes(dir)}
                        onCheckedChange={() => handleWatchDirToggle(dir)}
                      >
                        <div className="flex flex-col items-start">
                          <span className="font-medium text-sm">
                            {dir.split('/').pop() || dir}
                          </span>
                          <span className="text-xs text-muted-foreground">
                            {dir}
                          </span>
                        </div>
                      </DropdownMenuCheckboxItem>
                    ))}
                  </div>
                </DropdownMenuContent>
              </DropdownMenu>
            )}

            {/* Source Filter */}
            {availableSources.length > 1 && (
              <DropdownMenu modal={false}>
                <DropdownMenuTrigger asChild>
                  <button className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50">
                    {selectedSources.length === availableSources.length 
                      ? "All Transcript Sources" 
                      : selectedSources.length === 0 
                      ? "No Transcript Sources" 
                      : `${selectedSources.length} Source${selectedSources.length === 1 ? '' : 's'}`}
                    <svg className="ml-2 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                    </svg>
                  </button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="start" className="w-80">
                  <DropdownMenuLabel>Sources</DropdownMenuLabel>
                  <DropdownMenuSeparator />
                  <div className="grid grid-cols-1 gap-1 p-2">
                    <DropdownMenuCheckboxItem
                      checked={selectedSources.length === availableSources.length}
                      onCheckedChange={(checked) => {
                        if (checked) {
                          handleSelectAllSources()
                        } else {
                          handleDeselectAllSources()
                        }
                      }}
                      className="font-medium"
                    >
                      Select All
                    </DropdownMenuCheckboxItem>
                    <DropdownMenuSeparator />
                    {availableSources.map(source => (
                      <DropdownMenuCheckboxItem
                        key={source}
                        checked={selectedSources.includes(source)}
                        onCheckedChange={() => handleSourceToggle(source)}
                      >
                        <div className="flex items-center">
                          <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${getModelChipColor(source)}`}>
                            {source}
                          </span>
                        </div>
                      </DropdownMenuCheckboxItem>
                    ))}
                  </div>
                </DropdownMenuContent>
              </DropdownMenu>
            )}
          </div>
        )}

        {/* Bulk Regenerate Button */}
        {selectedSources.length === 1 && (
          <div className={`mb-6 flex items-center justify-between ${shouldUseMobileView ? 'px-4' : ''}`}>
            <button
              onClick={handleBulkRegenerate}
              disabled={isBulkRegenerating}
              className="inline-flex items-center px-4 py-2 text-sm font-medium text-white bg-orange-600 hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-orange-500 rounded-md disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isBulkRegenerating ? (
                <>
                  <svg className="w-4 h-4 mr-2 animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                  Regenerating...
                </>
              ) : (
                <>
                  <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                  Regenerate All ({selectedSources[0]})
                </>
              )}
            </button>
            <span className="text-sm text-muted-foreground">
              {(() => {
                const displayedFiles = sortedFiles.filter(file => {
                  if (activeSearchTerm && !searchResults.includes(file.base_name)) {
                    return false
                  }
                  return file.transcript
                })
                return `${displayedFiles.length} file${displayedFiles.length === 1 ? '' : 's'} with transcripts`
              })()}
            </span>
          </div>
        )}

        {/* File List - Full Width */}
        <div>
          {/* Search Results */}
          {searchResults.length > 0 && (
            <div className={`mb-6 p-4 bg-accent/10 border border-accent/20 rounded-md ${shouldUseMobileView ? 'mx-4' : ''}`}>
              <h3 className="text-sm font-medium text-accent-foreground mb-2">
                Found in {searchResults.length} file(s)
              </h3>
            </div>
          )}
          
          {activeSearchTerm && searchResults.length === 0 && !isSearching && (
            <div className={`mb-6 p-4 bg-muted border border-border rounded-md ${shouldUseMobileView ? 'mx-4' : ''}`}>
              <p className="text-sm text-muted-foreground">No files found containing "{activeSearchTerm}"</p>
            </div>
          )}

          {!isSmallScreen && !isLeftPaneWidthMeasured ? (
            // Loading state - don't render anything until width is measured on desktop
            <div className="flex items-center justify-center py-20">
              <div className="text-muted-foreground">Loading...</div>
            </div>
          ) : shouldUseMobileView ? (
            // Mobile/Narrow view - render cards
            <MobileTranscriptList
              sortedFiles={sortedFiles}
              activeSearchTerm={activeSearchTerm}
              searchResults={searchResults}
              transcriptData={transcriptData}
              expandedFiles={expandedFiles}
              regeneratingFiles={regeneratingFiles}
              replacingFiles={replacingFiles}
              searchLineNumbers={searchLineNumbers}
              onExpandFile={(filename) => {
                const file = sortedFiles.find(f => f.base_name === filename)
                if (file?.transcript) {
                  handleExpandFile(filename)
                }
              }}
              onRegenerate={handleRegenerate}
              onReplace={handleReplace}
              onRename={handleRename}
              onRegenerateMeta={handleRegenerateMeta}
              onFetchTranscript={fetchTranscript}
              onSetRightPaneUrl={handleSetRightPaneUrl}
              isFileBeingProcessed={isFileBeingProcessed}
              formatDate={(dateString: string) => formatDate(dateString, leftPaneWidth >= 1129)}
              getModelChipColor={getModelChipColor}
              expandContext={expandContext}
              expandAll={expandAll}
              clipStart={clipStart}
              clipEnd={clipEnd}
              clipTranscript={clipTranscript}
              onSetClipStart={handleSetClipStart}
              onSetClipEnd={handleSetClipEnd}
              onClearClip={handleClearClip}

              onClipBlock={handleClipBlock}
            />
          ) : (
            // Desktop view - render table
            <Table className="table-fixed w-full">
              <TableHeader>
                <TableRow>
                  <TableHead 
                    className="text-center w-[34%] cursor-pointer hover:bg-accent transition-colors"
                    onClick={() => handleSort('name')}
                  >
                    <div className="flex items-center justify-center gap-1">
                      Filename
                      {getSortIndicator('name')}
                    </div>
                    {activeSearchTerm && searchResults.length > 0 && (
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
                    Generated
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
                  className="text-center w-[8%] cursor-pointer hover:bg-accent transition-colors"
                  onClick={() => handleSort('model')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Source
                    {getSortIndicator('model')}
                  </div>
                </TableHead>
                <TableHead className="text-center w-[10%]">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {sortedFiles.map((file) => {
                if (activeSearchTerm != '' && !searchResults.includes(file.base_name)) {
                  return <></>;
                }
                
                const transcriptInfo = transcriptData[file.base_name] || { text: '', loading: false, error: null }
                
                return (
                <>
                <TableRow 
                  key={file.base_name} 
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
                  <TableCell className="font-medium w-[34%] max-w-0">
                    <div 
                      className="leading-tight overflow-hidden whitespace-nowrap text-ellipsis"
                      style={{ 
                        direction: 'rtl', 
                        textAlign: 'left',
                        unicodeBidi: 'plaintext',
                        fontSize: (() => {
                          const filename = file.name.split('/').pop()?.split('\\').pop() || file.name;
                          const length = filename.length;
                          if (length <= 15) return '14px';
                          if (length <= 20) return '13px';
                          return '12px';
                        })()
                      }}
                      title={file.name}
                    >
                      {file.name.split('/').pop()?.split('\\').pop() || file.name}
                    </div>
                  </TableCell>
                  <TableCell className="w-[14%] pr-10 text-foreground">{formatDate(file.created_at, leftPaneWidth >= 1129)}</TableCell>
                  <TableCell className="w-[14%] text-center text-foreground">{formatDate(file.last_generated || '', leftPaneWidth >= 1129)}</TableCell>
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
                  <TableCell className="w-[8%] text-center">
                    {file.model ? (
                      leftPaneWidth >= 1129 ? (
                        <div className="flex justify-center">
                          <span className={`inline-flex items-center rounded-full font-medium ${getModelChipColor(file.model)} ${
                            file.model.length > 10 
                              ? 'px-1.5 py-0.5 text-xs scale-75' 
                              : 'px-2.5 py-0.5 text-xs'
                          }`}>
                            {file.model}
                          </span>
                        </div>
                      ) : (
                        <div className="flex justify-center">
                          <div 
                            className={`w-3 h-3 rounded-full ${getModelChipColor(file.model).split(' ').find(cls => cls.startsWith('dark:bg-')) || 'bg-gray-200'}`}
                            title={file.model}
                          />
                        </div>
                      )
                    ) : (
                      <span className="text-muted-foreground">-</span>
                    )}
                  </TableCell>
                                    <TableCell className="w-[10%] text-center p-2">
                    <DropdownMenu modal={false}>
                      <DropdownMenuTrigger asChild>
                        <button
                          onClick={(e) => e.stopPropagation()}
                          className="p-2 text-muted-foreground hover:text-primary hover:bg-accent rounded-md transition-colors"
                          title="Actions"
                        >
                          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                          </svg>
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        {/* Only show regenerate option if transcript exists */}
                        {file.transcript && (
                          <DropdownMenuItem
                            onClick={(e) => handleRegenerate(file.base_name, e)}
                            disabled={regeneratingFiles.has(file.base_name)}
                          >
                            <span>Regenerate transcript</span>
                            {regeneratingFiles.has(file.base_name) ? (
                              <svg className="w-4 h-4 ml-auto animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                              </svg>
                            ) : (
                              <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                              </svg>
                            )}
                          </DropdownMenuItem>
                        )}
                        
                        {/* Only show edit option if transcript exists */}
                        {file.transcript && (
                          <DropdownMenuItem
                            onClick={(e) => handleReplace(file.base_name, e)}
                            disabled={replacingFiles.has(file.base_name)}
                            variant="destructive"
                          >
                            <span>Edit transcript</span>
                            {!replacingFiles.has(file.base_name) && (
                              <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                              </svg>
                            )}
                          </DropdownMenuItem>
                        )}
                        
                        {/* Rename option */}
                        <DropdownMenuItem
                          onClick={(e) => handleRename(file.base_name, e)}
                          disabled={isFileBeingProcessed(file.base_name) || regeneratingFiles.has(file.base_name) || replacingFiles.has(file.base_name)}
                        >
                          <span>Rename</span>
                          <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                          </svg>
                        </DropdownMenuItem>
                        
                        {/* Show processing status if file is being processed */}
                        {isFileBeingProcessed(file.base_name) && (
                          <DropdownMenuItem disabled>
                            <span>Processing transcript...</span>
                            <svg className="w-4 h-4 ml-auto animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                            </svg>
                          </DropdownMenuItem>
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
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
                    searchTerm={activeSearchTerm}
                    text={transcriptInfo.text}
                    loading={transcriptInfo.loading}
                    error={transcriptInfo.error}
                    visibleLines={searchLineNumbers[file.base_name] || []}
                    expandContext={expandContext}
                    expandAll={expandAll}
                    onEditSuccess={() => { fetchTranscript(file.base_name) }}
                    onSetRightPaneUrl={handleSetRightPaneUrl}
                    clipStart={clipStart}
                    clipEnd={clipEnd}
                    clipTranscript={clipTranscript}
                    onSetClipStart={(time) => handleSetClipStart(time, file.base_name)}
                    onSetClipEnd={(time) => handleSetClipEnd(time, file.base_name)}
                    onClearClip={handleClearClip}

                    onClipBlock={(startTime, endTime) => handleClipBlock(startTime, endTime, file.base_name)}
                  />
                </TableCell>
                </TableRow>
                </>
              )})}
            </TableBody>
          </Table>
          )}
        </div>
        </div>
        
        {/* Right Pane - Always visible on desktop */}
        {!isSmallScreen && (
          <div className="w-1/2 border-l border-border flex flex-col scrollbar-hide">
            {rightPaneUrl ? (
              <iframe
                src={rightPaneUrl}
                className="w-full flex-1 scrollbar-hide"
                title="Right Pane Content"
                frameBorder="0"
                sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-downloads"
              />
            ) : (
              <RightPanePlaceholder />
            )}
          </div>
        )}
      </div>

      {/* Replace Transcript Dialog */}
      <DualEditDialog
        isOpen={isReplaceDialogOpen}
        filename={replaceTranscriptFilename}
        transcriptInitialValue={replaceTranscriptInitialContent}
        metaInitialValue=""
        onTranscriptSave={handleReplaceTranscript}
        onMetaSave={() => {}}
        onCancel={handleReplaceCancel}
        isTranscriptSubmitting={isReplacingTranscript}
      />

      {/* Rename Dialog */}
      {isRenameDialogOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-background border border-border rounded-lg p-6 w-96 max-w-90vw">
            <h2 className="text-lg font-semibold mb-4">Rename File</h2>
            <p className="text-sm text-muted-foreground mb-4">
              Renaming "{renameFilename.split('/').pop()?.split('\\').pop() || renameFilename}" (this will rename the video, transcript, and meta files)
            </p>
            
            <div className="space-y-4">
              <div>
                <label htmlFor="new-filename" className="block text-sm font-medium mb-2">
                  New filename (without extension):
                </label>
                <input
                  id="new-filename"
                  type="text"
                  value={newFilename}
                  onChange={(e) => setNewFilename(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !isRenaming && newFilename.trim()) {
                      handleRenameSubmit()
                    } else if (e.key === 'Escape') {
                      handleRenameCancel()
                    }
                  }}
                  className="w-full px-3 py-2 border border-input bg-background text-foreground rounded focus:outline-none focus:ring-1 focus:ring-ring focus:border-transparent"
                  placeholder="Enter new filename"
                  disabled={isRenaming}
                  autoFocus
                />
              </div>
              
              {renameError && (
                <div className="text-sm text-destructive">
                  {renameError}
                </div>
              )}
              
              <div className="flex justify-end gap-3">
                <button
                  onClick={handleRenameCancel}
                  className="px-4 py-2 text-sm border border-input bg-background text-foreground rounded hover:bg-accent focus:outline-none focus:ring-1 focus:ring-ring"
                  disabled={isRenaming}
                >
                  Cancel
                </button>
                <button
                  onClick={handleRenameSubmit}
                  className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50 disabled:cursor-not-allowed"
                  disabled={isRenaming || !newFilename.trim()}
                >
                  {isRenaming ? 'Renaming...' : 'Rename'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  )
}