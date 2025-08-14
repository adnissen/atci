import React from 'react'
import { Card, CardHeader, CardContent, CardFooter } from './ui/card'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuItem,
} from './ui/dropdown-menu'

interface FileRow {
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

interface FileCardProps {
  file: FileRow
  onExpand: () => void
  isExpanded: boolean
  isRegenerating: boolean
  isReplacing: boolean
  isProcessing: boolean

  onRegenerate: (e: React.MouseEvent) => void
  onReplace: (e: React.MouseEvent) => void
  onRename: (e: React.MouseEvent) => void
  onRegenerateMeta: (e: React.MouseEvent) => void
  formatDate: (date: string) => string
  getModelChipColor: (model: string | undefined) => string
  isSmallScreen?: boolean
  mobileTranscriptRowRefs: React.MutableRefObject<Record<string, HTMLDivElement | null>>
}

export default function FileCard({
  file,
  onExpand,
  isExpanded,
  isRegenerating,
  isReplacing,
  isProcessing,

  onRegenerate,
  onReplace,
  onRename,
  onRegenerateMeta,
  formatDate,
  getModelChipColor,
  isSmallScreen = false,
  mobileTranscriptRowRefs,
}: FileCardProps) {
  const filename = file.name.split('/').pop()?.split('\\').pop() || file.name

  if (isSmallScreen) {
    return (
      <div className="w-full bg-background py-4 px-4">
        <div className="flex justify-between items-start mb-3">
          <div className="flex-1 mr-2 min-w-0 text-left" id="ref" ref={(el) => { mobileTranscriptRowRefs.current[file.base_name] = el }}>
            <h3 className="font-medium text-base break-all overflow-hidden text-left" title={file.name}>
              {filename}

            </h3>
            {file.model && (
              <div className="text-left">
                <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium mt-2 ${getModelChipColor(file.model)}`}>
                  {file.model}
                </span>
              </div>
            )}
          </div>
          <DropdownMenu modal={false}>
            <DropdownMenuTrigger asChild>
              <button
                onClick={(e) => e.stopPropagation()}
                className="p-2 text-muted-foreground hover:text-primary hover:bg-accent rounded-md transition-colors"
                title="Actions"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                </svg>
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={(e) => onRename(e)}
                disabled={isProcessing || isRegenerating || isReplacing}
              >
                <span>Rename</span>
                <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                </svg>
              </DropdownMenuItem>
              
              {file.transcript && (
                <DropdownMenuItem
                  onClick={(e) => onReplace(e)}
                  disabled={isReplacing}
                  className="text-blue-600 hover:text-blue-700"
                >
                  <span>Edit transcript</span>
                  {!isReplacing && (
                    <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                    </svg>
                  )}
                </DropdownMenuItem>
              )}
              
              {file.transcript && (
                <DropdownMenuItem
                  onClick={(e) => onRegenerate(e)}
                  disabled={isRegenerating}
                  className="text-green-600 hover:text-green-700"
                >
                  <span>Regenerate transcript</span>
                  {isRegenerating ? (
                    <svg className="w-4 h-4 ml-auto animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                  ) : (
                    <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                  )}
                </DropdownMenuItem>
              )}
              
              {isProcessing && (
                <DropdownMenuItem disabled>
                  <span>Processing transcript...</span>
                  <svg className="w-4 h-4 ml-auto animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
        
        <div className="space-y-3 mb-3">
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="min-w-0 text-left">
              <span className="text-muted-foreground text-xs">Date:</span>
              <div className="font-medium break-words text-xs">{formatDate(file.created_at)}</div>
            </div>
            <div className="min-w-0 text-left">
              <span className="text-muted-foreground text-xs">Last Generated:</span>
              <div className="font-medium break-words text-xs">{formatDate(file.last_generated || '')}</div>
            </div>
            <div className="min-w-0 text-left">
              <span className="text-muted-foreground text-xs">Lines:</span>
              <div className="font-medium text-xs">{file.line_count || 0}</div>
            </div>
            <div className="min-w-0 text-left">
              <span className="text-muted-foreground text-xs">Length:</span>
              <div className="font-medium flex items-center gap-1 text-xs">
                {file.length ? (
                  file.length
                ) : (
                  <>
                    <span>-</span>
                    <button
                      onClick={(e) => onRegenerateMeta(e)}
                      disabled={isRegenerating}
                      className="p-1 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                      title="Generate video length"
                    >
                      {isRegenerating ? (
                        <svg className="w-3 h-3 animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                      ) : (
                        <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                      )}
                    </button>
                  </>
                )}
              </div>
            </div>
          </div>
        </div>
        
        {file.transcript && (
          <button
            onClick={onExpand}
            className="w-full px-4 py-2 text-sm font-medium text-primary-foreground bg-primary hover:bg-primary/90 rounded-md transition-colors flex items-center justify-center gap-2"
          >
            {isExpanded ? (
              <>
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                </svg>
                Hide Transcript
              </>
            ) : (
              <>
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
                Show Transcript
              </>
            )}
          </button>
        )}
      </div>
    )
  }

  return (
    <Card className="w-full">
      <CardHeader className="pb-3">
        <div className="flex justify-between items-start">
          <div className="flex-1 mr-2 min-w-0">
            <h3 className="font-medium text-base break-all overflow-hidden" title={file.name}>
              {filename}

            </h3>
            {file.model && (
              <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium mt-2 ${getModelChipColor(file.model)}`}>
                {file.model}
              </span>
            )}
          </div>
          <DropdownMenu modal={false}>
            <DropdownMenuTrigger asChild>
              <button
                onClick={(e) => e.stopPropagation()}
                className="p-2 text-muted-foreground hover:text-primary hover:bg-accent rounded-md transition-colors"
                title="Actions"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                </svg>
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={(e) => onRename(e)}
                disabled={isProcessing || isRegenerating || isReplacing}
              >
                <span>Rename</span>
                <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                </svg>
              </DropdownMenuItem>
              
              {file.transcript && (
                <DropdownMenuItem
                  onClick={(e) => onReplace(e)}
                  disabled={isReplacing}
                  className="text-blue-600 hover:text-blue-700"
                >
                  <span>Edit transcript</span>
                  {!isReplacing && (
                    <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                    </svg>
                  )}
                </DropdownMenuItem>
              )}
              
              {file.transcript && (
                <DropdownMenuItem
                  onClick={(e) => onRegenerate(e)}
                  disabled={isRegenerating}
                  className="text-green-600 hover:text-green-700"
                >
                  <span>Regenerate transcript</span>
                  {isRegenerating ? (
                    <svg className="w-4 h-4 ml-auto animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                  ) : (
                    <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                  )}
                </DropdownMenuItem>
              )}
              
              {isProcessing && (
                <DropdownMenuItem disabled>
                  <span>Processing transcript...</span>
                  <svg className="w-4 h-4 ml-auto animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </CardHeader>
      
      <CardContent className="space-y-3">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
          <div className="min-w-0">
            <span className="text-muted-foreground">Date:</span>
            <div className="font-medium break-words">{formatDate(file.created_at)}</div>
          </div>
          <div className="min-w-0">
            <span className="text-muted-foreground">Last Generated:</span>
            <div className="font-medium break-words">{formatDate(file.last_generated || '')}</div>
          </div>
          <div className="min-w-0">
            <span className="text-muted-foreground">Lines:</span>
            <div className="font-medium">{file.line_count || 0}</div>
          </div>
          <div className="min-w-0">
            <span className="text-muted-foreground">Length:</span>
            <div className="font-medium flex items-center justify-center gap-1">
              {file.length ? (
                file.length
              ) : (
                <>
                  <span>-</span>
                  <button
                    onClick={(e) => onRegenerateMeta(e)}
                    disabled={isRegenerating}
                    className="p-1 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    title="Generate video length"
                  >
                    {isRegenerating ? (
                      <svg className="w-3 h-3 animate-reverse-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                      </svg>
                    ) : (
                      <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                      </svg>
                    )}
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      </CardContent>
      
      {file.transcript && (
        <CardFooter className="pt-3">
          <button
            onClick={onExpand}
            className="w-full px-4 py-2 text-sm font-medium text-primary-foreground bg-primary hover:bg-primary/90 rounded-md transition-colors flex items-center justify-center gap-2"
          >
            {isExpanded ? (
              <>
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                </svg>
                Hide Transcript
              </>
            ) : (
              <>
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
                Show Transcript
              </>
            )}
          </button>
        </CardFooter>
      )}
    </Card>
  )
}