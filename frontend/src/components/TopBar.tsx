

interface TopBarProps {
  watchDirectory: string
  searchTerm: string
  setSearchTerm: (term: string) => void
  activeSearchTerm: string
  setActiveSearchTerm: (term: string) => void
  setSearchLineNumbers: (lineNumbers: Record<string, number[]>) => void
  setExpandedFiles: (files: Set<string>) => void
  isSearching: boolean
  queue: Array<{ video_path: string; process_type: string }>
  currentProcessingFile: { video_path: string; process_type: string } | null
  outOfViewExpandedFile: string | null
  onSearch: () => void
  onClearSearch: () => void
  onScrollToTop: () => void
  onCollapseExpanded: () => void
  onConfigClick: () => void
  onQueueClick: () => void
}

export default function TopBar({
  watchDirectory,
  searchTerm,
  setSearchTerm,
  activeSearchTerm,
  setActiveSearchTerm,
  setSearchLineNumbers,
  setExpandedFiles,
  isSearching,
  queue,
  currentProcessingFile,
  outOfViewExpandedFile,
  onSearch,
  onClearSearch,
  onScrollToTop,
  onCollapseExpanded,
  onConfigClick,
  onQueueClick
}: TopBarProps) {
  const handleClearSearch = () => {
    setSearchTerm('')
    setActiveSearchTerm('')
    setSearchLineNumbers({})
    setExpandedFiles(new Set()) // Collapse all expanded files
    onClearSearch()
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
                    onClick={onConfigClick}
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
                      onClick={onQueueClick}
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
                        onKeyDown={(e) => e.key === 'Enter' && onSearch()}
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
                      onClick={onSearch}
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
                    onClick={onScrollToTop}
                    className="text-xs sm:text-sm text-primary hover:text-primary/80 underline transition-colors"
                    title={`Scroll to ${outOfViewExpandedFile}`}
                  >
                    scroll to row
                  </button>
                  <button
                    onClick={onCollapseExpanded}
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
    </>
  )
}
