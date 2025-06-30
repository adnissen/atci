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
import ConfigurationForm from './components/ConfigurationForm'
import { useEffect, useState } from 'react'

function App() {
  // Sample data for the table
  type FileRow = {
    name: string
    created_at: string
    transcript: boolean
    line_count?: number
    length?: string
    full_path?: string
    last_generated?: string
  }
  
  type TranscriptData = {
    text: string
    loading: boolean
    error: string | null
  }
  
  type SortColumn = 'created_at' | 'last_generated' | 'name' | 'line_count' | 'length'
  type SortDirection = 'asc' | 'desc'
  
  const [files, setFiles] = useState<FileRow[]>(window.autotranscript_files as FileRow[])
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set())
  const [searchTerm, setSearchTerm] = useState('')
  const [searchLineNumbers, setSearchLineNumbers] = useState<Record<string, number[]>>({})
  const [isSearching, setIsSearching] = useState(false)
  const [regeneratingFiles, setRegeneratingFiles] = useState<Set<string>>(new Set())
  const [queue, setQueue] = useState<string[]>([])
  const [currentProcessingFile, setCurrentProcessingFile] = useState<string>('')
  const [watchDirectory, setWatchDirectory] = useState<string>('')
  const [replacingFiles, setReplacingFiles] = useState<Set<string>>(new Set())
  const [transcriptData, setTranscriptData] = useState<Record<string, TranscriptData>>({})
  const [sortColumn, setSortColumn] = useState<SortColumn>('created_at')
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc')
  const [configValid, setConfigValid] = useState<boolean | null>(null) // null = loading, false = invalid, true = valid
  const [showConfigForm, setShowConfigForm] = useState(false)

  // Calculate search results from search line numbers
  const searchResults = Object.keys(searchLineNumbers).filter(filename => 
    searchLineNumbers[filename] && searchLineNumbers[filename].length > 0
  )

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
        <svg className="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4" />
        </svg>
      )
    }
    
    if (sortDirection === 'asc') {
      return (
        <svg className="w-4 h-4 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
        </svg>
      )
    } else {
      return (
        <svg className="w-4 h-4 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      )
    }
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
        const response = await fetch('/api/config/status')
        if (response.ok) {
          const configData = await response.json()
          setConfigValid(configData.valid)
          
          if (configData.valid) {
            // If config is valid, fetch watch directory for display
            setWatchDirectory(configData.config.watch_directory || '')
          } else {
            setShowConfigForm(true)
          }
        } else {
          console.error('Failed to fetch configuration status')
          setConfigValid(false)
          setShowConfigForm(true)
        }
      } catch (error) {
        console.error('Error checking configuration:', error)
        setConfigValid(false)
        setShowConfigForm(true)
      }
    }

    checkConfiguration()
  }, [])

  // Handle configuration completion
  const handleConfigComplete = () => {
    setShowConfigForm(false)
    setConfigValid(true)
    // Reload the page to reinitialize with new config
    window.location.reload()
  }

  // Poll /queue endpoint every second
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const response = await fetch('/queue')
        if (response.ok) {
          const queueData = await response.json()
          if (queueData.queue && queueData.queue.length > 0) {
            setCurrentProcessingFile(queueData.current_file || '')
            setQueue(queueData.queue)
            setRegeneratingFiles(new Set(queue))
          } else {
            setQueue([])
            setCurrentProcessingFile('')
            setRegeneratingFiles(new Set())
          }
        }
      } catch (error) {
        console.error('Error polling queue:', error)
      }
    }, 1000)

    return () => clearInterval(interval)
  }, [])

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

  // Clear search results when search term is empty
  useEffect(() => {
    if (!searchTerm.trim()) {
      setSearchLineNumbers({})
      setExpandedFiles(new Set())
    } else {
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

  const handleKeyPress = () => {
    handleSearch()
  }

  const handleRegenerate = async (filename: string, e: React.MouseEvent) => {
    e.stopPropagation() // Prevent row expansion
    setRegeneratingFiles(prev => new Set(prev).add(filename))
    
    try {
      // Get CSRF token from meta tag
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content')
      
      const response = await fetch(`/transcripts/${filename}/regenerate`, {
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
    
    // Prompt user for new content
    const newContent = prompt(`Enter new content for ${filename}:`)
    if (!newContent) return // User cancelled
    
    setReplacingFiles(prev => new Set(prev).add(filename))
    
    try {
      // Get CSRF token from meta tag
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content')
      
      const response = await fetch(`/transcripts/${filename}/replace`, {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ text: newContent })
      })
      
      if (response.ok) {
        // Update the transcript data with the new content
        setTranscriptData(prev => ({
          ...prev,
          [filename]: { text: newContent, loading: false, error: null }
        }))
        
        // Force a refresh of the files list to update any metadata
        const filesResponse = await fetch('/files')
        if (filesResponse.ok) {
          const filesData = await filesResponse.json()
          setFiles(filesData)
        }
      } else {
        console.error('Failed to replace transcript')
        alert('Failed to replace transcript. Please try again.')
        setReplacingFiles(prev => {
          const newSet = new Set(prev)
          newSet.delete(filename)
          return newSet
        })
      }
    } catch (error) {
      console.error('Replacement error:', error)
      alert('Error replacing transcript. Please try again.')
      setReplacingFiles(prev => {
        const newSet = new Set(prev)
        newSet.delete(filename)
        return newSet
      })
    } finally {
      setReplacingFiles(prev => {
        const newSet = new Set(prev)
        newSet.delete(filename)
        return newSet
      })
    }
  }

  // Show loading state while checking configuration
  if (configValid === null) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading...</p>
        </div>
      </div>
    )
  }

  // Show configuration form if configuration is invalid
  if (!configValid || showConfigForm) {
    return <ConfigurationForm onConfigComplete={handleConfigComplete} />
  }

  return (
    <div className="min-h-screen">
      {/* Watch Directory Bar - Fixed to top */}
      {watchDirectory && (
        <div className="fixed top-0 left-0 right-0 bg-gray-100 border-b border-gray-200 px-4 py-2 z-10">
          <div className="container mx-auto">
            <div className="flex gap-6 justify-between items-center">
              <div className="flex-1">
                <div className="flex gap-2 max-w-md">
                  <div className="text-sm text-gray-700 font-medium text-left">
                    Directory: {watchDirectory}
                  </div>
                </div>
              </div>
              <div className="flex gap-4 items-center">
                {currentProcessingFile && (
                  <div className="text-sm text-blue-700 font-medium">
                    Processing: {currentProcessingFile}
                  </div>
                )}
                <div className="relative group">
                  <div className="text-sm text-gray-700 font-medium">
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
        <h1 className="text-2xl font-bold mb-6">File List</h1>
        
        {/* File List - Full Width */}
        <div>
          {/* Search Bar */}
          <div className="mb-6">
            <div className="flex gap-2 max-w-md">
              <input
                type="text"
                placeholder="Search in transcripts..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
              <button
                onClick={handleSearch}
                disabled={isSearching}
                className="px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSearching ? 'Searching...' : 'Search'}
              </button>
            </div>
            
            {/* Search Results */}
            {searchResults.length > 0 && (
              <div className="mt-4 p-4 bg-green-50 border border-green-200 rounded-md">
                <h3 className="text-sm font-medium text-green-800 mb-2">
                  Found in {searchResults.length} file(s)
                </h3>
              </div>
            )}
            
            {searchTerm && searchResults.length === 0 && !isSearching && (
              <div className="mt-4 p-4 bg-gray-50 border border-gray-200 rounded-md">
                <p className="text-sm text-gray-600">No files found containing "{searchTerm}"</p>
              </div>
            )}
          </div>

          <Table className="table-fixed w-full">
            <TableHeader>
              <TableRow>
                <TableHead 
                  className="text-center w-1/6 cursor-pointer hover:bg-gray-50 transition-colors"
                  onClick={() => handleSort('name')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Filename
                    {getSortIndicator('name')}
                  </div>
                  {searchTerm && searchResults.length > 0 && (
                    <span className="text-xs text-green-600 ml-2">(Search Results)</span>
                  )}
                </TableHead>
                <TableHead 
                  className="text-center w-1/6 cursor-pointer hover:bg-gray-50 transition-colors"
                  onClick={() => handleSort('created_at')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Date
                    {getSortIndicator('created_at')}
                  </div>
                </TableHead>
                <TableHead 
                  className="text-center w-1/6 pl-14 cursor-pointer hover:bg-gray-50 transition-colors"
                  onClick={() => handleSort('last_generated')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Last Generated
                    {getSortIndicator('last_generated')}
                  </div>
                </TableHead>
                <TableHead 
                  className="text-center w-1/6 cursor-pointer hover:bg-gray-50 transition-colors"
                  onClick={() => handleSort('line_count')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Lines
                    {getSortIndicator('line_count')}
                  </div>
                </TableHead>
                <TableHead 
                  className="text-center w-1/6 cursor-pointer hover:bg-gray-50 transition-colors"
                  onClick={() => handleSort('length')}
                >
                  <div className="flex items-center justify-center gap-1">
                    Length
                    {getSortIndicator('length')}
                  </div>
                </TableHead>
                <TableHead className="text-center w-1/6">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {sortedFiles.map((file, index) => {
                if (searchTerm != '' && !searchResults.includes(file.name)) {
                  return <></>;
                }
                
                const transcriptInfo = transcriptData[file.name] || { text: '', loading: false, error: null }
                
                return (
                <>
                <TableRow key={index} onClick={() => {
                  // Only expand if transcript exists
                  if (file.transcript) {
                    setExpandedFiles(prev => {
                      const newSet = new Set(prev)
                      if (newSet.has(file.name)) {
                        newSet.delete(file.name)
                      } else {
                        newSet.add(file.name)
                      }
                      return newSet
                    })
                  }
                }}>
                  <TableCell className="font-medium w-1/6">
                    <a 
                      href={`/player/${file.name}`}
                      className="text-blue-600 hover:text-blue-800 underline"
                      onClick={(e) => e.stopPropagation()}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      {file.name}
                    </a>
                  </TableCell>
                  <TableCell className="w-1/6 pr-10">{formatDate(file.created_at)}</TableCell>
                  <TableCell className="w-1/6 pl-10">{formatDate(file.last_generated || '')}</TableCell>
                  <TableCell className="w-1/6">{file.line_count || 0}</TableCell>
                  <TableCell className="w-1/6">{file.length || '0:00'}</TableCell>
                  <TableCell className="w-1/6 text-center">
                    <div className="flex justify-center gap-2">
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          window.open(`/player/${file.name}`, '_blank')
                        }}
                        className="p-2 text-gray-600 hover:text-green-600 hover:bg-green-50 rounded-md transition-colors"
                        title="View file"
                      >
                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                        </svg>
                      </button>
                      
                      {/* Warning icon for files without transcript */}
                      {!file.transcript && !currentProcessingFile.includes(file.name) && (
                        <div className="p-2 text-red-600" title="No transcript available">
                          <svg className="w-5 h-5" fill="none" stroke="red" strokeWidth="2" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                          </svg>
                        </div>
                      )}
                      
                      {/* Only show regenerate button if transcript exists */}
                      {file.transcript ? (
                        <>
                          <button
                            onClick={(e) => handleRegenerate(file.name, e)}
                            disabled={regeneratingFiles.has(file.name)}
                            className={`p-2 text-gray-600 hover:text-blue-600 hover:bg-blue-50 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${
                              currentProcessingFile.includes(file.name) ? 'animate-spin' : ''
                            }`}
                            title="Regenerate transcript"
                          >
                            {regeneratingFiles.has(file.name) ? (
                              <svg className="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                              </svg>
                            ) : (
                              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                              </svg>
                            )}
                          </button>
                          
                          <button
                            onClick={(e) => handleReplace(file.name, e)}
                            disabled={replacingFiles.has(file.name)}
                            className={`p-2 text-gray-600 hover:text-orange-600 hover:bg-orange-50 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed`}
                            title="Edit transcript"
                          >
                            {replacingFiles.has(file.name) ? (
                              <svg className="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                              </svg>
                            ) : (
                              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                              </svg>
                            )}
                          </button>
                        </>
                      ) : currentProcessingFile.includes(file.name) ? (
                        <button
                          disabled={true}
                          className="p-2 text-gray-600 opacity-50 cursor-not-allowed rounded-md"
                          title="Processing transcript"
                        >
                          <svg className="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                          </svg>
                        </button>
                      ) : null}
                    </div>
                  </TableCell>
                </TableRow>
                <TableRow>
                  
                <TableCell colSpan={6} className="p-0">
                  <TranscriptView
                    visible={expandedFiles.has(file.name)}
                    name={file.name}
                    className="w-full"
                    searchTerm={searchTerm}
                    text={transcriptInfo.text}
                    loading={transcriptInfo.loading}
                    error={transcriptInfo.error}
                    visibleLines={searchLineNumbers[file.name] || []}
                    expandContext={expandContext}
                  />
                </TableCell>
                </TableRow>
                </>
              )})}
            </TableBody>
          </Table>
        </div>
      </div>
    </div>
  )
}

export default App
