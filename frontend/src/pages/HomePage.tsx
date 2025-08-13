import TopBar from '../components/TopBar'
import TranscriptList from '../components/TranscriptList'
import RightPanePlaceholder from '../components/RightPanePlaceholder'
import ClipPlayer from '../components/ClipPlayer'
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


  const [isAtTop, setIsAtTop] = useState<boolean>(true)


  // Right pane component state
  const [rightPaneComponent, setRightPaneComponent] = useState<React.ReactNode | null>(null)
  const [leftPaneWidth, setLeftPaneWidth] = useState<number>(0)
  const [isLeftPaneWidthMeasured, setIsLeftPaneWidthMeasured] = useState<boolean>(false)
  const [showConfigInRightPane, setShowConfigInRightPane] = useState<boolean>(false)
  const [showQueueInRightPane, setShowQueueInRightPane] = useState<boolean>(true)

  // Mobile clip player state
  const [mobileClipPlayerComponent, setMobileClipPlayerComponent] = useState<React.ReactNode | null>(null)

  // Mobile config and queue state
  const [mobileConfigComponent, setMobileConfigComponent] = useState<React.ReactNode | null>(null)
  const [mobileQueueComponent, setMobileQueueComponent] = useState<React.ReactNode | null>(null)

  // Clip state variables
  const [clipStart, setClipStart] = useState<number | null>(null)
  const [clipEnd, setClipEnd] = useState<number | null>(null)
  const [clipTranscript, setClipTranscript] = useState<string | null>(null)
  const fileRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const transcriptRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const mobileTranscriptRowRefs = useRef<Record<string, HTMLDivElement | null>>({})
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

  // Track scroll position of left pane to show/hide scroll to top button
  useEffect(() => {
    const leftPaneContainer = leftPaneRef.current
    if (!leftPaneContainer) return

    const handleScroll = () => {
      const scrollTop = leftPaneContainer.scrollTop
      setIsAtTop(scrollTop <= 10) // Allow small tolerance for "at top"
    }

    leftPaneContainer.addEventListener('scroll', handleScroll)
    
    // Initial check
    handleScroll()

    return () => {
      leftPaneContainer.removeEventListener('scroll', handleScroll)
    }
  }, [])  // Empty dependency array since we want this to run once when component mounts





  // Handle scroll to top
  const handleScrollToTop = () => {
    const leftPaneContainer = leftPaneRef.current
    if (leftPaneContainer) {
      leftPaneContainer.scrollTo({ 
        top: 0, 
        behavior: 'smooth' 
      })
    }
  }

  // Handle collapse all expanded files
  const handleCollapseAll = () => {
    // Scroll to the top of the left pane container
    const leftPaneContainer = leftPaneRef.current
    if (leftPaneContainer) {
      leftPaneContainer.scrollTo({ 
        top: 0, 
        behavior: 'smooth' 
      })
    }
    
    setExpandedFiles(new Set())
  }

  // Handle collapse - find the transcript row closest to the top of the screen and collapse it
  const handleCollapseExpanded = () => {
    const leftPaneContainer = leftPaneRef.current
    if (!leftPaneContainer || expandedFiles.size === 0) {
      return
    }
    
    const containerRect = leftPaneContainer.getBoundingClientRect()
    const topBarHeight = watchDirectory ? 64 : 0
    const viewportTop = containerRect.top + topBarHeight // Account for top bar
    
    let closestFile = null
    let closestDistance = Infinity
    
    // Check all expanded files to find the one with a transcript row closest to the top
    expandedFiles.forEach(filename => {
      const transcriptRow = transcriptRowRefs.current[filename]
      const mobileTranscriptRow = mobileTranscriptRowRefs.current[filename]
      if (transcriptRow) {

        const transcriptRect = transcriptRow.getBoundingClientRect()
        // Use the top edge of the transcript row
        const distanceFromTop = Math.abs(transcriptRect.top - viewportTop)
        
        if (distanceFromTop < closestDistance) {
          closestDistance = distanceFromTop
          closestFile = filename
        }
      } else if (mobileTranscriptRow) {
        const mobileTranscriptRect = mobileTranscriptRow.getBoundingClientRect()
        const distanceFromTop = Math.abs(mobileTranscriptRect.top - viewportTop)

        if (distanceFromTop < closestDistance) {
          closestDistance = distanceFromTop
          closestFile = filename
        }
      }
    })
    
    if (closestFile) {
      const targetFile = closestFile
      
      // First scroll to the file row in the left pane container
      const rowElement = fileRowRefs.current[targetFile] || mobileTranscriptRowRefs.current[targetFile]
      
      if (rowElement && leftPaneContainer) {
        // Calculate the position relative to the scrollable container
        const rowRect = rowElement.getBoundingClientRect()
        const currentScrollTop = leftPaneContainer.scrollTop
        
        // Calculate where to scroll to position the row with top bar offset
        const targetScrollTop = currentScrollTop + (rowRect.top - containerRect.top) - topBarHeight

        leftPaneContainer.scrollTo({ 
          top: targetScrollTop, 
          behavior: 'smooth' 
        })
      }
      
      // Wait for scroll to complete, then collapse the row
      setTimeout(() => {
        setExpandedFiles(prev => {
          const newSet = new Set(prev)
          newSet.delete(targetFile)
          return newSet
        })
      }, 500) // Wait for smooth scroll to complete (typically 300-500ms)
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

  // Helper functions for ClipPlayer
  const secondsToTimestamp = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    const remainingSeconds = seconds % 60
    const wholeSeconds = Math.floor(remainingSeconds)
    const milliseconds = Math.round((remainingSeconds - wholeSeconds) * 1000)
    
    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${wholeSeconds.toString().padStart(2, '0')}.${milliseconds.toString().padStart(3, '0')}`
  }



  const handleSetRightPaneComponent = useCallback((component: React.ReactNode | null, _fallbackUrl?: string) => {
    if (isSmallScreen) {
      if (component) {
        // On mobile, show component inline instead of opening new window
        setMobileClipPlayerComponent(component)
        // Clear other mobile components
        setMobileConfigComponent(null)
        setMobileQueueComponent(null)
      } else {
        // Clear mobile clip player
        setMobileClipPlayerComponent(null)
      }
    } else {
      // On desktop, set the component directly
      setRightPaneComponent(component)
      setShowConfigInRightPane(false) // Hide config when showing other content
      setShowQueueInRightPane(false) // Hide queue when showing other content
    }
  }, [isSmallScreen])

  const handleConfigClick = () => {
    if (isSmallScreen) {
      // On mobile, show config component
      const configComponent = (
        <div className="w-full">
          <ConfigPage onClose={() => setMobileConfigComponent(null)} />
        </div>
      )
      setMobileConfigComponent(configComponent)
      // Clear other mobile components
      setMobileClipPlayerComponent(null)
      setMobileQueueComponent(null)
    } else {
      // On desktop, show config in right pane
      setShowConfigInRightPane(true)
      setShowQueueInRightPane(false) // Hide queue when showing config
      setRightPaneComponent(null) // Clear any existing component
    }
  }

  const handleQueueClick = () => {
    if (isSmallScreen) {
      // On mobile, show queue component
      const queueComponent = (
        <div className="w-full">
          <QueuePage onClose={() => setMobileQueueComponent(null)} />
        </div>
      )
      setMobileQueueComponent(queueComponent)
      // Clear other mobile components
      setMobileClipPlayerComponent(null)
      setMobileConfigComponent(null)
    } else {
      // On desktop, show queue in right pane
      setShowQueueInRightPane(true)
      setShowConfigInRightPane(false) // Hide config when showing queue
      setRightPaneComponent(null) // Clear any existing component
    }
  }

  const handleCloseConfig = () => {
    setShowConfigInRightPane(false)
    
    // If we have clip start and end values, restore the clip player
    if (clipStart !== null && clipEnd !== null && clipTranscript) {
      const fallbackUrl = `/clip_player/${encodeURIComponent(clipTranscript)}?start_time=${clipStart}&end_time=${clipEnd}&display_text=false`
      const clipPlayerComponent = (
        <div className="w-full flex-1 overflow-y-auto scrollbar-hide">
          <ClipPlayer
            key={`${clipTranscript}-${clipStart}-${clipEnd}`}
            filename={clipTranscript}
            start_time_formatted={secondsToTimestamp(clipStart)}
            end_time_formatted={secondsToTimestamp(clipEnd)}
            font_size=""
            text=""
            display_text={false}
            onBack={() => {
              if (isSmallScreen) {
                // On mobile, just hide the clip player but keep clip times
                setMobileClipPlayerComponent(null)
              } else {
                // On desktop, clear everything as before
                setRightPaneComponent(null)
                handleClearClip()
              }
            }}
          />
        </div>
      )
      handleSetRightPaneComponent(clipPlayerComponent, fallbackUrl)
    }
  }

  const handleCloseQueue = () => {
    setShowQueueInRightPane(false)
    
    // If we have clip start and end values, restore the clip player
    if (clipStart !== null && clipEnd !== null && clipTranscript) {
      const fallbackUrl = `/clip_player/${encodeURIComponent(clipTranscript)}?start_time=${clipStart}&end_time=${clipEnd}&display_text=false`
      const clipPlayerComponent = (
        <div className="w-full flex-1 overflow-y-auto scrollbar-hide">
          <ClipPlayer
            key={`${clipTranscript}-${clipStart}-${clipEnd}`}
            filename={clipTranscript}
            start_time_formatted={secondsToTimestamp(clipStart)}
            end_time_formatted={secondsToTimestamp(clipEnd)}
            font_size=""
            text=""
            display_text={false}
            onBack={() => {
              if (isSmallScreen) {
                // On mobile, just hide the clip player but keep clip times
                setMobileClipPlayerComponent(null)
              } else {
                // On desktop, clear everything as before
                setRightPaneComponent(null)
                handleClearClip()
              }
            }}
          />
        </div>
      )
      handleSetRightPaneComponent(clipPlayerComponent, fallbackUrl)
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
    // Clear panes to show placeholder
    setRightPaneComponent(null)
    setMobileClipPlayerComponent(null)
    setMobileConfigComponent(null)
    setMobileQueueComponent(null)
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
      const fallbackUrl = `/clip_player/${encodeURIComponent(clipTranscript)}?start_time=${clipStart}&end_time=${clipEnd}&display_text=false`
      const clipPlayerComponent = (
        <div className="w-full flex-1 overflow-y-auto scrollbar-hide">
          <ClipPlayer
            key={`${clipTranscript}-${clipStart}-${clipEnd}`}
            filename={clipTranscript}
            start_time_formatted={secondsToTimestamp(clipStart)}
            end_time_formatted={secondsToTimestamp(clipEnd)}
            font_size=""
            text=""
            display_text={false}
            onBack={() => {
              if (isSmallScreen) {
                // On mobile, just hide the clip player but keep clip times
                setMobileClipPlayerComponent(null)
              } else {
                // On desktop, clear everything as before
                setRightPaneComponent(null)
                handleClearClip()
              }
            }}
          />
        </div>
      )

      handleSetRightPaneComponent(clipPlayerComponent, fallbackUrl)
    } else if (clipStart !== null || clipEnd !== null) {
      // If we have partial clip data, clear the right pane to show placeholder
      if (isSmallScreen) {
        setMobileClipPlayerComponent(null)
      } else {
        setRightPaneComponent(null)
      }
    }
  }, [clipStart, clipEnd, clipTranscript, handleSetRightPaneComponent, isSmallScreen])

  // Make the function available globally for testing
  useEffect(() => {
    if (typeof window !== 'undefined') {
      (window as any).handleSetRightPaneComponent = handleSetRightPaneComponent
    }
  }, [handleSetRightPaneComponent])

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
        expandedFiles={expandedFiles}
        isSearching={isSearching}
        queue={queue}
        currentProcessingFile={currentProcessingFile}
        isAtTop={isAtTop}
        onSearch={handleSearch}
        onClearSearch={handleClearSearch}
        onScrollToTop={handleScrollToTop}
        onCollapseExpanded={handleCollapseExpanded}
        onCollapseAll={handleCollapseAll}
        onConfigClick={handleConfigClick}
        onQueueClick={handleQueueClick}
      />

      {/* Main content with top padding to account for fixed header */}
      <div className={`flex h-screen`}>
        {/* Conditional rendering for mobile */}
        {isSmallScreen && (mobileClipPlayerComponent || mobileConfigComponent || mobileQueueComponent) ? (
          // Show active mobile component
          <div className="w-full">
            {mobileClipPlayerComponent || mobileConfigComponent || mobileQueueComponent}
          </div>
        ) : (
          // Show transcript list (default view)
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


            leftPaneWidth={leftPaneWidth}
            setLeftPaneWidth={setLeftPaneWidth}
            isLeftPaneWidthMeasured={isLeftPaneWidthMeasured}
            setIsLeftPaneWidthMeasured={setIsLeftPaneWidthMeasured}
            clipStart={clipStart}
            clipEnd={clipEnd}
            clipTranscript={clipTranscript}
            fileRowRefs={fileRowRefs}
            transcriptRowRefs={transcriptRowRefs}
            mobileTranscriptRowRefs={mobileTranscriptRowRefs}
            leftPaneRef={leftPaneRef}
            onSetRightPaneUrl={handleSetRightPaneComponent}
            onSetClipStart={handleSetClipStart}
            onSetClipEnd={handleSetClipEnd}
            onClearClip={handleClearClip}
            onClipBlock={handleClipBlock}
          />
        )}
        
        {/* Right Pane - Always visible on desktop */}
        {!isSmallScreen && (
          <div className="w-1/2 border-l border-border flex flex-col scrollbar-hide">
            {showConfigInRightPane ? (
              <ConfigPage onClose={handleCloseConfig} />
            ) : showQueueInRightPane ? (
              <QueuePage onClose={handleCloseQueue} />
            ) : rightPaneComponent ? (
              rightPaneComponent
            ) : (
              <RightPanePlaceholder />
            )}
          </div>
        )}
      </div>
    </>
  )
}