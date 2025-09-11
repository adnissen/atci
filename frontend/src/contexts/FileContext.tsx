import React, { createContext, useContext, useState, useEffect, type ReactNode } from 'react'
import { addTimestamp } from '../lib/utils'
import { useLSState } from '../hooks/useLSState'

type FileRow = {
  name: string
  base_name: string
  created_at: string
  transcript: boolean
  line_count?: number
  length?: string
  full_path: string
  last_generated?: string
  source?: string
}

interface FileContextType {
  files: FileRow[]
  setFiles: (files: FileRow[]) => void
  refreshFiles: (selectedWatchDirs?: string[], selectedSources?: string[]) => Promise<void>
  selectedWatchDirs: string[]
  setSelectedWatchDirs: (dirs: string[] | ((prev: string[]) => string[])) => void
  availableWatchDirs: string[]
  setAvailableWatchDirs: (dirs: string[]) => void
  selectedSources: string[]
  setSelectedSources: (sources: string[] | ((prev: string[]) => string[])) => void
  availableSources: string[]
  setAvailableSources: (sources: string[]) => void
  showAllFiles: boolean
  setShowAllFiles: (show: boolean | ((prev: boolean) => boolean)) => void
  // Pagination
  page: number
  setPage: (page: number | ((prev: number) => number)) => void
  pageSize: number
  setPageSize: (size: number | ((prev: number) => number)) => void
  // Sorting
  sortColumn: string
  setSortColumn: (column: string | ((prev: string) => string)) => void
  sortDirection: 'asc' | 'desc'
  setSortDirection: (direction: 'asc' | 'desc' | ((prev: 'asc' | 'desc') => 'asc' | 'desc')) => void
  // Pagination metadata from API
  totalPages: number
  totalRecords: number
}

const FileContext = createContext<FileContextType | undefined>(undefined)

export const useFileContext = () => {
  const context = useContext(FileContext)
  if (context === undefined) {
    throw new Error('useFileContext must be used within a FileProvider')
  }
  return context
}

interface FileProviderProps {
  children: ReactNode
}

export const FileProvider: React.FC<FileProviderProps> = ({ children }) => {
  const [files, setFiles] = useState<FileRow[]>([])
  const [selectedWatchDirs, setSelectedWatchDirs] = useLSState<string[]>('selectedWatchDirs', [])
  const [availableWatchDirs, setAvailableWatchDirs] = useState<string[]>([])
  const [selectedSources, setSelectedSources] = useLSState<string[]>('selectedSources', [])
  const [availableSources, setAvailableSources] = useState<string[]>([])
  const [showAllFiles, setShowAllFiles] = useLSState<boolean>('showAllFiles', false)
  
  // Pagination state
  const [page, setPage] = useLSState<number>('filePage', 0)
  const [pageSize, setPageSize] = useLSState<number>('filePageSize', 25)
  
  // Sorting state
  const [sortColumn, setSortColumn] = useLSState<string>('fileSortColumn', 'created_at')
  const [sortDirection, setSortDirection] = useLSState<'asc' | 'desc'>('fileSortDirection', 'desc')
  
  // Pagination metadata from API
  const [totalPages, setTotalPages] = useState<number>(0)
  const [totalRecords, setTotalRecords] = useState<number>(0)

  const refreshFiles = async (watchDirs?: string[], sources?: string[]) => {
    // Use provided parameters or fall back to context state
    const dirsToUse = watchDirs !== undefined ? watchDirs : selectedWatchDirs
    const sourcesToUse = sources !== undefined ? sources : selectedSources
    
    try {
      const params = new URLSearchParams()
      
      // Add filter parameters
      const filters = [...dirsToUse, ...sourcesToUse].filter(Boolean)
      if (filters.length > 0) {
        params.append('filter', filters.join(','))
      }
      
      // Add pagination and sorting parameters
      params.append('page', page.toString())
      params.append('page_size', pageSize.toString())
      params.append('sort_by', sortColumn)
      params.append('sort_order', sortDirection === 'asc' ? '0' : '1')

      const url = `/api/files?${params.toString()}`
      
      const response = await fetch(addTimestamp(url))
      if (response.ok) {
        const data = await response.json()
        if (data.success) {
          // Check if we got pagination metadata (from paginated endpoint)
          if (data.data && typeof data.data === 'object' && data.data.files) {
            // Paginated response structure
            setFiles(data.data.files || [])
            setTotalPages(data.data.pages || 0)
            setTotalRecords(data.data.total_records || 0)
          } else {
            // Non-paginated response structure (just array of files)
            setFiles(data.data || [])
            setTotalPages(0)
            setTotalRecords(0)
          }
        }
      }
    } catch (error) {
      console.error('Error refreshing files:', error)
    }
  }

  // Fetch configured watch directories from the API
  useEffect(() => {
    const fetchWatchDirectories = async () => {
      try {
        const response = await fetch(addTimestamp('/api/config'))
        if (response.ok) {
          const config = await response.json()
          if (config.success) {
            setAvailableWatchDirs(config.data.config.watch_directories || [])
          }
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
        const response = await fetch(addTimestamp('/api/sources'))
        if (response.ok) {
          const data = await response.json()
          if (data.success) {
            setAvailableSources(data.data || [])
          }
        }
      } catch (error) {
        console.error('Error fetching sources:', error)
      }
    }
    
    fetchSources()
  }, [])

  // Refresh files when filters, pagination, or sorting change, but only if showAllFiles is enabled
  useEffect(() => {
    if (showAllFiles) {
      refreshFiles()
    }
  }, [selectedWatchDirs, selectedSources, showAllFiles, page, pageSize, sortColumn, sortDirection])

  const value: FileContextType = {
    files,
    setFiles,
    refreshFiles,
    selectedWatchDirs,
    setSelectedWatchDirs,
    availableWatchDirs,
    setAvailableWatchDirs,
    selectedSources,
    setSelectedSources,
    availableSources,
    setAvailableSources,
    showAllFiles,
    setShowAllFiles,
    // Pagination
    page,
    setPage,
    pageSize,
    setPageSize,
    // Sorting
    sortColumn,
    setSortColumn,
    sortDirection,
    setSortDirection,
    // Pagination metadata
    totalPages,
    totalRecords,
  }

  return <FileContext.Provider value={value}>{children}</FileContext.Provider>
}