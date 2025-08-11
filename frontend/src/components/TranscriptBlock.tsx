import React from 'react';
import { Edit2, Camera, Video, RotateCcw } from 'lucide-react';
import DualEditDialog from './DualEditDialog';
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
  onSetRightPaneUrl?: (url: string) => void;
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
  onSetRightPaneUrl
}) => {
  const [isEditing, setIsEditing] = React.useState(false);
  const [isSubmitting, setIsSubmitting] = React.useState(false);
  const [isEditingTimestamp, setIsEditingTimestamp] = React.useState(false);
  const [hoveredLineNumber, setHoveredLineNumber] = React.useState<number | null>(null);
  const [activeDropdown, setActiveDropdown] = React.useState<'timestamp' | 'content' | null>(null);

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

  // Handle regenerate action
  const handleRegenerate = async (filename: string, time: string) => {
    if (!time) return;
    
    const confirmed = window.confirm(`Are you sure you want to regenerate the transcript from ${time}? This will reprocess the video from this timestamp onwards.`);
    if (!confirmed) return;

    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content')

      const response = await fetch(addTimestamp(`/transcripts/${encodeURIComponent(filename)}/partial_reprocess`), {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ time }),
      });

      if (response.ok) {
        // Reload the page to show updated transcript
        window.location.reload();
      } else {
        const error = await response.json();
        alert(`Error: ${error.error || 'Failed to regenerate transcript'}`);
      }
    } catch (error) {
      console.error('Error calling partial reprocess:', error);
      alert('Error: Failed to regenerate transcript. Please try again.');
    }
  };

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

  return (
              <div className={`${isSearchResult ? 'bg-primary/10 border-l-4 border-primary pl-2' : ''}`}>
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
                    <span 
                      onClick={() => onSetRightPaneUrl && onSetRightPaneUrl(`/clip_player/${encodeURIComponent(name)}?start_time=${timestampToSeconds(startTime)}&end_time=${timestampToSeconds(endTime)}&text=${encodeURIComponent(text)}&display_text=false`)}
                      className="text-sky-700 hover:text-sky-600 underline cursor-pointer"
                      onMouseEnter={() => setHoveredLineNumber(timestampLineNumber)}
                      onMouseLeave={() => setHoveredLineNumber(null)}
                    >
                      {startTime}
                    </span>
                    {' --> '}
                    <span 
                      onClick={() => onSetRightPaneUrl && onSetRightPaneUrl(`/clip_player/${encodeURIComponent(name)}?start_time=${timestampToSeconds(startTime)}&end_time=${timestampToSeconds(endTime)}&text=${encodeURIComponent(text)}&display_text=false`)}
                      className="text-sky-700 hover:text-sky-600 underline cursor-pointer"
                      onMouseEnter={() => setHoveredLineNumber(timestampLineNumber)}
                      onMouseLeave={() => setHoveredLineNumber(null)}
                    >
                      {endTime}
                    </span>
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
                        onClick={() => onSetRightPaneUrl && onSetRightPaneUrl(`/clip_player/${encodeURIComponent(name)}?start_time=${timestampToSeconds(startTime)}&end_time=${timestampToSeconds(endTime)}&text=${encodeURIComponent(text)}&display_text=false`)}
                        className="flex items-center gap-2"
                      >
                        <Video size={16} className="text-[#be185d]" />
                        Play Clip
                      </DropdownMenuItem>
                      <DropdownMenuItem 
                        onClick={() => handleRegenerate(name, startTime)}
                        className="flex items-center gap-2"
                      >
                        <RotateCcw size={16} className="text-[#059669]" />
                        Regenerate from {startTime}
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