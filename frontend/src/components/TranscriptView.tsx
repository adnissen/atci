import React from 'react';
import TranscriptBlock from './TranscriptBlock';

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

interface TranscriptBlockData {
  startTime?: string;
  endTime?: string;
  text: string;
  visible: boolean;
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

  // Parse text into transcript blocks
  const parseTranscriptBlocks = (text: string, searchTerm: string): TranscriptBlockData[] => {
    const lines = text.split('\n');
    const blocks: TranscriptBlockData[] = [];
    const searchTermLower = searchTerm.toLowerCase();

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      if (!line) continue;

      const timeRangeInfo = hasTimeRange(line);
      
      if (timeRangeInfo.hasRange && timeRangeInfo.time1 && timeRangeInfo.time2) {
        // Check if start and end times are different
        if (timeRangeInfo.time1 !== timeRangeInfo.time2) {
          // This is a timestamp line, get the next line as text
          const nextLine = i < lines.length - 1 ? lines[i + 1].trim() : '';
          if (nextLine) {
            const textLower = nextLine.toLowerCase();
            const isVisible = !searchTerm || textLower.includes(searchTermLower);
            
            blocks.push({
              startTime: timeRangeInfo.time1,
              endTime: timeRangeInfo.time2,
              text: nextLine,
              visible: isVisible
            });
            
            // Skip the next line since we've already processed it
            i++;
          }
        } else {
          // Same start and end time, treat as regular text
          const textLower = line.toLowerCase();
          const isVisible = !searchTerm || textLower.includes(searchTermLower);
          
          blocks.push({
            text: line,
            visible: isVisible
          });
        }
      } else {
        // Regular text line
        const textLower = line.toLowerCase();
        const isVisible = !searchTerm || textLower.includes(searchTermLower);
        
        blocks.push({
          text: line,
          visible: isVisible
        });
      }
    }

    return blocks;
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

  // Parse the transcript into blocks
  const transcriptBlocks = parseTranscriptBlocks(text, searchTerm);

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
            {searchTerm && transcriptBlocks.filter(block => block.visible).length === 0 && (
              <div className="text-gray-500 italic">
                No matches found for "{searchTerm}" in this transcript.
              </div>
            )}
            {transcriptBlocks.filter(block => block.visible).length > 0 && (
              <div className="space-y-2">
                {transcriptBlocks.map((block, index) => (
                  <TranscriptBlock
                    key={index}
                    startTime={block.startTime}
                    endTime={block.endTime}
                    visible={block.visible}
                    text={block.text}
                    name={name}
                  />
                ))}
              </div>
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