import FileCard from './FileCard'
import TranscriptView from './TranscriptView'

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

interface MobileTranscriptListProps {
  sortedFiles: FileRow[]
  activeSearchTerm: string
  searchResults: string[]
  showAllFiles: boolean
  transcriptData: Record<string, TranscriptData>
  expandedFiles: Set<string>
  searchLineNumbers: Record<string, number[]>

  onExpandFile: (filename: string) => void
  onRegenerate: (filename: string, source: string | undefined, e: React.MouseEvent) => void
  onReplace: (filename: string, e: React.MouseEvent) => void
  onRename: (filename: string, e: React.MouseEvent) => void
  onFetchTranscript: (filename: string) => void
  onSetRightPaneUrl: (component: React.ReactNode | null, fallbackUrl?: string) => void
  formatDate: (dateString: string) => string
  getSourceChipColor: (source: string | undefined) => string
  clipStart: number | null
  clipEnd: number | null
  clipTranscript: string | null
  onSetClipStart: (time: number, transcript: string) => void
  onSetClipEnd: (time: number, transcript: string) => void
  onClearClip: () => void
  onClipBlock: (startTime: number, endTime: number, text: string, transcript: string) => void
  expandContext: (filename: string, direction: "up" | "down", line: number) => void
  mobileTranscriptRowRefs: React.MutableRefObject<Record<string, HTMLDivElement | null>>
}

export default function MobileTranscriptList({
  sortedFiles,
  activeSearchTerm,
  searchResults,
  showAllFiles,
  transcriptData,
  expandedFiles,
  searchLineNumbers,

  onExpandFile,
  onRegenerate,
  onReplace,
  onRename,
  onFetchTranscript,
  onSetRightPaneUrl,
  formatDate,
  getSourceChipColor,
  expandContext,
  clipStart,
  clipEnd,
  clipTranscript,
  onSetClipStart,
  onSetClipEnd,
  onClearClip,
  onClipBlock,
  mobileTranscriptRowRefs
}: MobileTranscriptListProps) {
  return (
    <div className="divide-y divide-border">
      {sortedFiles.map((file) => {
        // When searching, only show files with search results regardless of showAllFiles toggle
        // When not searching, show all files if showAllFiles is enabled, otherwise show none
        const shouldShowFile = activeSearchTerm 
          ? searchResults.includes(file.full_path)
          : showAllFiles
        if (!shouldShowFile) {
          return null;
        }
        
        const transcriptInfo = transcriptData[file.full_path] || { text: '', loading: false, error: null }
        const isExpanded = expandedFiles.has(file.full_path)
        
        return (
          <div key={file.base_name} className="w-full" >
            <div id="ref">
            <FileCard
              file={file}
              onExpand={() => onExpandFile(file.full_path)}
              isExpanded={isExpanded}
              isRegenerating={!file.transcript}
              isReplacing={!file.transcript}
              isProcessing={false}

              onRegenerate={(e) => onRegenerate(file.full_path, file.source, e)}
              onReplace={(e) => onReplace(file.full_path, e)}
              onRename={(e) => onRename(file.full_path, e)}
              onRegenerateMeta={(e) => onRegenerate(file.full_path, file.source, e)}
              formatDate={formatDate}
              getSourceChipColor={getSourceChipColor}
              isSmallScreen={true}
              mobileTranscriptRowRefs={mobileTranscriptRowRefs}
            />
            </div>
            {isExpanded && (
              <TranscriptView
                visible={true}
                name={file.base_name}
                fullPath={file.full_path}
                className="w-full"
                searchTerm={activeSearchTerm}
                text={transcriptInfo.text}
                loading={transcriptInfo.loading}
                error={transcriptInfo.error}
                visibleLines={searchLineNumbers[file.full_path] || []}
                expandContext={expandContext}
                onEditSuccess={() => { onFetchTranscript(file.full_path) }}
                isSmallScreen={true}
                onSetRightPaneUrl={onSetRightPaneUrl}
                clipStart={clipStart}
                clipEnd={clipEnd}
                clipTranscript={clipTranscript}
                onSetClipStart={(time) => onSetClipStart(time, file.full_path)}
                onSetClipEnd={(time) => onSetClipEnd(time, file.full_path)}
                onClearClip={onClearClip}

                onClipBlock={onClipBlock}
              />
            )}
          </div>
        )
      })}
    </div>
  )
}
