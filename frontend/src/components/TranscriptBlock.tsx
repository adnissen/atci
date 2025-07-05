import React from 'react';
import { Edit2 } from 'lucide-react';

interface TranscriptBlockProps {
  startTime?: string;
  endTime?: string;
  visible: boolean;
  text: string;
  name: string;
  isSearchResult?: boolean;
  lineNumbers: number[];
  onEditSuccess?: () => void;
}

const TranscriptBlock: React.FC<TranscriptBlockProps> = ({
  startTime,
  endTime,
  visible,
  text,
  name,
  isSearchResult = false,
  lineNumbers,
  onEditSuccess
}) => {
  const [isEditing, setIsEditing] = React.useState(false);
  const [editedText, setEditedText] = React.useState(text);
  const [isSubmitting, setIsSubmitting] = React.useState(false);
  const [isEditingTimestamp, setIsEditingTimestamp] = React.useState(false);
  const [editedTimestamp, setEditedTimestamp] = React.useState(`${startTime} --> ${endTime}`);

  // Sync editedText with text prop
  React.useEffect(() => {
    setEditedText(text);
  }, [text]);

  // Sync editedTimestamp with startTime and endTime props
  React.useEffect(() => {
    setEditedTimestamp(`${startTime} --> ${endTime}`);
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
    setEditedText(text);
    setIsEditing(true);
  };

  // Handle save edit
  const handleSaveEdit = async () => {
    if (editedText.trim() === text.trim()) {
      setIsEditing(false);
      return;
    }

    setIsSubmitting(true);
    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');
      const contentLineNumber = lineNumbers[lineNumbers.length - 1]; // Use the content line number

      const response = await fetch(`/transcripts/${encodeURIComponent(name)}/set_line`, {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ 
          line_number: contentLineNumber.toString(), 
          text: editedText 
        }),
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
    setEditedText(text);
    setIsEditing(false);
  };

  // Handle edit timestamp
  const handleEditTimestamp = () => {
    setEditedTimestamp(`${startTime} --> ${endTime}`);
    setIsEditingTimestamp(true);
  };

  // Handle save timestamp edit
  const handleSaveTimestampEdit = async () => {
    if (editedTimestamp.trim() === `${startTime} --> ${endTime}`.trim()) {
      setIsEditingTimestamp(false);
      return;
    }

    setIsSubmitting(true);
    try {
      const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');
      const lineNumber = timestampLineNumber; // Use the timestamp line number

      const response = await fetch(`/transcripts/${encodeURIComponent(name)}/set_line`, {
        method: 'POST',
        headers: {
          'X-CSRF-Token': csrfToken || '',
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ 
          line_number: lineNumber.toString(), 
          text: editedTimestamp 
        }),
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
    setEditedTimestamp(`${startTime} --> ${endTime}`);
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
    <div className={`mb-2 group ${isSearchResult ? 'bg-yellow-50 dark:bg-yellow-950/20 border-l-4 border-yellow-400 dark:border-yellow-600 pl-2' : ''}`}>
        {startTime && endTime && (
                <div className="grid grid-cols-12 gap-1">

          <div className="col-span-1 text-muted-foreground text-sm font-mono flex items-center gap-2">
            <div className="flex items-center gap-2 justify-start">
              {/* Camera icon */}
              <span className="inline-flex items-center cursor-pointer text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity">
                <a href={`/frame/${encodeURIComponent(name)}/${timestampToSeconds(startTime) + (timestampToSeconds(endTime) - timestampToSeconds(startTime)) / 2}?text=${encodeURIComponent(text)}`} target="_blank" className="inline-flex items-baseline">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block hover:stroke-[#6d28d9]">
                    <path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"/>
                    <circle cx="12" cy="13" r="3"/>
                  </svg>
                </a>
              </span>
              
              {/* Video icon */}
              <span className="inline-flex items-center cursor-pointer text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity">
                <a href={`/clip?filename=${encodeURIComponent(name)}&start_time=${timestampToSeconds(startTime)}&end_time=${timestampToSeconds(endTime)}`} target="_blank" className="inline-flex items-baseline">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block hover:stroke-[#be185d]">
                    <polygon points="23 7 16 12 23 17 23 7"/>
                    <rect x="1" y="5" width="15" height="14" rx="2" ry="2"/>
                  </svg>
                </a>
              </span>
              
              {/* Regenerate icon */}
              <span className="inline-flex items-center cursor-pointer text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity">
                <button 
                  onClick={() => handleRegenerate(name, startTime)}
                  className="inline-flex items-baseline p-0 border-none bg-transparent"
                  title={`Regenerate transcript from ${startTime}`}
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="inline-block hover:stroke-[#059669]">
                    <polyline points="23 4 23 10 17 10"/>
                    <polyline points="1 20 1 14 7 14"/>
                    <path d="m20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15"/>
                  </svg>
                </button>
              </span>
              
              {/* Edit icon for timestamp line */}
              <span className="inline-flex items-center cursor-pointer text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity">
                <button 
                  onClick={() => handleEditTimestamp()}
                  className="inline-flex items-baseline p-0 border-none bg-transparent"
                  title="Edit this timestamp line"
                >
                  <Edit2 size={16} className="hover:stroke-[#3b82f6]" />
                </button>
              </span>
            </div>
          </div>
          <div className="col-span-11 text-muted-foreground text-sm font-mono flex items-center gap-2">      
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
        )}
        {text != undefined && (
          <div className="grid grid-cols-12 gap-1">

            <div className="col-span-1 text-muted-foreground text-sm font-mono flex items-center gap-2">
            <div className="flex items-center gap-2 justify-end">
              {/* Edit icon for content lines */}
              <span className="inline-flex items-center cursor-pointer text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity">
                <button 
                  onClick={handleEdit}
                  className="inline-flex items-baseline p-0 border-none bg-transparent"
                  title="Edit this line"
                >
                  <Edit2 size={16} className="hover:stroke-[#3b82f6]" />
                </button>
              </span>
            </div>
            </div>
            <div className="col-span-11 text-muted-foreground text-sm font-mono flex items-center gap-2">
            <span className="text-muted-foreground text-xs mr-2 flex-shrink-0 text-right w-8">{contentLineNumber}</span>
            <div className="w-4"></div>
            <div 
                className="text-foreground font-mono text-sm leading-relaxed whitespace-pre-wrap break-words w-full"
                dangerouslySetInnerHTML={{ __html: processedText }}
            />
          </div>
          </div>
        )}
      
      
      {/* Edit Dialog */}
      {isEditing && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 p-6 rounded-lg max-w-2xl w-full mx-4">
            <h3 className="text-lg font-semibold mb-4">Edit Line {contentLineNumber}</h3>
            <textarea
              value={editedText}
              onChange={(e) => setEditedText(e.target.value)}
              className="w-full h-32 p-3 border border-gray-300 dark:border-gray-600 rounded-md font-mono text-sm"
              placeholder="Enter text..."
            />
            <div className="flex justify-end gap-2 mt-4">
              <button
                onClick={handleCancelEdit}
                disabled={isSubmitting}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 border border-gray-300 rounded-md hover:bg-gray-200 disabled:opacity-50"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveEdit}
                disabled={isSubmitting}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {isSubmitting ? 'Saving...' : 'Save'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Edit Timestamp Dialog */}
      {isEditingTimestamp && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 p-6 rounded-lg max-w-2xl w-full mx-4">
            <h3 className="text-lg font-semibold mb-4">Edit Timestamp Line {timestampLineNumber}</h3>
            <input
              type="text"
              value={editedTimestamp}
              onChange={(e) => setEditedTimestamp(e.target.value)}
              className="w-full p-3 border border-gray-300 dark:border-gray-600 rounded-md font-mono text-sm"
              placeholder="00:00:00.000 --> 00:00:00.000"
            />
            <div className="flex justify-end gap-2 mt-4">
              <button
                onClick={handleCancelTimestampEdit}
                disabled={isSubmitting}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 border border-gray-300 rounded-md hover:bg-gray-200 disabled:opacity-50"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveTimestampEdit}
                disabled={isSubmitting}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {isSubmitting ? 'Saving...' : 'Save'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default TranscriptBlock; 