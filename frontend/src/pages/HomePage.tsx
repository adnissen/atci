import TopBar from '../components/TopBar'
import TranscriptList from '../components/TranscriptList'
import RightPanePlaceholder from '../components/RightPanePlaceholder'
import ClipPlayer from '../components/ClipPlayer'
import ConfigPage from './ConfigPage'
import QueuePage from './QueuePage'
import { useEffect, useState, useRef, useCallback } from 'react'
import { useIsSmallScreen } from '../hooks/useMediaQuery'
import { addTimestamp } from '../lib/utils'
import { useFileContext } from '../contexts/FileContext'
import {
  Drawer,
  DrawerContent,
  DrawerHeader,
  DrawerTitle,
} from '../components/ui/drawer'

// Type definitions
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

type TranscriptData = {
  text: string
  loading: boolean
  error: string | null
}

type QueueItem = {
  video_path: string
  process_type: string
}

function HomePageContent() {
  const isSmallScreen = useIsSmallScreen()
  const { 
    selectedWatchDirs, 
    selectedSources, 
    setFiles, 
    refreshFiles, 
    showAllFiles, 
    setShowAllFiles,
    page,
    setPage,
    pageSize,
    setPageSize,
    sortColumn,
    setSortColumn,
    sortDirection,
    setSortDirection,
    totalPages,
    totalRecords
  } = useFileContext()
  
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set())
  const [searchTerm, setSearchTerm] = useState('')
  const [activeSearchTerm, setActiveSearchTerm] = useState('')
  const [searchLineNumbers, setSearchLineNumbers] = useState<Record<string, number[]>>({})
  const [isSearching, setIsSearching] = useState(false)
  const [queue] = useState<QueueItem[]>([])
  const [currentProcessingFile] = useState<QueueItem | null>(null)
  const [watchDirectory] = useState<string>('TODO delete')
  const [transcriptData, setTranscriptData] = useState<Record<string, TranscriptData>>({})

  const [isAtTop, setIsAtTop] = useState<boolean>(true)
  const [leftPaneScrollOffset, setLeftPaneScrollOffset] = useState<number>(0)


  // Right pane component state
  const [rightPaneComponent, setRightPaneComponent] = useState<React.ReactNode | null>(null)
  const [leftPaneWidth, setLeftPaneWidth] = useState<number>(0)
  const [isLeftPaneWidthMeasured, setIsLeftPaneWidthMeasured] = useState<boolean>(false)
  const [showConfigInRightPane, setShowConfigInRightPane] = useState<boolean>(false)
  const [showQueueInRightPane, setShowQueueInRightPane] = useState<boolean>(true)

  // Mobile drawer state
  const [isClipDrawerOpen, setIsClipDrawerOpen] = useState(false)
  const [isConfigDrawerOpen, setIsConfigDrawerOpen] = useState(false)
  const [isQueueDrawerOpen, setIsQueueDrawerOpen] = useState(false)

  // Clip state variables
  const [clipStart, setClipStart] = useState<number | null>(null)
  const [clipEnd, setClipEnd] = useState<number | null>(null)
  const [clipTranscript, setClipTranscript] = useState<string | null>(null)
  const [clipText, setClipText] = useState<string>('')
  const fileRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const transcriptRowRefs = useRef<Record<string, HTMLTableRowElement | null>>({})
  const mobileTranscriptRowRefs = useRef<Record<string, HTMLDivElement | null>>({})
  const leftPaneRef = useRef<HTMLDivElement | null>(null)

  // Always show transcript list - drawers overlay on top
  const showingTranscriptList = true

  // Track scroll position of left pane to show/hide scroll to top button
  useEffect(() => {
    const leftPaneContainer = leftPaneRef.current
    if (!leftPaneContainer || !showingTranscriptList) return

    const handleScroll = () => {
      const scrollTop = leftPaneContainer.scrollTop
      setIsAtTop(scrollTop <= 10) // Allow small tolerance for "at top"
      setLeftPaneScrollOffset(scrollTop)
    }

    leftPaneContainer.addEventListener('scroll', handleScroll)
    
    // Initial check when transcript list is shown
    handleScroll()

    return () => {
      leftPaneContainer.removeEventListener('scroll', handleScroll)
    }
  }, [showingTranscriptList])  // Re-run when showing transcript list changes





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


  const handleSearch = async () => {
    if (!searchTerm.trim()) {
      setSearchLineNumbers({})
      setActiveSearchTerm('')
      return
    }

    setIsSearching(true)

    try {
      // Build query parameters for filtering
      const params = new URLSearchParams()      
      params.append('query', searchTerm)

      // Add watch directory filters if any are selected
      if (selectedWatchDirs.length > 0) {
        params.append('filter', selectedWatchDirs.join(','))
      }
      
      // Add source filters if any are selected
      if (selectedSources.length > 0) {
        params.append('sources', selectedSources.join(','))
      }

      const response = await fetch(addTimestamp(`/api/search?${params.toString()}`))
      if (response.ok) {
        const data = await response.json()
        if (data.success) {
          setSearchLineNumbers(data.data.map((result: any) => {
            return {
              [result.file_path]: result.matches.map((match: any) => match.line_number)
            }
          }).reduce((acc: any, curr: any) => {
            return { ...acc, ...curr }
          }, {}))
          
          // Collect video info from search results
          const videoInfos: FileRow[] = data.data.map((result: any) => {
            // Get video info from the first match (all matches have the same video info)
            const firstMatch = result.matches[0]
            if (firstMatch && firstMatch.video_info) {
              return firstMatch.video_info as FileRow
            }
            return null
          }).filter((info: FileRow | null) => info !== null)
          
          // Remove duplicates based on full_path
          const uniqueVideoInfos = videoInfos.filter((info, index, self) => 
            index === self.findIndex(v => v.full_path === info.full_path)
          )
          
          setFiles(uniqueVideoInfos)
        }
        setActiveSearchTerm(searchTerm)
        
        // Don't automatically expand files with search results
      }
    } catch (error) {
      console.error('Error searching:', error)
    } finally {
      setIsSearching(false)
    }
  }

  const handleToggleShowAllFiles = () => {
    setShowAllFiles(prev => !prev)
  }

  const handleClearSearch = () => {
    setSearchTerm('')
    setActiveSearchTerm('')
    setSearchLineNumbers({})
    setExpandedFiles(new Set()) // Collapse all expanded files
    if (showAllFiles) {
      refreshFiles()
    }
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

  // Callback functions for ClipPlayer time changes
  const handleStartTimeChange = (timeInSeconds: number) => {
    setClipStart(timeInSeconds)
  }

  const handleEndTimeChange = (timeInSeconds: number) => {
    setClipEnd(timeInSeconds)
  }



  const handleSetRightPaneComponent = useCallback((component: React.ReactNode | null, _fallbackUrl?: string) => {
    if (isSmallScreen) {
      if (component) {
        // On mobile, open clip drawer
        setIsClipDrawerOpen(true)
        // Close other drawers
        setIsConfigDrawerOpen(false)
        setIsQueueDrawerOpen(false)
      } else {
        // Close clip drawer
        setIsClipDrawerOpen(false)
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
      // On mobile, open config drawer
      setIsConfigDrawerOpen(true)
      // Close other drawers
      setIsClipDrawerOpen(false)
      setIsQueueDrawerOpen(false)
    } else {
      // On desktop, show config in right pane
      setShowConfigInRightPane(true)
      setShowQueueInRightPane(false) // Hide queue when showing config
      setRightPaneComponent(null) // Clear any existing component
    }
  }

  const handleQueueClick = () => {
    if (isSmallScreen) {
      // On mobile, open queue drawer
      setIsQueueDrawerOpen(true)
      // Close other drawers
      setIsClipDrawerOpen(false)
      setIsConfigDrawerOpen(false)
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
            key={clipTranscript}
            filename={clipTranscript}
            start_time_formatted={secondsToTimestamp(clipStart)}
            end_time_formatted={secondsToTimestamp(clipEnd)}
            font_size=""
            text={clipText}
            display_text={false}
            onStartTimeChange={handleStartTimeChange}
            onEndTimeChange={handleEndTimeChange}
            onBack={() => {
              if (isSmallScreen) {
                // On mobile, just close the clip drawer but keep clip times
                setIsClipDrawerOpen(false)
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
            key={clipTranscript}
            filename={clipTranscript}
            start_time_formatted={secondsToTimestamp(clipStart)}
            end_time_formatted={secondsToTimestamp(clipEnd)}
            font_size=""
            text={clipText}
            display_text={false}
            onStartTimeChange={handleStartTimeChange}
            onEndTimeChange={handleEndTimeChange}
            onBack={() => {
              if (isSmallScreen) {
                // On mobile, just close the clip drawer but keep clip times
                setIsClipDrawerOpen(false)
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
    setClipText('')
    // Clear panes to show placeholder
    setRightPaneComponent(null)
    setIsClipDrawerOpen(false)
    setIsConfigDrawerOpen(false)
    setIsQueueDrawerOpen(false)
    setShowConfigInRightPane(false)
    setShowQueueInRightPane(false)
  }

  const handleClipBlock = (startTime: number, endTime: number, text: string, transcript: string) => {
    setClipStart(startTime)
    setClipEnd(endTime)
    setClipTranscript(transcript)
    setClipText(text)
  }

  // Pagination handlers
  const handlePageChange = (newPage: number) => {
    setPage(newPage)
  }

  const handlePageSizeChange = (newPageSize: number) => {
    setPageSize(newPageSize)
    setPage(0) // Reset to first page when changing page size
  }

  // Pagination component
  const PaginationControls = ({ className = "" }: { className?: string }) => {
    if (!showAllFiles || totalPages <= 1) return null

    const startRecord = page * pageSize + 1
    const endRecord = Math.min((page + 1) * pageSize, totalRecords)

    return (
      <div className={`flex items-center justify-between gap-4 p-4 border-t border-border bg-background ${className}`}>
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <span>Show</span>
          <select 
            value={pageSize} 
            onChange={(e) => handlePageSizeChange(Number(e.target.value))}
            className="border border-border rounded px-2 py-1 bg-background"
          >
            <option value={10}>10</option>
            <option value={25}>25</option>
            <option value={50}>50</option>
            <option value={100}>100</option>
          </select>
          <span>per page</span>
        </div>

        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <span>
            {startRecord}-{endRecord} of {totalRecords} files
          </span>
        </div>

        <div className="flex items-center gap-1">
          <button
            onClick={() => handlePageChange(0)}
            disabled={page === 0}
            className="px-3 py-1 text-sm border border-border rounded hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
          >
            First
          </button>
          <button
            onClick={() => handlePageChange(page - 1)}
            disabled={page === 0}
            className="px-3 py-1 text-sm border border-border rounded hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Previous
          </button>
          
          <div className="flex items-center gap-1">
            {Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
              let pageNum
              if (totalPages <= 5) {
                pageNum = i
              } else if (page < 3) {
                pageNum = i
              } else if (page >= totalPages - 3) {
                pageNum = totalPages - 5 + i
              } else {
                pageNum = page - 2 + i
              }
              
              return (
                <button
                  key={pageNum}
                  onClick={() => handlePageChange(pageNum)}
                  className={`px-3 py-1 text-sm border border-border rounded hover:bg-accent ${ 
                    pageNum === page ? 'bg-primary text-primary-foreground' : ''
                  }`}
                >
                  {pageNum + 1}
                </button>
              )
            })}
          </div>

          <button
            onClick={() => handlePageChange(page + 1)}
            disabled={page >= totalPages - 1}
            className="px-3 py-1 text-sm border border-border rounded hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Next
          </button>
          <button
            onClick={() => handlePageChange(totalPages - 1)}
            disabled={page >= totalPages - 1}
            className="px-3 py-1 text-sm border border-border rounded hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Last
          </button>
        </div>
      </div>
    )
  }

  const handlePlayClip = () => {
    if (clipStart !== null && clipEnd !== null && clipTranscript) {
      const fallbackUrl = `/clip_player/${encodeURIComponent(clipTranscript)}?start_time=${clipStart}&end_time=${clipEnd}&display_text=false`
      const clipPlayerComponent = (
        <div className="w-full flex-1 overflow-y-auto scrollbar-hide">
          <ClipPlayer
            key={clipTranscript}
            filename={clipTranscript}
            start_time_formatted={secondsToTimestamp(clipStart)}
            end_time_formatted={secondsToTimestamp(clipEnd)}
            font_size=""
            text={clipText}
            display_text={false}
            onStartTimeChange={handleStartTimeChange}
            onEndTimeChange={handleEndTimeChange}
            onBack={() => {
              if (isSmallScreen) {
                handleSetRightPaneComponent(null)
                setIsAtTop(true)
                setTimeout(() => {
                  if (leftPaneRef.current) {
                    leftPaneRef.current.scrollTop = leftPaneScrollOffset
                  }
                }, 0)
              } else {
                setRightPaneComponent(null)
              }
            }}
          />
        </div>
      )
      handleSetRightPaneComponent(clipPlayerComponent, fallbackUrl)
    }
  }

  // Auto-update right pane when both clip start and end are set, or clear when incomplete
  useEffect(() => {
    if (clipStart !== null && clipEnd !== null && clipTranscript) {
      const fallbackUrl = `/clip_player/${encodeURIComponent(clipTranscript)}?start_time=${clipStart}&end_time=${clipEnd}&display_text=false`
      const clipPlayerComponent = (
        <div className="w-full flex-1 overflow-y-auto scrollbar-hide">
          <ClipPlayer
            key={clipTranscript}
            filename={clipTranscript}
            start_time_formatted={secondsToTimestamp(clipStart)}
            end_time_formatted={secondsToTimestamp(clipEnd)}
            font_size=""
            text={clipText}
            display_text={false}
            onStartTimeChange={handleStartTimeChange}
            onEndTimeChange={handleEndTimeChange}
            onBack={() => {
              if (isSmallScreen) {
                // On mobile, just close the clip drawer but keep clip times
                setIsClipDrawerOpen(false)
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
        setIsClipDrawerOpen(false)
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
        show={true}
        searchTerm={searchTerm}
        setSearchTerm={setSearchTerm}
        activeSearchTerm={activeSearchTerm}
        setActiveSearchTerm={setActiveSearchTerm}
        setSearchLineNumbers={setSearchLineNumbers}
        setExpandedFiles={setExpandedFiles}
        isSearching={isSearching}
        queue={queue}
        currentProcessingFile={currentProcessingFile}
        isAtTop={isAtTop}
        clipStart={clipStart}
        clipEnd={clipEnd}
        selectedWatchDirs={selectedWatchDirs}
        setSelectedWatchDirs={() => {}}
        availableWatchDirs={[]}
        selectedSources={selectedSources}
        setSelectedSources={() => {}}
        availableSources={[]}
        showAllFiles={showAllFiles}
        onToggleShowAllFiles={handleToggleShowAllFiles}
        onSearch={handleSearch}
        onClearSearch={handleClearSearch}
        onScrollToTop={handleScrollToTop}
        onConfigClick={handleConfigClick}
        onQueueClick={handleQueueClick}
        mobileClipPlayerComponent={null}
        onPlayClip={handlePlayClip}
      />

      {/* Main content with top padding to account for fixed header */}
      <div className={`flex h-screen`}>
        {/* Left pane with transcript list and pagination */}
        <div className="w-1/2 flex flex-col">
          {/* Top pagination - only show when viewing all files */}
          <PaginationControls className="border-b" />
          
          {/* Always show transcript list */}
          <div className="flex-1 overflow-hidden">
            <TranscriptList
              watchDirectory={watchDirectory}
              isSmallScreen={isSmallScreen}
              activeSearchTerm={activeSearchTerm}
              searchLineNumbers={searchLineNumbers}
              setSearchLineNumbers={setSearchLineNumbers}
              expandedFiles={expandedFiles}
              setExpandedFiles={setExpandedFiles}
              transcriptData={transcriptData}
              setTranscriptData={setTranscriptData}
              currentProcessingFile={currentProcessingFile}

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
              showAllFiles={showAllFiles}
            />
          </div>
          
          {/* Bottom pagination - only show when viewing all files */}
          <PaginationControls />
        </div>
        
        {/* Right Pane - Always visible on desktop */}
        {!isSmallScreen && (
          <div className={`w-1/2 border-l border-border flex flex-col scrollbar-hide pt-16`}>
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

      {/* Mobile Drawers */}
      {isSmallScreen && (
        <>
          {/* Clip Player Drawer */}
          <Drawer open={isClipDrawerOpen} onOpenChange={setIsClipDrawerOpen}>
            <DrawerContent>
              <div className="flex-1 overflow-y-auto p-4">
                {clipStart !== null && clipEnd !== null && clipTranscript && (
                  <ClipPlayer
                    key={clipTranscript}
                    filename={clipTranscript}
                    start_time_formatted={secondsToTimestamp(clipStart)}
                    end_time_formatted={secondsToTimestamp(clipEnd)}
                    font_size=""
                    text={clipText}
                    display_text={false}
                    onStartTimeChange={handleStartTimeChange}
                    onEndTimeChange={handleEndTimeChange}
                    onBack={() => setIsClipDrawerOpen(false)}
                  />
                )}
              </div>
            </DrawerContent>
          </Drawer>

          {/* Config Drawer */}
          <Drawer open={isConfigDrawerOpen} onOpenChange={setIsConfigDrawerOpen}>
            <DrawerContent>
              <DrawerHeader>
                <DrawerTitle>Configuration</DrawerTitle>
              </DrawerHeader>
              <div className="flex-1 overflow-y-auto">
                <ConfigPage onClose={() => setIsConfigDrawerOpen(false)} />
              </div>
            </DrawerContent>
          </Drawer>

          {/* Queue Drawer */}
          <Drawer open={isQueueDrawerOpen} onOpenChange={setIsQueueDrawerOpen}>
            <DrawerContent>
              <DrawerHeader>
                <DrawerTitle>Processing Queue</DrawerTitle>
              </DrawerHeader>
              <div className="flex-1 overflow-y-auto">
                <QueuePage onClose={() => setIsQueueDrawerOpen(false)} />
              </div>
            </DrawerContent>
          </Drawer>
        </>
      )}
    </>
  )
}

export default function HomePage() {
  return <HomePageContent />
}