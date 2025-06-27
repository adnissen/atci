import React from 'react';

interface TranscriptViewProps {
  visible?: boolean;
  name: string;
  className?: string;
  searchTerm?: string;
  text?: string;
  loading?: boolean;
  error?: string | null;
}

// Extend Window interface for our custom handlers
declare global {
  interface Window {
    handleTimestampHover?: (name: string, timestamp: string) => void;
    handleTimestampLeave?: () => void;
    handleCameraIconHover?: (name: string, time1: string, time2: string) => void;
    handleCameraIconLeave?: () => void;
  }
}

const TranscriptView: React.FC<TranscriptViewProps> = ({
  visible = false,
  name,
  className = '',
  searchTerm = '',
  text = '',
  loading = false,
  error = null
}) => {
  if (!visible) {
    return null;
  }

  const [hoveredTimestamp, setHoveredTimestamp] = React.useState<string | null>(null);
  const [thumbnailUrl, setThumbnailUrl] = React.useState<string | null>(null);
  const [hoveredTimeRange, setHoveredTimeRange] = React.useState<{time1: string, time2: string} | null>(null);

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

  // Handle timestamp link hover
  const handleTimestampHover = (name: string, timestamp: string) => {
    const seconds = timestampToSeconds(timestamp);
    setHoveredTimestamp(timestamp);
    setThumbnailUrl(`/frame/${encodeURIComponent(name)}/${seconds}`);
  };

  const handleTimestampLeave = () => {
    setHoveredTimestamp(null);
    setThumbnailUrl(null);
  };

  // Check if a line contains two timestamps in the format "time1 --> time2"
  const hasTimeRange = (line: string): { hasRange: boolean; time1?: string; time2?: string } => {
    const timestampRegex = /\d{2}:\d{2}:\d{2}\.\d{3}/g;
    const timestamps = [...new Set(line.match(timestampRegex))];
    if (timestamps && timestamps.length >= 2) {
      // Check if the line contains "-->"
      if (line.includes('-->')) {
        return {
          hasRange: true,
          time1: timestamps[0],
          time2: timestamps[1]
        };
      }
    }
    
    return { hasRange: false };
  };

  // Filter content to show only lines with search term and 1 line above
  const filterContentForSearch = (text: string, searchTerm: string): string => {
    if (!searchTerm.trim()) {
      return text;
    }

    const lines = text.split('\n');
    const filteredLines: string[] = [];
    const searchTermLower = searchTerm.toLowerCase();

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lineLower = line.toLowerCase();
      
      // Check if current line contains search term
      if (lineLower.includes(searchTermLower)) {
        // Add previous line if it exists and we haven't already added it
        if (i > 0 && !filteredLines.includes(lines[i - 1])) {
          filteredLines.push(lines[i - 1]);
        }
        // Add current line
        filteredLines.push(line);
      }
    }

    return filteredLines.join('\n');
  };

  // Highlight search term in text
  const highlightSearchTerm = (text: string, searchTerm: string): string => {
    if (!searchTerm.trim()) {
      return text;
    }

    // temporarily turn off highlighting until we move to a real "block" system for displaying the transcripts
    const regex = new RegExp(`(${searchTerm.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    return text;
  };

  // Process content to replace timestamps with clickable links
  const processContentWithTimestamps = (text: string): string => {
    // Regex to match timestamp format 00:00:00.000
    const timestampRegex = /(\d{2}:\d{2}:\d{2}\.\d{3})/g;
    
    return text.replace(timestampRegex, (match) => {
      const seconds = timestampToSeconds(match);
      return `<a href="/player/${encodeURIComponent(name)}?time=${seconds}" class="text-blue-600 hover:text-blue-800 underline cursor-pointer timestamp-link" data-timestamp="${match}" onmouseover="window.handleTimestampHover('${name}', '${match}')" onmouseout="window.handleTimestampLeave()">${match}</a>`;
    });
  };

  // Process content to add camera icons to lines with time ranges
  const processContentWithCameraIcons = (text: string): string => {
    const lines = text.split('\n');
    const processedLines = lines.map((line, index) => {
      const timeRangeInfo = hasTimeRange(line);
      const nextLine = index < lines.length - 1 ? lines[index + 1] : null;
      
      if (timeRangeInfo.hasRange && timeRangeInfo.time1 && timeRangeInfo.time2) {
        const seconds1 = timestampToSeconds(timeRangeInfo.time1); //ie 100
        const seconds2 = timestampToSeconds(timeRangeInfo.time2); //ie 104
        const delta = seconds2 - seconds1; //ie 4
        const middle = seconds1 + (delta / 2.0); //ie 102
        const cameraIcon = `<span class="inline-flex items-center ml-2 cursor-pointer text-gray-600 hover:text-blue-600 transition-colors" style="vertical-align: text-bottom; display: inline-flex; align-items: baseline;"><a href="/frame/${encodeURIComponent(name)}/${middle}?text=${nextLine}" target="_blank" class="inline-flex items-baseline"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="inline-block" style="vertical-align: text-bottom;"><path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"/><circle cx="12" cy="13" r="3"/></svg></a></span>`;
        return line + cameraIcon;
      }
      
      return line;
    });
    
    return processedLines.join('\n');
  };

  // Process content to add video camera icons to lines with time ranges
  const processContentWithVideoIcons = (text: string): string => {
    const lines = text.split('\n');
    const processedLines = lines.map((line, index) => {
      const timeRangeInfo = hasTimeRange(line);
      const nextLine = index < lines.length - 1 ? lines[index + 1] : null;
      
      if (timeRangeInfo.hasRange && timeRangeInfo.time1 && timeRangeInfo.time2) {
        const seconds1 = timestampToSeconds(timeRangeInfo.time1);
        const seconds2 = timestampToSeconds(timeRangeInfo.time2);
        const videoIcon = `<span class="inline-flex items-center ml-2 cursor-pointer text-gray-600 hover:text-blue-600 transition-colors" style="vertical-align: text-bottom; display: inline-flex; align-items: baseline;"><a href="/clip?filename=${encodeURIComponent(name)}&start_time=${seconds1}&end_time=${seconds2}" target="_blank" class="inline-flex items-baseline"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="inline-block" style="vertical-align: text-bottom;"><polygon points="23 7 16 12 23 17 23 7"/><rect x="1" y="5" width="15" height="14" rx="2" ry="2"/></svg></a></span>`;
        return line + videoIcon;
      }
      
      return line;
    });
    
    return processedLines.join('\n');
  };

  // Set up global handlers for the dynamically created links
  React.useEffect(() => {
    window.handleTimestampHover = handleTimestampHover;
    window.handleTimestampLeave = handleTimestampLeave;
    return () => {
      delete window.handleTimestampHover;
      delete window.handleTimestampLeave;
    };
  }, []);

  // Process content: filter for search, highlight search term, then add timestamp links
  const filteredContent = filterContentForSearch(text, searchTerm);
  const highlightedContent = highlightSearchTerm(filteredContent, searchTerm);
  const processedContent = processContentWithTimestamps(highlightedContent);
  const processedWithCameraIcons = processContentWithCameraIcons(processedContent);
  const finalContent = processContentWithVideoIcons(processedWithCameraIcons);

  return (
    <div className={`w-full p-6 bg-white ${className}`}>
      <div className="space-y-4">
        {loading && (
          <div className="text-gray-600">Loading transcript...</div>
        )}
        {error && (
          <div className="text-red-600">Error: {error}</div>
        )}
        {!loading && !error && (
          <div className="text-gray-600 text-left relative">
            {searchTerm && filteredContent.trim() === '' && (
              <div className="text-gray-500 italic">
                No matches found for "{searchTerm}" in this transcript.
              </div>
            )}
            {filteredContent.trim() !== '' && (
              <pre 
                className="text-left whitespace-pre-wrap font-mono text-sm leading-relaxed max-w-none overflow-x-auto"
                dangerouslySetInnerHTML={{ __html: finalContent }}
              />
            )}
            
            {/* Thumbnail overlay */}
            {(hoveredTimestamp || hoveredTimeRange) && thumbnailUrl && (
              <div 
                className="fixed z-50 bg-white border border-gray-300 rounded-lg shadow-lg p-2"
                style={{
                  left: '50%',
                  top: '20px',
                  transform: 'translateX(-50%)',
                  maxWidth: '200px',
                  maxHeight: '150px'
                }}
              >
                <img 
                  src={thumbnailUrl} 
                  alt={`Frame at ${hoveredTimestamp || hoveredTimeRange?.time1}`}
                  className="w-full h-auto object-contain"
                  onError={(e) => {
                    e.currentTarget.style.display = 'none';
                  }}
                />
                <div className="text-xs text-gray-600 text-center mt-1">
                  {hoveredTimeRange ? `${hoveredTimeRange.time1} → ${hoveredTimeRange.time2}` : hoveredTimestamp}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default TranscriptView; 