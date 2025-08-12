import React, { useState, useEffect, useRef } from 'react'
import { Card, CardContent } from './ui/card'
import { Button } from './ui/button'
import { Input } from './ui/input'
import { Textarea } from './ui/textarea'
import { Checkbox } from './ui/checkbox'
import { Download, ChevronLeft } from 'lucide-react'

interface ClipPlayerProps {
  filename: string
  clip_url?: string
  start_time_formatted?: string
  end_time_formatted?: string
  font_size?: string
  text?: string
  display_text?: boolean
  onBack?: () => void
}

const ClipPlayer: React.FC<ClipPlayerProps> = ({
  filename,
  clip_url,
  start_time_formatted = '00:00:00.000',
  end_time_formatted = '00:00:10.000',
  font_size = '',
  text = '',
  display_text = false,
  onBack
}) => {
  const [startTime, setStartTime] = useState(start_time_formatted)
  const [endTime, setEndTime] = useState(end_time_formatted)
  const [fontSize, setFontSize] = useState(font_size)
  const [textOverlay, setTextOverlay] = useState(text)
  const [showTextOverlay, setShowTextOverlay] = useState(display_text)
  const [isLoading, setIsLoading] = useState(false)
  const [currentClipUrl, setCurrentClipUrl] = useState(clip_url || '')

  const videoRef = useRef<HTMLVideoElement>(null)
  const updateTimeoutRef = useRef<NodeJS.Timeout | null>(null)

  // Convert timestamp (HH:MM:SS.mmm) to seconds
  const timestampToSeconds = (timestamp: string): number => {
    const parts = timestamp.split(':')
    const hours = parseInt(parts[0], 10)
    const minutes = parseInt(parts[1], 10)
    const seconds = parseFloat(parts[2])
    return hours * 3600 + minutes * 60 + seconds
  }

  // Validate timestamp format
  const isValidTimestamp = (timestamp: string): boolean => {
    const timestampRegex = /^(\d{2}):(\d{2}):(\d{2})\.(\d{3})$/
    return timestampRegex.test(timestamp)
  }

  // Build clip URL with parameters
  const buildClipUrl = (format = 'mp4'): string => {
    if (!startTime || !endTime || !isValidTimestamp(startTime) || !isValidTimestamp(endTime)) {
      return ''
    }

    const startTimeSeconds = timestampToSeconds(startTime)
    const endTimeSeconds = timestampToSeconds(endTime)
    
    if (startTimeSeconds < 0 || endTimeSeconds <= startTimeSeconds) {
      return ''
    }

    const clipParams = new URLSearchParams({
      filename: filename,
      start_time: startTimeSeconds.toString(),
      end_time: endTimeSeconds.toString(),
      format: format,
      cachebuster: Date.now().toString()
    })

    // Add optional parameters
    if (textOverlay && textOverlay.trim() !== '') {
      clipParams.set('text', textOverlay)
    }
    if (fontSize && fontSize.trim() !== '') {
      clipParams.set('font_size', fontSize)
    }
    if (showTextOverlay) {
      clipParams.set('display_text', 'true')
    }

    return '/clip?' + clipParams.toString()
  }

  // Generate filename for downloads
  const generateFilename = (format: string): string => {
    let filename = `clip.${format}`
    
    if (textOverlay && textOverlay.trim() !== '') {
      const sanitizedText = textOverlay.trim()
        .substring(0, 50)
        .replace(/[^a-zA-Z0-9\s\-_]/g, '')
        .replace(/\s+/g, '_')
      if (sanitizedText) {
        filename = sanitizedText + `.${format}`
      }
    }
    
    return filename
  }

  // Update video and download links
  const updateVideo = () => {
    if (!isValidTimestamp(startTime) || !isValidTimestamp(endTime)) {
      return
    }

    const startSeconds = timestampToSeconds(startTime)
    const endSeconds = timestampToSeconds(endTime)
    if (startSeconds < 0 || endSeconds <= startSeconds) {
      return
    }

   
    const newClipUrl = buildClipUrl('mp4')
    if (newClipUrl) {
      setCurrentClipUrl(newClipUrl)
    }
  }

  useEffect(() => {
    if (currentClipUrl) {
      videoRef.current?.load()
    }
  }, [currentClipUrl])

  // Debounced update function
  const debouncedUpdate = () => {
    if (updateTimeoutRef.current) {
      clearTimeout(updateTimeoutRef.current)
    }
    updateTimeoutRef.current = setTimeout(updateVideo, 500)
  }

  // Handle video loading events
  const handleVideoLoad = () => {
    setIsLoading(false)
    if (videoRef.current) {
      window.scrollTo({ top: 0, behavior: 'smooth' })
    }
  }

  const handleVideoError = () => {
    setIsLoading(false)
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }

  // Validate timestamp input
  const validateTimestampInput = (value: string, setter: (value: string) => void) => {
    setter(value)
    // Note: Video reloading is now handled by useEffect below
  }

  // Update state when props change
  useEffect(() => {
    setStartTime(start_time_formatted)
    setEndTime(end_time_formatted)
    setFontSize(font_size)
    setTextOverlay(text)
    setShowTextOverlay(display_text)
    setCurrentClipUrl(clip_url || '')
  }, [start_time_formatted, end_time_formatted, font_size, text, display_text, clip_url])

  // Reload video when both start and end times are valid and complete
  useEffect(() => {
    if (isValidTimestamp(startTime) && isValidTimestamp(endTime)) {
      updateVideo()
    }
  }, [startTime, endTime])

  // Initialize video on mount and when key props change
  useEffect(() => {
    updateVideo()
    return () => {
      if (updateTimeoutRef.current) {
        clearTimeout(updateTimeoutRef.current)
      }
    }
  }, [filename, start_time_formatted, end_time_formatted])

  return (
    <div className="container max-w-6xl mx-auto p-4 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-lg font-semibold">{filename}</h2>
        {onBack && (
          <button
            onClick={onBack}
            className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
          >
            <ChevronLeft className="h-4 w-4" />
            Close
          </button>
        )}
      </div>
      
      {/* Clip Info */}
      <div className="text-center mb-4">
        <div className="text-lg font-medium text-muted-foreground">
          {startTime} to {endTime}
        </div>
      </div>

      {/* Video Container */}
      <Card className="relative overflow-hidden bg-black">
        <CardContent className="p-0">
          {isLoading && (
            <div className="absolute inset-0 flex flex-col items-center justify-center min-h-[300px] bg-black/95 backdrop-blur-sm z-10">
              <div className="w-12 h-12 border-3 border-muted border-t-primary rounded-full animate-spin mb-6" />
              <div className="text-lg font-semibold text-primary-foreground mb-2">Processing...</div>
              <div className="text-sm text-muted-foreground">Generating your clip, please wait</div>
            </div>
          )}
          {currentClipUrl && (
            <video
              ref={videoRef}
              controls
              autoPlay
              className="w-full h-auto block"
              onLoadedData={handleVideoLoad}
              onError={handleVideoError}
            >
              <source src={currentClipUrl} type="video/mp4" />
              Your browser does not support the video tag.
            </video>
          )}
        </CardContent>
      </Card>

      {/* Controls Form */}
      <Card>
        <CardContent className="p-6 space-y-6">
          {/* Time Controls */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="space-y-2">
              <label htmlFor="start_time" className="text-sm font-medium">
                Start Time (hh:mm:ss.sss)
              </label>
              <Input
                id="start_time"
                type="text"
                value={startTime}
                onChange={(e) => validateTimestampInput(e.target.value, setStartTime)}
                pattern="^(\d{2}):(\d{2}):(\d{2})\.(\d{3})$"
                placeholder="00:00:00.000"
                className="font-mono text-sm tracking-wider"
                required
              />
            </div>
            <div className="space-y-2">
              <label htmlFor="end_time" className="text-sm font-medium">
                End Time (hh:mm:ss.sss)
              </label>
              <Input
                id="end_time"
                type="text"
                value={endTime}
                onChange={(e) => validateTimestampInput(e.target.value, setEndTime)}
                pattern="^(\d{2}):(\d{2}):(\d{2})\.(\d{3})$"
                placeholder="00:00:00.000"
                className="font-mono text-sm tracking-wider"
                required
              />
            </div>
            <div className="space-y-2">
              <label htmlFor="font_size" className="text-sm font-medium">
                Font Size
              </label>
              <Input
                id="font_size"
                type="number"
                value={fontSize}
                onChange={(e) => {
                  setFontSize(e.target.value)
                  debouncedUpdate()
                }}
                min="10"
                max="500"
                placeholder="Auto"
              />
            </div>
          </div>

          {/* Text Overlay */}
          <div className="space-y-2">
            <label htmlFor="text" className="text-sm font-medium">
              Text Overlay
            </label>
            <Textarea
              id="text"
              value={textOverlay}
              onChange={(e) => {
                setTextOverlay(e.target.value)
                debouncedUpdate()
              }}
              placeholder="Enter text to overlay on video"
              className="min-h-[100px] resize-y"
            />
          </div>

          {/* Show Text Overlay Checkbox */}
          <div className="flex items-center space-x-3">
            <Checkbox
              id="display_text"
              checked={showTextOverlay}
              onCheckedChange={(checked) => {
                setShowTextOverlay(checked as boolean)
                updateVideo() // Immediate update for checkbox
              }}
            />
            <label htmlFor="display_text" className="text-sm font-medium cursor-pointer">
              Show Text Overlay
            </label>
          </div>

          {/* Download Buttons */}
          <div className="flex flex-col gap-3 items-center pt-4">
            <Button asChild className="bg-emerald-600 hover:bg-emerald-700">
              <a
                href={buildClipUrl('mp4')}
                download={generateFilename('mp4')}
                className="inline-flex items-center gap-2"
              >
                <Download className="w-4 h-4" />
                Download MP4
              </a>
            </Button>
            <Button asChild className="bg-emerald-600 hover:bg-emerald-700">
              <a
                href={buildClipUrl('gif')}
                download={generateFilename('gif')}
                className="inline-flex items-center gap-2"
              >
                <Download className="w-4 h-4" />
                Download GIF
              </a>
            </Button>
            <Button asChild className="bg-emerald-600 hover:bg-emerald-700">
              <a
                href={buildClipUrl('mp3')}
                download={generateFilename('mp3')}
                className="inline-flex items-center gap-2"
              >
                <Download className="w-4 h-4" />
                Download MP3
              </a>
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

export default ClipPlayer