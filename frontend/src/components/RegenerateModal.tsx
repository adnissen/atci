import { useState, useEffect } from 'react'
import { addTimestamp } from '../lib/utils'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from './ui/dropdown-menu'
import { Button } from './ui/button'

interface SubtitleStream {
  index: number
  language: string | null
}

interface ModelInfo {
  name: string
  downloaded: boolean
  path: string
  configured: boolean
}

interface RegenerateOption {
  id: string
  type: 'subtitle' | 'whisper'
  label: string
  subtitle_stream_index?: number
  model?: string
}

interface RegenerateModalProps {
  isOpen: boolean
  videoPath: string
  onClose: () => void
  onRegenerate: (model?: string, subtitleStreamIndex?: number) => Promise<void>
}

export default function RegenerateModal({
  isOpen,
  videoPath,
  onClose,
  onRegenerate
}: RegenerateModalProps) {
  const [, setSubtitleStreams] = useState<SubtitleStream[]>([])
  const [, setModels] = useState<ModelInfo[]>([])
  const [options, setOptions] = useState<RegenerateOption[]>([])
  const [selectedOption, setSelectedOption] = useState<string>('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [isRegenerating, setIsRegenerating] = useState(false)

  // Fetch subtitle streams and models when modal opens
  useEffect(() => {
    if (isOpen && videoPath) {
      fetchData()
    }
  }, [isOpen, videoPath])

  const fetchData = async () => {
    setLoading(true)
    setError(null)
    
    try {
      // Fetch subtitle streams and models in parallel
      const [subtitleResponse, modelsResponse] = await Promise.all([
        fetch(addTimestamp(`/api/video/subtitle-streams?path=${encodeURIComponent(videoPath)}`)),
        fetch(addTimestamp('/api/models/list'))
      ])

      let streams: SubtitleStream[] = []
      let modelsList: ModelInfo[] = []

      // Handle subtitle streams response
      if (subtitleResponse.ok) {
        const subtitleData = await subtitleResponse.json()
        if (subtitleData.success) {
          streams = subtitleData.data
        }
      }

      // Handle models response
      if (modelsResponse.ok) {
        const modelsData = await modelsResponse.json()
        if (modelsData.success) {
          modelsList = modelsData.data.filter((model: ModelInfo) => model.downloaded)
        }
      }

      setSubtitleStreams(streams)
      setModels(modelsList)

      // Build options list
      const newOptions: RegenerateOption[] = []

      // Add subtitle options
      streams.forEach((stream) => {
        const language = stream.language || 'Unknown'
        newOptions.push({
          id: `subtitle_${stream.index}`,
          type: 'subtitle',
          label: `Subtitles: ${language} (${stream.index})`,
          subtitle_stream_index: stream.index
        })
      })

      // Add whisper model options
      modelsList.forEach((model) => {
        const status = model.configured ? ' (currently configured)' : ''
        newOptions.push({
          id: `whisper_${model.name}`,
          type: 'whisper',
          label: `Whisper Model: ${model.name}${status}`,
          model: model.name
        })
      })

      setOptions(newOptions)

      // Clear selection to force user to choose
      setSelectedOption('')

    } catch (err) {
      console.error('Error fetching regeneration data:', err)
      setError('Failed to load regeneration options')
    } finally {
      setLoading(false)
    }
  }

  const handleRegenerate = async () => {
    if (!selectedOption) return

    const option = options.find(opt => opt.id === selectedOption)
    if (!option) return

    setIsRegenerating(true)
    
    try {
      await onRegenerate(
        option.type === 'whisper' ? option.model : undefined,
        option.type === 'subtitle' ? option.subtitle_stream_index : undefined
      )
      onClose()
    } catch (err) {
      console.error('Regeneration failed:', err)
      setError('Failed to regenerate transcript')
    } finally {
      setIsRegenerating(false)
    }
  }

  const handleClose = () => {
    if (!isRegenerating) {
      onClose()
    }
  }

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-background border border-border rounded-lg p-6 w-96 max-w-90vw max-h-80vh overflow-y-auto">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-lg font-semibold">Regenerate Transcript</h2>
          <button
            onClick={handleClose}
            disabled={isRegenerating}
            className="text-muted-foreground hover:text-foreground disabled:opacity-50"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <p className="text-sm text-muted-foreground mb-4">
          Choose a processing method for: {videoPath.split('/').pop()?.split('\\').pop() || videoPath}
        </p>

        {loading && (
          <div className="flex items-center justify-center py-8">
            <div className="flex items-center gap-2 text-muted-foreground">
              <svg className="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              Loading options...
            </div>
          </div>
        )}

        {error && (
          <div className="text-sm text-destructive mb-4 p-3 bg-destructive/10 border border-destructive/20 rounded">
            {error}
          </div>
        )}

        {!loading && !error && options.length === 0 && (
          <div className="text-sm text-muted-foreground mb-4 p-3 bg-muted border border-border rounded">
            No processing options available. Make sure you have subtitle streams or downloaded Whisper models.
          </div>
        )}

        {!loading && !error && options.length > 0 && (
          <>
            <div className="space-y-2 mb-6">
              <label className="block text-sm font-medium">Processing Method:</label>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button 
                    variant="outline" 
                    className="w-full justify-between"
                    disabled={isRegenerating}
                  >
                    {selectedOption ? 
                      options.find(opt => opt.id === selectedOption)?.label || 'Select a processing method...' 
                      : 'Select a processing method...'
                    }
                    <svg className="w-4 h-4 ml-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                    </svg>
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent className="w-full min-w-[400px]">
                  {options.map((option) => (
                    <DropdownMenuItem 
                      key={option.id} 
                      onClick={() => setSelectedOption(option.id)}
                      className="flex flex-col items-start p-3"
                    >
                      <div className="font-medium">{option.label}</div>
                      <div className="text-xs text-muted-foreground">
                        {option.type === 'subtitle' ? 'Extract from embedded subtitles' : 'AI transcription'}
                      </div>
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuContent>
              </DropdownMenu>
            </div>

            <div className="flex justify-end gap-3">
              <button
                onClick={handleClose}
                disabled={isRegenerating}
                className="px-4 py-2 text-sm border border-input bg-background text-foreground rounded hover:bg-accent focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Cancel
              </button>
              <button
                onClick={handleRegenerate}
                disabled={isRegenerating || !selectedOption}
                className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
              >
                {isRegenerating ? (
                  <>
                    <svg className="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                    Regenerating...
                  </>
                ) : (
                  'Regenerate'
                )}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  )
}