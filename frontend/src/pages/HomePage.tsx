import TopBar from '../components/TopBar'
import TranscriptList from '../components/TranscriptList'
import RightPanePlaceholder from '../components/RightPanePlaceholder'
import ConfigPage from './ConfigPage'
import QueuePage from './QueuePage'
import { useEffect, useState, useRef, useCallback } from 'react'
import { useLSState } from '../hooks/useLSState'
import { useIsSmallScreen } from '../hooks/useMediaQuery'
import { addTimestamp } from '../lib/utils'

// Type definitions
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

type QueueItem = {
  video_path: string
  process_type: string
}

export default function HomePage() {
  const isSmallScreen = useIsSmallScreen()
  
  const [files, setFiles] = useState<FileRow[]>(window.autotranscript_files as FileRow[])
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set())
  const [searchTerm, setSearchTerm] = useState('')
  const [activeSearchTerm, setActiveSearchTerm] = useState('')
  const [searchLineNumbers, setSearchLineNumbers] = useState<Record<string, number[]>>({})
  const [isSearching, setIsSearching] = useState(false)
  const [regeneratingFiles, setRegeneratingFiles] = useState<Set<string>>(new Set())
  const [queue, setQueue] = useState<QueueItem[]>([])
  const [currentProcessingFile, setCurrentProcessingFile] = useState<QueueItem | null>(null)
  const [watchDirectory, setWatchDirectory] = useState<string>('')
  const [replacingFiles, setReplacingFiles] = useState<Set<string>>(new Set())
  const [transcriptData, setTranscriptData] = useState<Record<string, TranscriptData>>({})
  const [selectedWatchDirs, setSelectedWatchDirs] = useLSState<string[]>('selectedWatchDirs', [])
  const [availableWatchDirs, setAvailableWatchDirs] = useState<string[]>([])
  const [selectedSources, setSelectedSources] = useLSState<string[]>('selectedSources', [])
  const [availableSources, setAvailableSources] = useState<string[]>([])

  // State for tracking out-of-view expanded rows
  const [outOfViewExpandedFile, setOutOfViewExpandedFile] = useState<string | null>(null)
  const [flashingRow, setFlashingRow] = useState<string | null>(null)

  // Right pane URL state
  const [rightPaneUrl, setRightPaneUrl] = useState<string>('')
  const [leftPaneWidth, setLeftPaneWidth] = useState<number>(0)
  const [isLeftPaneWidthMeasured, setIsLeftPaneWidthMeasured] = useState<boolean>(false)
  const [showConfigInRightPane, setShowConfigInRightPane] = useState<boolean>(false)
  const [showQueueInRightPane, setShowQueueInRightPane] = useState<boolean>(true)

  // Clip state variables
  const [clipStart, setClipStart] = useState<number | null>(null)
  const [clipEnd, setClipEnd] = useState<number | null>(null)
  const [clipTranscript, setClipTranscript] = useState<string | null>(null)
  const fileRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const transcriptRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const observerRef = useRef<IntersectionObserver | null>(null)
  const transcriptObserverRef = useRef<IntersectionObserver | null>(null)
  const leftPaneRef = useRef<HTMLDivElement | null>(null)

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
      refreshQueue()
    }, 3000)
    
    return () => clearInterval(interval)
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

  const handleSetRightPaneUrl = useCallback((url: string) => {
    if (isSmallScreen) {
      // On mobile, open URL in new browser window
      window.open(url, '_blank')
    } else {
      // On desktop, set the state variable
      setRightPaneUrl(url)
      setShowConfigInRightPane(false) // Hide config when showing other content
      setShowQueueInRightPane(false) // Hide queue when showing other content
    }
  }, [isSmallScreen])

  const handleConfigClick = () => {
    if (isSmallScreen) {
      // On mobile, navigate to config page
      window.location.href = '/config'
    } else {
      // On desktop, show config in right pane
      setShowConfigInRightPane(true)
      setShowQueueInRightPane(false) // Hide queue when showing config
      setRightPaneUrl('') // Clear any existing URL
    }
  }

  const handleQueueClick = () => {
    if (isSmallScreen) {
      // On mobile, navigate to queue page
      window.location.href = '/queue'
    } else {
      // On desktop, show queue in right pane
      setShowQueueInRightPane(true)
      setShowConfigInRightPane(false) // Hide config when showing queue
      setRightPaneUrl('') // Clear any existing URL
    }
  }

  const handleCloseConfig = () => {
    setShowConfigInRightPane(false)
    
    // If we have clip start and end values, restore the clip player
    if (clipStart !== null && clipEnd !== null && clipTranscript) {
      const url = `/clip_player/${encodeURIComponent(clipTranscript)}?start_time=${clipStart}&end_time=${clipEnd}&display_text=false`
      handleSetRightPaneUrl(url)
    }
  }

  const handleCloseQueue = () => {
    setShowQueueInRightPane(false)
    
    // If we have clip start and end values, restore the clip player
    if (clipStart !== null && clipEnd !== null && clipTranscript) {
      const url = `/clip_player/${encodeURIComponent(clipTranscript)}?start_time=${clipStart}&end_time=${clipEnd}&display_text=false`
      handleSetRightPaneUrl(url)
    }
  }

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
    setShowConfigInRightPane(false)
    setShowQueueInRightPane(false)
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

  // Make the function available globally for testing
  useEffect(() => {
    if (typeof window !== 'undefined') {
      (window as any).handleSetRightPaneUrl = handleSetRightPaneUrl
    }
  }, [handleSetRightPaneUrl])

  return (
    <>
      <TopBar
        watchDirectory={watchDirectory}
        searchTerm={searchTerm}
        setSearchTerm={setSearchTerm}
        activeSearchTerm={activeSearchTerm}
        setActiveSearchTerm={setActiveSearchTerm}
        setSearchLineNumbers={setSearchLineNumbers}
        setExpandedFiles={setExpandedFiles}
        isSearching={isSearching}
        queue={queue}
        currentProcessingFile={currentProcessingFile}
        outOfViewExpandedFile={outOfViewExpandedFile}
        onSearch={handleSearch}
        onClearSearch={handleClearSearch}
        onScrollToTop={handleScrollToTop}
        onCollapseExpanded={handleCollapseExpanded}
        onConfigClick={handleConfigClick}
        onQueueClick={handleQueueClick}
      />

      {/* Main content with top padding to account for fixed header */}
      <div className={`${!isSmallScreen ? 'flex h-screen' : 'px-0 py-4'}`}>
        {/* Left Pane - TranscriptList */}
        <TranscriptList
          watchDirectory={watchDirectory}
          isSmallScreen={isSmallScreen}
          files={files}
          setFiles={setFiles}
          activeSearchTerm={activeSearchTerm}
          searchLineNumbers={searchLineNumbers}
          setSearchLineNumbers={setSearchLineNumbers}
          expandedFiles={expandedFiles}
          setExpandedFiles={setExpandedFiles}
          regeneratingFiles={regeneratingFiles}
          setRegeneratingFiles={setRegeneratingFiles}
          replacingFiles={replacingFiles}
          setReplacingFiles={setReplacingFiles}
          transcriptData={transcriptData}
          setTranscriptData={setTranscriptData}
          currentProcessingFile={currentProcessingFile}
          selectedWatchDirs={selectedWatchDirs}
          setSelectedWatchDirs={setSelectedWatchDirs}
          availableWatchDirs={availableWatchDirs}
          setAvailableWatchDirs={setAvailableWatchDirs}
          selectedSources={selectedSources}
          setSelectedSources={setSelectedSources}
          availableSources={availableSources}
          setAvailableSources={setAvailableSources}
          flashingRow={flashingRow}
          leftPaneWidth={leftPaneWidth}
          setLeftPaneWidth={setLeftPaneWidth}
          isLeftPaneWidthMeasured={isLeftPaneWidthMeasured}
          setIsLeftPaneWidthMeasured={setIsLeftPaneWidthMeasured}
          clipStart={clipStart}
          clipEnd={clipEnd}
          clipTranscript={clipTranscript}
          fileRowRefs={fileRowRefs}
          transcriptRowRefs={transcriptRowRefs}
          leftPaneRef={leftPaneRef}
          onSetRightPaneUrl={handleSetRightPaneUrl}
          onSetClipStart={handleSetClipStart}
          onSetClipEnd={handleSetClipEnd}
          onClearClip={handleClearClip}
          onClipBlock={handleClipBlock}
        />
        
        {/* Right Pane - Always visible on desktop */}
        {!isSmallScreen && (
          <div className="w-1/2 border-l border-border flex flex-col scrollbar-hide">
            {showConfigInRightPane ? (
              <ConfigPage onClose={handleCloseConfig} />
            ) : showQueueInRightPane ? (
              <QueuePage onClose={handleCloseQueue} />
            ) : rightPaneUrl ? (
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
    </>
  )
}