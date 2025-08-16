import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuCheckboxItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from "./ui/dropdown-menu"

interface SearchPopupProps {
  searchTerm: string
  setSearchTerm: (term: string) => void
  activeSearchTerm: string
  setActiveSearchTerm: (term: string) => void
  setSearchLineNumbers: (lineNumbers: Record<string, number[]>) => void
  setExpandedFiles: (files: Set<string>) => void
  isSearching: boolean
  isOpen: boolean
  onClose: () => void
  onSearch: () => void
  onClearSearch: () => void
  selectedWatchDirs: string[]
  setSelectedWatchDirs: (dirs: string[]) => void
  availableWatchDirs: string[]
  selectedSources: string[]
  setSelectedSources: (sources: string[]) => void
  availableSources: string[]
}

export default function SearchPopup({
  searchTerm,
  setSearchTerm,
  activeSearchTerm,
  setActiveSearchTerm,
  setSearchLineNumbers,
  setExpandedFiles,
  isSearching,
  isOpen,
  onClose,
  onSearch,
  onClearSearch,
  selectedWatchDirs,
  setSelectedWatchDirs,
  availableWatchDirs,
  selectedSources,
  setSelectedSources,
  availableSources
}: SearchPopupProps) {
  const handleClearSearch = () => {
    setSearchTerm('')
    setActiveSearchTerm('')
    setSearchLineNumbers({})
    setExpandedFiles(new Set())
    onClearSearch()
  }

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-start justify-center pt-20">
      <div className="bg-background border border-border rounded-lg shadow-lg w-full max-w-md mx-4">
        <div className="p-4">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold">Search</h3>
            <button
              onClick={onClose}
              className="p-1 text-muted-foreground hover:text-foreground transition-colors"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
          <div className="relative mb-4">
            <input
              type="text"
              placeholder="Search"
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
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  onSearch()
                  onClose()
                }
              }}
              className="w-full px-3 py-2 pr-10 text-sm border border-input bg-background text-foreground rounded focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent"
              autoFocus
            />
            {(searchTerm || activeSearchTerm) && (
              <button
                onClick={handleClearSearch}
                className="absolute right-3 top-1/2 transform -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
                title="Clear search"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            )}
          </div>

          {/* Filters */}
          {(availableWatchDirs.length > 1 || availableSources.length > 1) && (
            <div className="mb-4 space-y-3">
              <h4 className="text-sm font-medium text-foreground">Search Filters</h4>
              
              {/* Watch Directory Filter */}
              {availableWatchDirs.length > 1 && (
                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">Watch Directories</label>
                  <DropdownMenu modal={false}>
                    <DropdownMenuTrigger asChild>
                      <button className="w-full inline-flex items-center justify-between whitespace-nowrap rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50">
                        {selectedWatchDirs.length === availableWatchDirs.length 
                          ? "All Directories" 
                          : selectedWatchDirs.length === 0 
                          ? "No Directories" 
                          : `${selectedWatchDirs.length} Director${selectedWatchDirs.length === 1 ? 'y' : 'ies'}`}
                        <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                        </svg>
                      </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start" className="w-80">
                      <DropdownMenuLabel>Watch Directories</DropdownMenuLabel>
                      <DropdownMenuSeparator />
                      <div className="grid grid-cols-1 gap-1 p-2">
                        <DropdownMenuCheckboxItem
                          checked={selectedWatchDirs.length === availableWatchDirs.length}
                          onCheckedChange={(checked) => {
                            if (checked) {
                              setSelectedWatchDirs([...availableWatchDirs])
                            } else {
                              setSelectedWatchDirs([])
                            }
                          }}
                        >
                          Select All
                        </DropdownMenuCheckboxItem>
                        <DropdownMenuSeparator />
                        {availableWatchDirs.map((dir) => (
                          <DropdownMenuCheckboxItem
                            key={dir}
                            checked={selectedWatchDirs.includes(dir)}
                            onCheckedChange={(checked) => {
                              if (checked) {
                                setSelectedWatchDirs([...selectedWatchDirs, dir])
                              } else {
                                setSelectedWatchDirs(selectedWatchDirs.filter(d => d !== dir))
                              }
                            }}
                          >
                            <span className="truncate" title={dir}>
                              {dir.split('/').pop() || dir}
                            </span>
                          </DropdownMenuCheckboxItem>
                        ))}
                      </div>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              )}

              {/* Source Filter */}
              {availableSources.length > 1 && (
                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">Transcript Sources</label>
                  <DropdownMenu modal={false}>
                    <DropdownMenuTrigger asChild>
                      <button className="w-full inline-flex items-center justify-between whitespace-nowrap rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50">
                        {selectedSources.length === availableSources.length 
                          ? "All Sources" 
                          : selectedSources.length === 0 
                          ? "No Sources" 
                          : `${selectedSources.length} Source${selectedSources.length === 1 ? '' : 's'}`}
                        <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                        </svg>
                      </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start" className="w-80">
                      <DropdownMenuLabel>Sources</DropdownMenuLabel>
                      <DropdownMenuSeparator />
                      <div className="grid grid-cols-1 gap-1 p-2">
                        <DropdownMenuCheckboxItem
                          checked={selectedSources.length === availableSources.length}
                          onCheckedChange={(checked) => {
                            if (checked) {
                              setSelectedSources([...availableSources])
                            } else {
                              setSelectedSources([])
                            }
                          }}
                        >
                          Select All
                        </DropdownMenuCheckboxItem>
                        <DropdownMenuSeparator />
                        {availableSources.map((source) => (
                          <DropdownMenuCheckboxItem
                            key={source}
                            checked={selectedSources.includes(source)}
                            onCheckedChange={(checked) => {
                              if (checked) {
                                setSelectedSources([...selectedSources, source])
                              } else {
                                setSelectedSources(selectedSources.filter(s => s !== source))
                              }
                            }}
                          >
                            {source}
                          </DropdownMenuCheckboxItem>
                        ))}
                      </div>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              )}
            </div>
          )}
          <div className="flex gap-2">
            <button
              onClick={() => {
                onSearch()
                onClose()
              }}
              disabled={isSearching}
              className="flex-1 px-4 py-2 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              <svg
                className={`w-4 h-4 ${isSearching ? "animate-spin" : ""}`}
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <circle cx="11" cy="11" r="7" strokeWidth={2} />
                <line x1="16.5" y1="16.5" x2="21" y2="21" strokeWidth={2} strokeLinecap="round" />
              </svg>
              Search
            </button>
            <button
              onClick={handleClearSearch}
              className="px-4 py-2 text-sm border border-red-300 bg-red-50 text-red-700 rounded hover:bg-red-100 focus:outline-none focus:ring-2 focus:ring-red-500"
            >
              Clear
            </button>
            <button
              onClick={onClose}
              className="px-4 py-2 text-sm border border-input bg-background text-foreground rounded hover:bg-accent focus:outline-none focus:ring-2 focus:ring-ring"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}