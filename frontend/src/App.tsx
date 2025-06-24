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
import { useEffect, useState } from 'react'

function App() {
  // Sample data for the table
  type FileRow = {
    name: string
    created_at: string
    line_count?: number
    length?: string
  }
  const [files, setFiles] = useState<FileRow[]>(window.autotranscript_files as FileRow[])
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set())
  const [searchTerm, setSearchTerm] = useState('')
  const [searchResults, setSearchResults] = useState<string[]>([])
  const [isSearching, setIsSearching] = useState(false)
  const [regeneratingFiles, setRegeneratingFiles] = useState<Set<string>>(new Set())
  const [queue, setQueue] = useState<string[]>([])
  const [currentProcessingFile, setCurrentProcessingFile] = useState<string>('')

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
          } else {
            setQueue([])
            setCurrentProcessingFile('')
          }
        }
      } catch (error) {
        console.error('Error polling queue:', error)
      }
    }, 1000)

    return () => clearInterval(interval)
  }, [])

  useEffect(() => {
    // Refresh files list when queue changes or processing completes
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

    refreshFiles()
  }, [currentProcessingFile])

  // Clear search results when search term is empty
  useEffect(() => {
    if (!searchTerm.trim()) {
      setSearchResults([])
      setExpandedFiles(new Set())
    }
  }, [searchTerm])

  const handleSearch = async () => {
    if (!searchTerm.trim()) {
      setSearchResults([])
      setExpandedFiles(new Set())
      return
    }

    setIsSearching(true)
    try {
      const response = await fetch(`/transcripts/grep/${encodeURIComponent(searchTerm.trim())}`)
      if (response.ok) {
        const result = await response.text()
        const files = result.trim() ? result.split('\n') : []
        setSearchResults(files.map(file => file.replace('.txt', '')))
        setExpandedFiles(new Set(files.map(file => file.replace('.txt', ''))))
      } else {
        setSearchResults([])
        setExpandedFiles(new Set())
      }
    } catch (error) {
      console.error('Search error:', error)
      setSearchResults([])
    } finally {
      setIsSearching(false)
    }
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    handleSearch()
  }

  const handleRegenerate = async (filename: string, e: React.MouseEvent) => {
    e.stopPropagation() // Prevent row expansion
    setRegeneratingFiles(prev => new Set(prev).add(filename))
    
    try {
      // Get CSRF token from meta tag
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content')
      
      const response = await fetch(`/transcripts/regenerate/${filename}`, {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json'
        }
      })
      if (response.ok) {
        // Reload the page after successful regeneration
        window.location.reload()
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

  return (
    <div className="container mx-auto py-10">
      <h1 className="text-2xl font-bold mb-6">File List</h1>
      
      {/* Queue Status */}
      {queue.length > 0 && (
        <div className="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-md">
          <div className="flex items-start">
            <svg className="w-5 h-5 animate-spin text-blue-600 mr-2 mt-0.5" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            <div className="flex-1">
              {currentProcessingFile && (
                <p className="text-sm font-medium text-blue-800 mb-1">
                  Currently processing: <span className="font-semibold">{currentProcessingFile}</span>
                </p>
              )}
              <p className="text-sm text-blue-700">
                {queue.filter(file => file !== currentProcessingFile).length > 0 && (
                  <>Up next: {queue.filter(file => file !== currentProcessingFile).join(', ')}</>
                )}
              </p>
            </div>
          </div>
        </div>
      )}
      
      {/* Search Bar */}
      <div className="mb-6">
        <div className="flex gap-2 max-w-md">
          <input
            type="text"
            placeholder="Search in transcripts..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            onKeyPress={handleKeyPress}
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
              Found in {searchResults.length - 1} file(s)
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
            <TableHead className="text-center w-1/5">Filename</TableHead>
            <TableHead className="text-center w-1/5">Date</TableHead>
            <TableHead className="text-center w-1/5">Lines</TableHead>
            <TableHead className="text-center w-1/5">Length</TableHead>
            <TableHead className="text-center w-1/5">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {files.map((file, index) => {
            if (searchTerm != '' && !searchResults.includes(file.name)) {
              return <></>;
            }
            return (
            <>
            <TableRow key={index} onClick={() => {
              setExpandedFiles(prev => {
                const newSet = new Set(prev)
                newSet.has(file.name) ? newSet.delete(file.name) : newSet.add(file.name)
                return newSet
              })
            }}>
              <TableCell className="font-medium w-1/5">
                <a 
                  href={`/transcripts/${file.name}`}
                  className="text-blue-600 hover:text-blue-800 underline"
                  onClick={(e) => e.stopPropagation()}
                >
                  {file.name}
                </a>
              </TableCell>
              <TableCell className="w-1/5">{file.created_at}</TableCell>
              <TableCell className="w-1/5">{file.line_count || 0}</TableCell>
              <TableCell className="w-1/5">{file.length || '0:00'}</TableCell>
              <TableCell className="w-1/5 text-center">
                <button
                  onClick={(e) => handleRegenerate(file.name, e)}
                  disabled={regeneratingFiles.has(file.name)}
                  className="p-2 text-gray-600 hover:text-blue-600 hover:bg-blue-50 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
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
              </TableCell>
            </TableRow>
            <TableRow>
            <TranscriptView
              visible={expandedFiles.has(file.name)}
              name={file.name}
              className="w-full"
            />
            </TableRow>
            </>
          )})}
        </TableBody>
      </Table>
    </div>
  )
}

export default App
