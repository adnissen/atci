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

  const refreshFiles = async (watchDirs?: string[], sources?: string[]) => {
    // Use provided parameters or fall back to context state
    const dirsToUse = watchDirs !== undefined ? watchDirs : selectedWatchDirs
    const sourcesToUse = sources !== undefined ? sources : selectedSources
    
    try {
      const params = new URLSearchParams()
      params.append('filter', dirsToUse.join(','))
      params.append('sources', sourcesToUse.join(','))

      const queryString = params.toString()
      const url = queryString ? `/api/files?${queryString}` : '/api/files'
      
      const response = await fetch(addTimestamp(url))
      if (response.ok) {
        const data = await response.json()
        if (data.success) {
          setFiles(data.data || [])
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

  // Refresh files when filters change, but only if showAllFiles is enabled
  useEffect(() => {
    if (showAllFiles) {
      refreshFiles()
    }
  }, [selectedWatchDirs, selectedSources, showAllFiles])

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
  }

  return <FileContext.Provider value={value}>{children}</FileContext.Provider>
}