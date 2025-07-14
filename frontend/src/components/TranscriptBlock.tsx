import React from 'react';
import { Edit2 } from 'lucide-react';
import DualEditDialog from './DualEditDialog';

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
  fullTranscript = ''
}) => {
  const [isEditing, setIsEditing] = React.useState(false);
  const [isSubmitting, setIsSubmitting] = React.useState(false);
  const [isEditingTimestamp, setIsEditingTimestamp] = React.useState(false);

  // Sync with props
  React.useEffect(() => {
    // No need to sync editedText anymore since we're using fullTranscript
  }, [text]);

  React.useEffect(() => {
    // No need to sync editedTimestamp anymore since we're using fullTranscript
  }, [startTime, endTime]);

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

  // Process content to replace timestamps with clickable links
  const processContentWithTimestamps = (text: string): string => {
    // Regex to match timestamp format 00:00:00.000
    const timestampRegex = /(\d{2}:\d{2}:\d{2}\.\d{3})/g;
    
    return text.replace(timestampRegex, (match) => {
      const seconds = timestampToSeconds(match);
      return `<a href="/player/${encodeURIComponent(name)}?time=${seconds}" class="text-sky-700 hover:text-sky-600 underline cursor-pointer timestamp-link" data-timestamp="${match}">${match}</a>`;
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

      const response = await fetch(`/transcripts/${encodeURIComponent(filename)}/partial_reprocess`, {
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

      const response = await fetch(`/transcripts/${encodeURIComponent(name)}/replace`, {
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

      const response = await fetch(`/transcripts/${encodeURIComponent(name)}/replace`, {
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
              <div className="grid grid-cols-24 gap-1">
                <div className="col-span-1 text-muted-foreground text-sm font-mono flex items-center gap-2"></div>
                <div className="col-span-23 text-muted-foreground text-sm font-mono flex items-center gap-2">      
                  <span className="text-muted-foreground text-xs mr-2 flex-shrink-0 text-right w-8">{timestampLineNumber}</span>
                  <span>
                    <a 
                      href={`/player/${encodeURIComponent(name)}?time=${timestampToSeconds(startTime)}`}
                      className="text-sky-700 hover:text-sky-600 underline cursor-pointer"
                      target="_blank"
                    >
                      {startTime}
                    </a>
                    {' --> '}
                    <a 
                      href={`/player/${encodeURIComponent(name)}?time=${timestampToSeconds(endTime)}`}
                      className="text-sky-700 hover:text-sky-600 underline cursor-pointer"
                      target="_blank"
                    >
                      {endTime}
                    </a>
                  </span>
                </div>
              </div>
              
              {/* Content line */}
              <div className="grid grid-cols-24 gap-1">
                <div className="col-span-1 text-muted-foreground text-sm font-mono flex items-center gap-2"></div>
                <div className="col-span-23 text-muted-foreground text-sm font-mono flex items-center gap-2">
                  <span className="text-muted-foreground text-xs mr-2 flex-shrink-0 text-right w-8">{contentLineNumber}</span>
                  <div className="w-4"></div>
                  <div 
                    className="text-foreground font-mono text-sm leading-relaxed whitespace-pre-wrap break-words w-full"
                    dangerouslySetInnerHTML={{ __html: processedText }}
                  />
                </div>
              </div>
              
              {/* Icons - positioned at the exact midpoint between timestamp and content */}
              <div className="absolute left-0 top-1/2 transform -translate-y-1/2 z-10 opacity-0 group-hover:opacity-100 transition-opacity">
                <div className="grid grid-cols-2 gap-1">
                  {/* Camera icon */}
                  <span className="inline-flex items-center cursor-pointer text-muted-foreground">
                    <a href={`/frame/${encodeURIComponent(name)}/${timestampToSeconds(startTime) + (timestampToSeconds(endTime) - timestampToSeconds(startTime)) / 2}?text=${encodeURIComponent(text)}`} target="_blank" className="inline-flex items-baseline">
                      <div className="border-2 border-[#6d28d9] rounded-full w-6 h-6 flex items-center justify-center hover:bg-[#6d28d9]/10 transition-colors z-50 relative">
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block hover:stroke-[#6d28d9] stroke-[#6d28d9]">
                          <path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"/>
                          <circle cx="12" cy="13" r="3"/>
                        </svg>
                      </div>
                    </a>
                  </span>
                  
                  {/* Video icon */}
                  <span className="inline-flex items-center cursor-pointer text-muted-foreground">
                    <a href={`/clip_player/${encodeURIComponent(name)}?start_time=${timestampToSeconds(startTime)}&end_time=${timestampToSeconds(endTime)}&text=${encodeURIComponent(text)}&display_text=false`} target="_blank" className="inline-flex items-baseline">
                      <div className="border-2 border-[#be185d] rounded-full w-6 h-6 flex items-center justify-center hover:bg-[#be185d]/10 transition-colors z-50 relative">
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block hover:stroke-[#be185d] stroke-[#be185d]">
                          <polygon points="23 7 16 12 23 17 23 7"/>
                          <rect x="1" y="5" width="15" height="14" rx="2" ry="2"/>
                        </svg>
                      </div>
                    </a>
                  </span>
                  
                  {/* Regenerate icon */}
                  <span className="inline-flex items-center cursor-pointer text-muted-foreground">
                    <button 
                      onClick={() => handleRegenerate(name, startTime)}
                      className="inline-flex items-baseline p-0 border-none bg-transparent"
                      title={`Regenerate transcript from ${startTime}`}
                    >
                      <div className="border-2 border-[#059669] rounded-full w-6 h-6 flex items-center justify-center hover:bg-[#059669]/10 transition-colors z-50 relative">
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block hover:stroke-[#059669] stroke-[#059669]">
                          <polyline points="23 4 23 10 17 10"/>
                          <polyline points="1 20 1 14 7 14"/>
                          <path d="m20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15"/>
                        </svg>
                      </div>
                    </button>
                  </span>
                  
                  {/* Edit icon for timestamp line */}
                  <span className="inline-flex items-center cursor-pointer text-muted-foreground">
                    <button 
                      onClick={() => handleEditTimestamp()}
                      className="inline-flex items-baseline p-0 border-none bg-transparent"
                      title="Edit this timestamp line"
                    >
                      <div className="border-2 border-[#3b82f6] rounded-full w-6 h-6 flex items-center justify-center hover:bg-[#3b82f6]/10 transition-colors z-50 relative">
                        <Edit2 size={12} className="hover:stroke-[#3b82f6] stroke-[#3b82f6]" />
                      </div>
                    </button>
                  </span>
                </div>
              </div>
            </span>
          </div>
        )}
        {text != undefined && !startTime && !endTime && (
          <div className="grid grid-cols-24 gap-1 group">

            <div className="col-span-1 text-muted-foreground text-sm font-mono flex items-center gap-2">
              <div className="flex items-center gap-2 justify-end">
                {/* Edit icon for content lines IF there's no start and end time */}
                <span className="inline-flex items-center cursor-pointer text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity">
                  <button 
                    onClick={handleEdit}
                    className="inline-flex items-baseline p-0 border-none bg-transparent"
                    title="Edit this line"
                  >
                    <div className="border-2 border-[#3b82f6] rounded-full w-7 h-7 flex items-center justify-center hover:bg-[#3b82f6]/10 transition-colors z-50 relative">
                      <Edit2 size={14} className="hover:stroke-[#3b82f6] stroke-[#3b82f6]" />
                    </div>
                  </button>
                </span>
              </div>
            </div>
            <div className="col-span-23 text-muted-foreground text-sm font-mono flex items-center gap-2">
            <span className="text-muted-foreground text-xs mr-2 flex-shrink-0 text-right w-8">{contentLineNumber}</span>
            <div className="w-4"></div>
            <div 
                className="text-foreground font-mono text-sm leading-relaxed whitespace-pre-wrap break-words w-full"
                dangerouslySetInnerHTML={{ __html: processedText }}
            />
          </div>
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