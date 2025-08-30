import React, { createContext, useContext, useState, type ReactNode } from 'react'
import { addTimestamp } from '../lib/utils'

type FileRow = {
  name: string
  base_name: string
  created_at: string
  transcript: boolean
  line_count?: number
  length?: string
  full_path: string
  last_generated?: string
  model?: string
}

interface FileContextType {
  files: FileRow[]
  setFiles: (files: FileRow[]) => void
  refreshFiles: (selectedWatchDirs: string[], selectedSources: string[]) => Promise<void>
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

  const refreshFiles = async (selectedWatchDirs: string[], selectedSources: string[]) => {
    try {
      const params = new URLSearchParams()
      params.append('filter', selectedWatchDirs.join(','))
      params.append('sources', selectedSources.join(','))

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

  const value: FileContextType = {
    files,
    setFiles,
    refreshFiles,
  }

  return <FileContext.Provider value={value}>{children}</FileContext.Provider>
}