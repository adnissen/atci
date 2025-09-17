
import { Button } from "./ui/button"
import { useState } from "react"
import { useIsSmallScreen } from "../hooks/useMediaQuery"
import SearchPopup from "./SearchPopup"

interface TopBarProps {
  show: boolean
  searchTerm: string
  setSearchTerm: (term: string) => void
  activeSearchTerm: string
  setActiveSearchTerm: (term: string) => void
  setSearchLineNumbers: (lineNumbers: Record<string, number[]>) => void
  setExpandedFiles: (files: Set<string>) => void
  isSearching: boolean
  queue: Array<{ video_path: string; process_type: string }>
  currentProcessingFile: { video_path: string; process_type: string } | null
  isAtTop: boolean
  clipStart?: number | null
  clipEnd?: number | null
  mobileClipPlayerComponent?: React.ReactNode | null
  selectedWatchDirs: string[]
  setSelectedWatchDirs: (dirs: string[]) => void
  availableWatchDirs: string[]
  selectedSources: string[]
  setSelectedSources: (sources: string[]) => void
  availableSources: string[]
  showAllFiles: boolean
  onToggleShowAllFiles: () => void
  onSearch: () => void
  onClearSearch: () => void
  onScrollToTop: () => void
  onConfigClick: () => void
  onQueueClick: () => void
  onPlayClip?: () => void
}

export default function TopBar({
  show,
  searchTerm,
  setSearchTerm,
  activeSearchTerm,
  setActiveSearchTerm,
  setSearchLineNumbers,
  setExpandedFiles,
  isSearching,
  queue,
  isAtTop,
  clipStart,
  clipEnd,
  mobileClipPlayerComponent,
  selectedWatchDirs,
  setSelectedWatchDirs,
  availableWatchDirs,
  selectedSources,
  setSelectedSources,
  availableSources,
  showAllFiles,
  onToggleShowAllFiles,
  onSearch,
  onClearSearch,
  onScrollToTop,
  onConfigClick,
  onQueueClick,
  onPlayClip
}: TopBarProps) {
  const [showSearchPopup, setShowSearchPopup] = useState(false)
  const isSmallScreen = useIsSmallScreen()

  const handleClearSearch = () => {
    setSearchTerm('')
    setActiveSearchTerm('')
    setSearchLineNumbers({})
    setExpandedFiles(new Set()) // Collapse all expanded files
    onClearSearch()
  }


  return (
    <>
      {/* Top Bar - Fixed to top */}
      {show && (
        <div className={`fixed top-0 left-0 right-0 bg-background/95 border-b border-border px-2 sm:px-4 z-10 backdrop-blur-sm ${isSmallScreen ? 'py-1' : 'py-2'}`}>
          <div className="w-full">
            {/* Mobile: Two-row layout */}
            {isSmallScreen ? (
              <div className="flex flex-col gap-2">
                {/* Top row - App title and Action buttons */}
                <div className="flex justify-between items-center gap-1">
                  {/* App title */}
                  <div className="flex items-center flex-shrink-0">
                    <span className="text-primary font-bold text-lg">atci</span>
                  </div>

                  <div className="flex items-center gap-1">
                    {/* Config button */}
                    <Button
                      onClick={onConfigClick}
                      variant="ghost"
                      size="sm"
                      className="px-2 py-1 rounded hover:bg-accent group text-xs gap-1"
                      title="Edit configuration"
                    >
                      <svg className="w-4 h-4 group-hover:animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                      </svg>
                      <span>Config</span>
                    </Button>

                    {/* Queue button */}
                    <div className="relative">
                      <Button
                        onClick={onQueueClick}
                        variant="ghost"
                        size="sm"
                        className="px-2 py-1 rounded hover:bg-accent text-xs gap-1 group"
                        title="View processing queue"
                      >
                        <div className="relative w-4 h-4 overflow-hidden">
                          <div className="group-hover:animate-[moveUp_1s_linear_infinite] transition-transform">
                            {/* Create individual animated lines */}
                            <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '2px'}}></div>
                            <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '6px'}}></div>
                            <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '10px'}}></div>
                            <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '14px'}}></div>
                            {/* Additional lines for continuous effect */}
                            <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '18px'}}></div>
                            <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '22px'}}></div>
                          </div>
                        </div>
                        <span>Queue</span>
                      </Button>
                      {queue.length > 0 && (
                        <div className="absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full min-w-[18px] h-[18px] flex items-center justify-center px-1">
                          {queue.length}
                        </div>
                      )}
                    </div>



                    {/* Clip editor button */}
                    <Button
                      onClick={onPlayClip}
                      disabled={clipStart === null || clipStart === undefined || clipEnd === null || clipEnd === undefined || !onPlayClip || mobileClipPlayerComponent !== null}
                      variant="ghost"
                      size="sm"
                      className="px-2 py-1 rounded hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed text-xs gap-1"
                      title={
                        mobileClipPlayerComponent !== null 
                          ? "Clip player is already open" 
                          : (clipStart !== null && clipStart !== undefined && clipEnd !== null && clipEnd !== undefined ? "Play Clip" : "No clip selected")
                      }
                    >
                      <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M8 5v14l11-7z"/>
                      </svg>
                      <span>Clip</span>
                    </Button>
                  </div>
                </div>

                {/* Bottom row - Search input, Search & All buttons */}
                <div className="flex gap-2 items-center">
                  {/* Search input */}
                  <div className="flex-1">
                    <div className="relative">
                      <input
                        type="text"
                        placeholder="looking for something?"
                        value={searchTerm}
                        onChange={(e) => {
                          const newValue = e.target.value
                          setSearchTerm(newValue)
                          if (newValue.trim() === '') {
                            setActiveSearchTerm('')
                            setSearchLineNumbers({})
                            setExpandedFiles(new Set())
                          }
                        }}
                        onKeyDown={(e) => e.key === 'Enter' && onSearch()}
                        className="w-full px-3 py-1.5 bg-muted text-foreground text-sm rounded-full border border-border focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent"
                      />
                      {(searchTerm || activeSearchTerm) && (
                        <button
                          onClick={handleClearSearch}
                          className="absolute right-2 top-1/2 transform -translate-y-1/2 p-0.5 text-muted-foreground hover:text-foreground transition-colors rounded-full hover:bg-accent"
                          title="Clear search"
                        >
                          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                          </svg>
                        </button>
                      )}
                    </div>
                  </div>

                  {/* Search button */}
                  <Button
                    onClick={() => {
                      if (showAllFiles) {
                        onToggleShowAllFiles()
                      }
                      onSearch()
                    }}
                    disabled={isSearching}
                    variant={(showAllFiles && !activeSearchTerm) ? "secondary" : "default"}
                    className={`px-3 py-1.5 rounded-full text-sm font-medium disabled:opacity-50 ${
                      (showAllFiles && !activeSearchTerm)
                        ? "" 
                        : "bg-primary hover:bg-primary/90 text-primary-foreground"
                    }`}
                  >
                    {isSearching ? (
                      <svg className="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <circle cx="11" cy="11" r="7" strokeWidth={2} />
                        <line x1="16.5" y1="16.5" x2="21" y2="21" strokeWidth={2} strokeLinecap="round" />
                      </svg>
                    ) : (
                      "Search"
                    )}
                  </Button>

                  {/* All button */}
                  <Button
                    onClick={() => {
                      handleClearSearch()
                      if (!showAllFiles) {
                        onToggleShowAllFiles()
                      }
                      onScrollToTop()
                    }}
                    variant={(showAllFiles && !activeSearchTerm) ? "default" : "secondary"}
                    className="px-3 py-1.5 rounded-full text-sm font-medium"
                  >
                    All
                  </Button>
                </div>
              </div>
            ) : (
              /* Desktop: Single row layout */
              <div className="flex gap-2 items-center">
                {/* App title */}
                <div className="flex items-center flex-shrink-0">
                  <span className="text-primary font-bold text-lg">atci</span>
                </div>
                
                {/* Search input (desktop) */}
                <div className="flex-1 max-w-md mx-4">
                  <div className="relative">
                    <input
                      type="text"
                      placeholder="looking for something?"
                      value={searchTerm}
                      onChange={(e) => {
                        const newValue = e.target.value
                        setSearchTerm(newValue)
                        if (newValue.trim() === '') {
                          setActiveSearchTerm('')
                          setSearchLineNumbers({})
                          setExpandedFiles(new Set())
                        }
                      }}
                      onKeyDown={(e) => e.key === 'Enter' && onSearch()}
                      className="w-full px-4 py-2 bg-muted text-foreground rounded-full border border-border focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent"
                    />
                    {(searchTerm || activeSearchTerm) && (
                      <button
                        onClick={handleClearSearch}
                        className="absolute right-3 top-1/2 transform -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors rounded-full hover:bg-accent"
                        title="Clear search"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                        </svg>
                      </button>
                    )}
                  </div>
                </div>

                {/* Action buttons */}
                <div className="flex gap-1 items-center flex-shrink-0">
                  {/* Search button */}
                  <Button
                    onClick={() => {
                      if (showAllFiles) {
                        onToggleShowAllFiles()
                      }
                      onSearch()
                    }}
                    disabled={isSearching}
                    variant={(showAllFiles && !activeSearchTerm) ? "secondary" : "default"}
                    className={`px-4 py-2 rounded-full text-sm font-medium disabled:opacity-50 ${
                      (showAllFiles && !activeSearchTerm)
                        ? "" 
                        : "bg-primary hover:bg-primary/90 text-primary-foreground"
                    }`}
                  >
                    {isSearching ? (
                      <svg className="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <circle cx="11" cy="11" r="7" strokeWidth={2} />
                        <line x1="16.5" y1="16.5" x2="21" y2="21" strokeWidth={2} strokeLinecap="round" />
                      </svg>
                    ) : (
                      "Search"
                    )}
                  </Button>

                  {/* All button */}
                  <Button
                    onClick={() => {
                      handleClearSearch()
                      if (!showAllFiles) {
                        onToggleShowAllFiles()
                      }
                      onScrollToTop()
                    }}
                    variant={(showAllFiles && !activeSearchTerm) ? "default" : "secondary"}
                    className="px-4 py-2 rounded-full text-sm font-medium"
                  >
                    All
                  </Button>

                  {/* Queue button */}
                  <div className="relative">
                    <Button
                      onClick={onQueueClick}
                      variant="ghost"
                      size="sm"
                      className="p-2 rounded-full hover:bg-accent group"
                      title="View processing queue"
                    >
                      <div className="relative w-5 h-5 overflow-hidden">
                        <div className="group-hover:animate-[moveUp_1s_linear_infinite] transition-transform">
                          {/* Create individual animated lines */}
                          <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '3px'}}></div>
                          <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '8px'}}></div>
                          <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '13px'}}></div>
                          <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '18px'}}></div>
                          {/* Additional lines for continuous effect */}
                          <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '23px'}}></div>
                          <div className="absolute left-0 right-0 h-px bg-current rounded-full" style={{top: '28px'}}></div>
                        </div>
                      </div>
                    </Button>
                    {queue.length > 0 && (
                      <div className="absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full min-w-[18px] h-[18px] flex items-center justify-center px-1">
                        {queue.length}
                      </div>
                    )}
                  </div>


                  {/* Scroll to top button */}
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      if (!isAtTop) {
                        onScrollToTop()
                      }
                    }}
                    disabled={isAtTop}
                    title={!isAtTop ? "Scroll to top" : "Already at top"}
                    className="p-2 rounded-full hover:bg-accent disabled:opacity-50"
                  >
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 10l7-7m0 0l7 7m-7-7v18" />
                    </svg>
                  </Button>

                  {/* Config button */}
                  <Button
                    onClick={onConfigClick}
                    variant="ghost"
                    size="sm"
                    className="p-2 rounded-full hover:bg-accent group"
                    title="Edit configuration"
                  >
                    <svg className="w-5 h-5 group-hover:animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                    </svg>
                  </Button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      <SearchPopup
        searchTerm={searchTerm}
        setSearchTerm={setSearchTerm}
        activeSearchTerm={activeSearchTerm}
        setActiveSearchTerm={setActiveSearchTerm}
        setSearchLineNumbers={setSearchLineNumbers}
        setExpandedFiles={setExpandedFiles}
        isSearching={isSearching}
        isOpen={showSearchPopup}
        onClose={() => setShowSearchPopup(false)}
        onSearch={onSearch}
        onClearSearch={onClearSearch}
        selectedWatchDirs={selectedWatchDirs}
        setSelectedWatchDirs={setSelectedWatchDirs}
        availableWatchDirs={availableWatchDirs}
        selectedSources={selectedSources}
        setSelectedSources={setSelectedSources}
        availableSources={availableSources}
      />
    </>
  )
}
