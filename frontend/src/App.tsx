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
  const files = window.autotranscript_files as FileRow[]
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set())
  const [searchTerm, setSearchTerm] = useState('')
  const [searchResults, setSearchResults] = useState<string[]>([])
  const [isSearching, setIsSearching] = useState(false)
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

  return (
    <div className="container mx-auto py-10">
      <h1 className="text-2xl font-bold mb-6">File List</h1>
      
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
            <TableHead className="text-center w-1/4">Filename</TableHead>
            <TableHead className="text-center w-1/4">Date</TableHead>
            <TableHead className="text-center w-1/4">Lines</TableHead>
            <TableHead className="text-center w-1/4">Length</TableHead>
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
              <TableCell className="font-medium w-1/4">
                <a 
                  href={`/transcripts/${file.name}`}
                  className="text-blue-600 hover:text-blue-800 underline"
                  onClick={(e) => e.stopPropagation()}
                >
                  {file.name}
                </a>
              </TableCell>
              <TableCell className="w-1/4">{file.created_at}</TableCell>
              <TableCell className="w-1/4">{file.line_count || 0}</TableCell>
              <TableCell className="w-1/4">{file.length || '0:00'}</TableCell>
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
