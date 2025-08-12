import React from 'react';
import { Edit2, Camera, Video } from 'lucide-react';
import DualEditDialog from './DualEditDialog';
import ClipMenu from './ClipMenu';
import ClipPlayer from './ClipPlayer';
import { addTimestamp } from '../lib/utils';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from './ui/dropdown-menu';

interface TranscriptBlockProps {
  startTime?: string;
  endTime?: string;
  visible: boolean;
  text: string;
  name: string;
  isSearchResult?: boolean;
  lineNumbers: number[];
  onEditSuccess?: () => void;
  fullTranscript?: string; // Full transcript text for editing
  isSmallScreen?: boolean;
  onSetRightPaneUrl?: (component: React.ReactNode | null, fallbackUrl?: string) => void;
  clipStart?: number | null;
  clipEnd?: number | null;
  clipTranscript?: string | null;
  onSetClipStart?: (time: number) => void;
  onSetClipEnd?: (time: number) => void;
  onClearClip?: () => void;
  onClipBlock?: (startTime: number, endTime: number) => void;
}

// Helper function to convert seconds to timestamp format
const secondsToTimestamp = (seconds: number): string => {
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const remainingSeconds = seconds % 60
  const wholeSeconds = Math.floor(remainingSeconds)
  const milliseconds = Math.round((remainingSeconds - wholeSeconds) * 1000)
  
  return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${wholeSeconds.toString().padStart(2, '0')}.${milliseconds.toString().padStart(3, '0')}`
}

const TranscriptBlock: React.FC<TranscriptBlockProps> = ({
  startTime,
  endTime,
  visible,
  text,
  name,
  isSearchResult = false,
  lineNumbers,
  onEditSuccess,
  fullTranscript = '',
  isSmallScreen: _isSmallScreen = false,
  onSetRightPaneUrl,
  clipStart,
  clipEnd,
  clipTranscript,
  onSetClipStart,
  onSetClipEnd,
  onClearClip,
  onClipBlock
}) => {
  const [isEditing, setIsEditing] = React.useState(false);
  const [isSubmitting, setIsSubmitting] = React.useState(false);
  const [isEditingTimestamp, setIsEditingTimestamp] = React.useState(false);
  const [hoveredLineNumber, setHoveredLineNumber] = React.useState<number | null>(null);
  const [activeDropdown, setActiveDropdown] = React.useState<'timestamp' | 'content' | null>(null);
  const [clipMenuOpen, setClipMenuOpen] = React.useState(false);
  const [selectedTimestamp, setSelectedTimestamp] = React.useState<{time: number, type: 'start' | 'end'} | null>(null);

  // Convert timestamp format 00:00:00.000 to seconds
  const timestampToSeconds = (timestamp: string): number => {
    const parts = timestamp.split(':');
    const hours = parseInt(parts[0], 10);
    const minutes = parseInt(parts[1], 10);
    const secondsParts = parts[2].split('.');
    const seconds = parseInt(secondsParts[0], 10);
    const milliseconds = parseInt(secondsParts[1] || '0', 10);
    
    return hours * 3600 + minutes * 60 + seconds + milliseconds / 1000;
  };

  // Check if this block contains clip start or end times
  const blockContainsClipStart = React.useMemo(() => {
    if (!startTime || !endTime || clipStart === null || clipStart === undefined || clipTranscript !== name) return false;
    const blockStart = timestampToSeconds(startTime);
    const blockEnd = timestampToSeconds(endTime);
    return clipStart >= blockStart && clipStart <= blockEnd;
  }, [startTime, endTime, clipStart, clipTranscript, name]);

  const blockContainsClipEnd = React.useMemo(() => {
    if (!startTime || !endTime || clipEnd === null || clipEnd === undefined || clipTranscript !== name) return false;
    const blockStart = timestampToSeconds(startTime);
    const blockEnd = timestampToSeconds(endTime);
    return clipEnd >= blockStart && clipEnd <= blockEnd;
  }, [startTime, endTime, clipEnd, clipTranscript, name]);

  // Check if this block falls completely within the clip range
  const blockWithinClipRange = React.useMemo(() => {
    if (!startTime || !endTime || clipStart === null || clipStart === undefined || clipEnd === null || clipEnd === undefined || clipTranscript !== name) return false;
    const blockStart = timestampToSeconds(startTime);
    const blockEnd = timestampToSeconds(endTime);
    return blockStart >= clipStart && blockEnd <= clipEnd;
  }, [startTime, endTime, clipStart, clipEnd, clipTranscript, name]);

  const hasClipHighlight = blockContainsClipStart || blockContainsClipEnd || blockWithinClipRange;

  // Handle timestamp clicks
  const handleTimestampClick = (time: number, type: 'start' | 'end') => {
    // Always show menu when timestamp is clicked
    setSelectedTimestamp({time, type});
    setClipMenuOpen(true);
  };

  // Sync with props
  React.useEffect(() => {
    // No need to sync editedText anymore since we're using fullTranscript
  }, [text]);

  React.useEffect(() => {
    // No need to sync editedTimestamp anymore since we're using fullTranscript
  }, [startTime, endTime]);

  // Handle clicks on timestamp links in processed content
  React.useEffect(() => {
    const handleTimestampClick = (event: Event) => {
      const target = event.target as HTMLElement;
      if (target.classList.contains('timestamp-link')) {
        event.preventDefault();
        const url = target.getAttribute('data-url');
        if (url && onSetRightPaneUrl) {
          onSetRightPaneUrl(url);
        }
      }
    };

    document.addEventListener('click', handleTimestampClick);
    return () => {
      document.removeEventListener('click', handleTimestampClick);
    };
  }, [onSetRightPaneUrl]);

  if (!visible || text === "WEBVTT") {
    return null;
  }

  // Process content to replace timestamps with clickable spans
  const processContentWithTimestamps = (text: string): string => {
    // Regex to match timestamp format 00:00:00.000
    const timestampRegex = /(\d{2}:\d{2}:\d{2}\.\d{3})/g;
    
    return text.replace(timestampRegex, (match) => {
      const seconds = timestampToSeconds(match);
      return `<span class="text-sky-700 hover:text-sky-600 underline cursor-pointer timestamp-link" data-timestamp="${match}" data-url="/player/${encodeURIComponent(name)}?time=${seconds}">${match}</span>`;
    });
  };

  // Process the text content (only timestamps, no icons)
  const processedText = processContentWithTimestamps(text);



  // Handle edit action
  const handleEdit = () => {
    setIsEditing(true);
  };

  // Handle save edit - now using replace transcript method
  const handleSaveEdit = async (newText: string) => {
    if (newText.trim() === fullTranscript.trim()) {
      setIsEditing(false);
      return;
    }

    setIsSubmitting(true);
    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');

      const response = await fetch(addTimestamp(`/transcripts/${encodeURIComponent(name)}/replace`), {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ text: newText }),
      });

      if (response.ok) {
        // Call onEditSuccess callback to refresh transcript data
        if (onEditSuccess) {
          onEditSuccess();
        }
        setIsEditing(false);
      } else {
        const error = await response.json();
        alert(`Error: ${error.error || 'Failed to update transcript'}`);
      }
    } catch (error) {
      console.error('Error updating transcript:', error);
      alert('Error: Failed to update transcript. Please try again.');
    } finally {
      setIsSubmitting(false);
    }
  };

  // Handle cancel edit
  const handleCancelEdit = () => {
    setIsEditing(false);
  };

  // Handle edit timestamp
  const handleEditTimestamp = () => {
    setIsEditingTimestamp(true);
  };

  // Handle save timestamp edit - now using replace transcript method
  const handleSaveTimestampEdit = async (newText: string) => {
    if (newText.trim() === fullTranscript.trim()) {
      setIsEditingTimestamp(false);
      return;
    }

    setIsSubmitting(true);
    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');

      const response = await fetch(addTimestamp(`/transcripts/${encodeURIComponent(name)}/replace`), {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ text: newText }),
      });

      if (response.ok) {
        // Call onEditSuccess callback to refresh transcript data
        if (onEditSuccess) {
          onEditSuccess();
        }
        setIsEditingTimestamp(false);
      } else {
        const error = await response.json();
        alert(`Error: ${error.error || 'Failed to update timestamp'}`);
      }
    } catch (error) {
      console.error('Error updating timestamp:', error);
      alert('Error: Failed to update timestamp. Please try again.');
    } finally {
      setIsSubmitting(false);
    }
  };

  // Handle cancel timestamp edit
  const handleCancelTimestampEdit = () => {
    setIsEditingTimestamp(false);
  };

  // Only return early if we have both start and end times and they're equal
  if (startTime && endTime && startTime === endTime) {
    return <></>;
  }

  // Determine which line number to show
  const timestampLineNumber = lineNumbers[0];
  const contentLineNumber = lineNumbers[lineNumbers.length - 1];

  // Determine background and border styling
  const getContainerClasses = () => {
    const baseClasses = [];
    
    if (isSearchResult && hasClipHighlight) {
      // Both search and clip highlighting - combine both
      baseClasses.push('bg-gradient-to-r', 'from-primary/10', 'to-amber-500/10', 'border-l-4', 'border-primary', 'pl-2');
    } else if (isSearchResult) {
      baseClasses.push('bg-primary/10', 'border-l-4', 'border-primary', 'pl-2');
    } else if (hasClipHighlight) {
      baseClasses.push('bg-amber-500/10', 'border-l-4', 'border-amber-500', 'pl-2');
    }
    
    return baseClasses.join(' ');
  };

  return (
              <div className={getContainerClasses()}>
        {hasClipHighlight && (
          <div className="flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400 mb-1">
            <div className="flex items-center gap-1">
              {blockContainsClipStart && (
                <span className="bg-amber-500/20 px-1 py-0.5 rounded text-amber-700 dark:text-amber-300 font-medium">
                  Clip Start
                </span>
              )}
              {blockContainsClipEnd && (
                <span className="bg-amber-500/20 px-1 py-0.5 rounded text-amber-700 dark:text-amber-300 font-medium">
                  Clip End
                </span>
              )}
            </div>
          </div>
        )}
        {startTime && endTime && (
          <div className="group">
            <span className="relative">
              {/* Timestamp line */}
              <div className="flex items-center gap-1 min-w-0">
                <div className="text-muted-foreground text-sm font-mono flex items-center gap-2 min-w-0">      
                  <span 
                    className={`text-xs mr-2 flex-shrink-0 text-right w-8 cursor-pointer transition-colors duration-200 ${
                      hoveredLineNumber === timestampLineNumber || activeDropdown === 'timestamp' 
                        ? 'text-blue-600 hover:text-blue-700' 
                        : 'text-muted-foreground hover:text-blue-500'
                    }`}
                    onClick={() => setActiveDropdown(activeDropdown === 'timestamp' ? null : 'timestamp')}
                    onMouseEnter={() => setHoveredLineNumber(timestampLineNumber)}
                    onMouseLeave={() => setHoveredLineNumber(null)}
                  >
                    {timestampLineNumber}
                  </span>
                  <span className="min-w-0">
                    <ClipMenu
                      open={clipMenuOpen && selectedTimestamp?.type === 'start'}
                      onOpenChange={(open) => {
                        setClipMenuOpen(open);
                        if (!open) setSelectedTimestamp(null);
                      }}
                      selectedTime={selectedTimestamp?.time || (startTime ? timestampToSeconds(startTime) : 0)}
                      clipStart={clipStart ?? null}
                      clipEnd={clipEnd ?? null}
                      clipTranscript={clipTranscript ?? null}
                      currentTranscript={name}
                      onSetClipStart={onSetClipStart || (() => {})}
                      onSetClipEnd={onSetClipEnd || (() => {})}
                      onClearClip={onClearClip || (() => {})}

                      onClipBlock={onClipBlock ? (blockStart, blockEnd) => onClipBlock(blockStart, blockEnd) : undefined}
                      blockStartTime={startTime ? timestampToSeconds(startTime) : undefined}
                      blockEndTime={endTime ? timestampToSeconds(endTime) : undefined}
                    >
                      <span 
                        onClick={() => startTime && handleTimestampClick(timestampToSeconds(startTime), 'start')}
                        className="text-sky-700 hover:text-sky-600 underline cursor-pointer"
                        onMouseEnter={() => setHoveredLineNumber(timestampLineNumber)}
                        onMouseLeave={() => setHoveredLineNumber(null)}
                      >
                        {startTime}
                      </span>
                    </ClipMenu>
                    {' --> '}
                    <ClipMenu
                      open={clipMenuOpen && selectedTimestamp?.type === 'end'}
                      onOpenChange={(open) => {
                        setClipMenuOpen(open);
                        if (!open) setSelectedTimestamp(null);
                      }}
                      selectedTime={selectedTimestamp?.time || (endTime ? timestampToSeconds(endTime) : 0)}
                      clipStart={clipStart ?? null}
                      clipEnd={clipEnd ?? null}
                      clipTranscript={clipTranscript ?? null}
                      currentTranscript={name}
                      onSetClipStart={onSetClipStart || (() => {})}
                      onSetClipEnd={onSetClipEnd || (() => {})}
                      onClearClip={onClearClip || (() => {})}

                      onClipBlock={onClipBlock ? (blockStart, blockEnd) => onClipBlock(blockStart, blockEnd) : undefined}
                      blockStartTime={startTime ? timestampToSeconds(startTime) : undefined}
                      blockEndTime={endTime ? timestampToSeconds(endTime) : undefined}
                    >
                      <span 
                        onClick={() => endTime && handleTimestampClick(timestampToSeconds(endTime), 'end')}
                        className="text-sky-700 hover:text-sky-600 underline cursor-pointer"
                        onMouseEnter={() => setHoveredLineNumber(timestampLineNumber)}
                        onMouseLeave={() => setHoveredLineNumber(null)}
                      >
                        {endTime}
                      </span>
                    </ClipMenu>
                  </span>
                </div>
              </div>
              
              {/* Content line */}
              <div className="flex items-center gap-1 min-w-0">
                <div className="text-muted-foreground text-sm font-mono flex items-center gap-2 min-w-0 flex-1">
                  <span 
                    className={`text-xs mr-2 flex-shrink-0 text-right w-8 cursor-pointer transition-colors duration-200 ${
                      hoveredLineNumber === contentLineNumber || activeDropdown === 'content' 
                        ? 'text-blue-600 hover:text-blue-700' 
                        : 'text-muted-foreground hover:text-blue-500'
                    }`}
                    onClick={() => setActiveDropdown(activeDropdown === 'content' ? null : 'content')}
                    onMouseEnter={() => setHoveredLineNumber(contentLineNumber)}
                    onMouseLeave={() => setHoveredLineNumber(null)}
                  >
                    {contentLineNumber}
                  </span>
                  <div 
                    className="text-foreground font-mono text-sm leading-relaxed whitespace-pre-wrap break-words flex-1 min-w-0"
                    dangerouslySetInnerHTML={{ __html: processedText }}
                    onMouseEnter={() => setHoveredLineNumber(contentLineNumber)}
                    onMouseLeave={() => setHoveredLineNumber(null)}
                  />
                </div>
              </div>
              
              {/* Dropdown menu - positioned relative to clicked line number */}
              {activeDropdown && (
                <div className="absolute left-0 top-0 z-10 dropdown-menu-container">
                  <DropdownMenu modal={false} open={activeDropdown === 'timestamp' || activeDropdown === 'content'} onOpenChange={(open) => !open && setActiveDropdown(null)}>
                    <DropdownMenuTrigger asChild>
                      <div className="invisible"></div>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start" side="right" sideOffset={8}>
                      <DropdownMenuItem asChild>
                        <a 
                          href={`/frame/${encodeURIComponent(name)}/${timestampToSeconds(startTime) + (timestampToSeconds(endTime) - timestampToSeconds(startTime)) / 2}?text=${encodeURIComponent(text)}`} 
                          target="_blank"
                          className="flex items-center gap-2 w-full"
                        >
                          <Camera size={16} className="text-[#6d28d9]" />
                          View Frame
                        </a>
                      </DropdownMenuItem>
                      <DropdownMenuItem 
                        onClick={() => {
                          if (startTime && endTime) {
                            // Play current block
                            const fallbackUrl = `/clip_player/${encodeURIComponent(name)}?start_time=${timestampToSeconds(startTime)}&end_time=${timestampToSeconds(endTime)}&text=${encodeURIComponent(text)}&display_text=false`
                            const clipPlayerComponent = (
                              <div className="w-full flex-1 overflow-y-auto scrollbar-hide">
                                <ClipPlayer
                                  key={`${name}-${timestampToSeconds(startTime)}-${timestampToSeconds(endTime)}`}
                                  filename={name}
                                  start_time_formatted={secondsToTimestamp(timestampToSeconds(startTime))}
                                  end_time_formatted={secondsToTimestamp(timestampToSeconds(endTime))}
                                  font_size=""
                                  text={text}
                                  display_text={false}
                                  onBack={() => {
                                    onSetRightPaneUrl && onSetRightPaneUrl(null)
                                  }}
                                />
                              </div>
                            )
                            onSetRightPaneUrl && onSetRightPaneUrl(clipPlayerComponent, fallbackUrl);
                          }
                        }}
                        className="flex items-center gap-2"
                      >
                        <Video size={16} className="text-[#be185d]" />
                        Play Block
                      </DropdownMenuItem>
                      <DropdownMenuItem 
                        onClick={() => handleEditTimestamp()}
                        className="flex items-center gap-2"
                      >
                        <Edit2 size={16} className="text-[#3b82f6]" />
                        Edit Line
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              )}
            </span>
          </div>
        )}
        {text != undefined && !startTime && !endTime && (
          <div className="group">
            <span className="relative">
              {/* Content line */}
              <div className="flex items-center gap-1 min-w-0">
                <div className="text-muted-foreground text-sm font-mono flex items-center gap-2 min-w-0 flex-1">
                  <span 
                    className={`text-xs mr-2 flex-shrink-0 text-right w-8 cursor-pointer transition-colors duration-200 ${
                      hoveredLineNumber === contentLineNumber || activeDropdown === 'content' 
                        ? 'text-blue-600 hover:text-blue-700' 
                        : 'text-muted-foreground hover:text-blue-500'
                    }`}
                    onClick={() => setActiveDropdown(activeDropdown === 'content' ? null : 'content')}
                    onMouseEnter={() => setHoveredLineNumber(contentLineNumber)}
                    onMouseLeave={() => setHoveredLineNumber(null)}
                  >
                    {contentLineNumber}
                  </span>
                  <div 
                    className="text-foreground font-mono text-sm leading-relaxed whitespace-pre-wrap break-words flex-1 min-w-0"
                    dangerouslySetInnerHTML={{ __html: processedText }}
                    onMouseEnter={() => setHoveredLineNumber(contentLineNumber)}
                    onMouseLeave={() => setHoveredLineNumber(null)}
                  />
                </div>
              </div>

              {/* Dropdown menu - positioned relative to clicked line number */}
              {activeDropdown === 'content' && (
                <div className="absolute left-0 top-0 z-10 dropdown-menu-container">
                  <DropdownMenu modal={false} open={true} onOpenChange={(open) => !open && setActiveDropdown(null)}>
                    <DropdownMenuTrigger asChild>
                      <div className="invisible"></div>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start" side="right" sideOffset={8}>
                      <DropdownMenuItem
                        onClick={handleEdit}
                        className="flex items-center gap-2"
                      >
                        <Edit2 size={16} className="text-[#3b82f6]" />
                        Edit Line
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              )}
            </span>
          </div>
        )}
      
      
      {/* Edit Dialog for Content */}
      <DualEditDialog
        isOpen={isEditing}
        filename={name}
        transcriptInitialValue={fullTranscript}
        metaInitialValue=""
        onTranscriptSave={handleSaveEdit}
        onMetaSave={() => {}}
        onCancel={handleCancelEdit}
        isTranscriptSubmitting={isSubmitting}
        transcriptTargetLineNumber={contentLineNumber}
      />

      {/* Edit Dialog for Timestamp */}
      <DualEditDialog
        isOpen={isEditingTimestamp}
        filename={name}
        transcriptInitialValue={fullTranscript}
        metaInitialValue=""
        onTranscriptSave={handleSaveTimestampEdit}
        onMetaSave={() => {}}
        onCancel={handleCancelTimestampEdit}
        isTranscriptSubmitting={isSubmitting}
        transcriptTargetLineNumber={timestampLineNumber}
      />
    </div>
  );
};

export default TranscriptBlock; 