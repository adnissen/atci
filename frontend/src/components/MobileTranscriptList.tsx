import FileCard from './FileCard'
import TranscriptView from './TranscriptView'

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

interface MobileTranscriptListProps {
  sortedFiles: FileRow[]
  activeSearchTerm: string
  searchResults: string[]
  transcriptData: Record<string, TranscriptData>
  expandedFiles: Set<string>
  regeneratingFiles: Set<string>
  replacingFiles: Set<string>
  searchLineNumbers: Record<string, number[]>

  onExpandFile: (filename: string) => void
  onRegenerate: (filename: string, e: React.MouseEvent) => void
  onReplace: (filename: string, e: React.MouseEvent) => void
  onRename: (filename: string, e: React.MouseEvent) => void
  onRegenerateMeta: (filename: string, e: React.MouseEvent) => void
  onFetchTranscript: (filename: string) => void
  onSetRightPaneUrl: (component: React.ReactNode | null, fallbackUrl?: string) => void
  isFileBeingProcessed: (filename: string) => boolean
  formatDate: (dateString: string) => string
  getModelChipColor: (model: string | undefined) => string
  clipStart: number | null
  clipEnd: number | null
  clipTranscript: string | null
  onSetClipStart: (time: number, transcript: string) => void
  onSetClipEnd: (time: number, transcript: string) => void
  onClearClip: () => void
  onClipBlock: (startTime: number, endTime: number, transcript: string) => void
  expandContext: (filename: string, direction: "up" | "down", line: number) => void
  expandAll: (filename: string) => void
  mobileTranscriptRowRefs: React.MutableRefObject<Record<string, HTMLDivElement | null>>
}

export default function MobileTranscriptList({
  sortedFiles,
  activeSearchTerm,
  searchResults,
  transcriptData,
  expandedFiles,
  regeneratingFiles,
  replacingFiles,
  searchLineNumbers,

  onExpandFile,
  onRegenerate,
  onReplace,
  onRename,
  onRegenerateMeta,
  onFetchTranscript,
  onSetRightPaneUrl,
  isFileBeingProcessed,
  formatDate,
  getModelChipColor,
  expandContext,
  expandAll,
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
        if (activeSearchTerm !== '' && !searchResults.includes(file.base_name)) {
          return null;
        }
        
        const transcriptInfo = transcriptData[file.base_name] || { text: '', loading: false, error: null }
        const isExpanded = expandedFiles.has(file.base_name)
        
        return (
          <div key={file.base_name} className="w-full" ref={(el) => { mobileTranscriptRowRefs.current[file.base_name] = el }}>
            <FileCard
              file={file}
              onExpand={() => onExpandFile(file.base_name)}
              isExpanded={isExpanded}
              isRegenerating={regeneratingFiles.has(file.base_name)}
              isReplacing={replacingFiles.has(file.base_name)}
              isProcessing={isFileBeingProcessed(file.base_name)}

              onRegenerate={(e) => onRegenerate(file.base_name, e)}
              onReplace={(e) => onReplace(file.base_name, e)}
              onRename={(e) => onRename(file.base_name, e)}
              onRegenerateMeta={(e) => onRegenerateMeta(file.base_name, e)}
              formatDate={formatDate}
              getModelChipColor={getModelChipColor}
              isSmallScreen={true}
            />
            {isExpanded && (
              <TranscriptView
                visible={true}
                name={file.base_name}
                className="w-full"
                searchTerm={activeSearchTerm}
                text={transcriptInfo.text}
                loading={transcriptInfo.loading}
                error={transcriptInfo.error}
                visibleLines={searchLineNumbers[file.base_name] || []}
                expandContext={expandContext}
                expandAll={expandAll}
                onEditSuccess={() => { onFetchTranscript(file.base_name) }}
                isSmallScreen={true}
                onSetRightPaneUrl={onSetRightPaneUrl}
                clipStart={clipStart}
                clipEnd={clipEnd}
                clipTranscript={clipTranscript}
                onSetClipStart={(time) => onSetClipStart(time, file.base_name)}
                onSetClipEnd={(time) => onSetClipEnd(time, file.base_name)}
                onClearClip={onClearClip}

                onClipBlock={(startTime, endTime) => onClipBlock(startTime, endTime, file.base_name)}
              />
            )}
          </div>
        )
      })}
    </div>
  )
}
