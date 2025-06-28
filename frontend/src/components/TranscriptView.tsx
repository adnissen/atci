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
  visibleLines?: number[];
  expandContext: (filename: string, direction: "up" | "down", line: number) => void;
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
  isSearchResult?: boolean;
  originalIndex?: number;
  lineNumbers: number[];
}

const TranscriptView: React.FC<TranscriptViewProps> = ({
  visible = false,
  name,
  className = '',
  searchTerm = '',
  text = '',
  loading = false,
  error = null,
  visibleLines = [],
  expandContext
}) => {
  if (!visible) {
    return null;
  }

  const [hoveredTimestamp, setHoveredTimestamp] = React.useState<string | null>(null);
  const [thumbnailUrl, setThumbnailUrl] = React.useState<string | null>(null);

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
  const parseTranscriptBlocks = (text: string, visibleLines: number[]): TranscriptBlockData[] => {
    const lines = text.split('\n');
    const allBlocks: TranscriptBlockData[] = [];
    const searchTermLower = searchTerm.toLowerCase();

    // First pass: create all blocks with their original indices and line numbers
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
            const isSearchResult = searchTerm ? textLower.includes(searchTermLower) : false;
            const lineNumbers = [i + 1, i + 2]; // Timestamp line and text line
            const isVisible = visibleLines.length === 0 || lineNumbers.some(lineNum => visibleLines.includes(lineNum));
            
            allBlocks.push({
              startTime: timeRangeInfo.time1,
              endTime: timeRangeInfo.time2,
              text: nextLine,
              visible: isVisible,
              isSearchResult,
              originalIndex: allBlocks.length,
              lineNumbers
            });
            
            // Skip the next line since we've already processed it
            i++;
          }
        } else {
          // Same start and end time, treat as regular text
          const textLower = line.toLowerCase();
          const isSearchResult = searchTerm ? textLower.includes(searchTermLower) : false;
          const lineNumbers = [i + 1];
          const isVisible = visibleLines.length === 0 || lineNumbers.some(lineNum => visibleLines.includes(lineNum));
          
          allBlocks.push({
            text: line,
            visible: isVisible,
            isSearchResult,
            originalIndex: allBlocks.length,
            lineNumbers
          });
        }
      } else {
        // Regular text line
        const textLower = line.toLowerCase();
        const isSearchResult = searchTerm ? textLower.includes(searchTermLower) : false;
        const lineNumbers = [i + 1];
        const isVisible = visibleLines.length === 0 || lineNumbers.some(lineNum => visibleLines.includes(lineNum));
        
        allBlocks.push({
          text: line,
          visible: isVisible,
          isSearchResult,
          originalIndex: allBlocks.length,
          lineNumbers
        });
      }
    }

    // Return all blocks - visibility is handled by the visible property
    return allBlocks;
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
  const transcriptBlocks = React.useMemo(() => 
    parseTranscriptBlocks(text, visibleLines), 
    [text, visibleLines]
  );

  // Process blocks to add "x lines..." messages between visible blocks
  const processedBlocks = React.useMemo(() => {
    if (visibleLines.length === 0) {
      // If no visible lines filter, return all blocks as is
      return transcriptBlocks.map(block => ({ type: 'block' as const, data: block }));
    }

    const result: Array<{ type: 'block' | 'message'; data: TranscriptBlockData | { count: number, line: number, direction: "up" | "down" } }> = [];
    let hiddenCount = 0;

    for (let i = 0; i < transcriptBlocks.length; i++) {
      const block = transcriptBlocks[i];
      
      if (block.visible) {
        // If we have accumulated hidden blocks, add a message
        if (hiddenCount > 0) {
          result.push({ type: 'message', data: { count: hiddenCount, line: block.lineNumbers[block.lineNumbers.length - 1], direction: "up" } });
          hiddenCount = 0;
        }
        // Add the visible block
        result.push({ type: 'block', data: block });
      } else {
        // Count hidden blocks
        hiddenCount += block.lineNumbers.length;
      }
    }

    // If there are hidden blocks at the end, add a final message
    if (hiddenCount > 0) {
      // For "down" expansion, use a line number that's guaranteed to be in the current visible lines
      // Use the last line number from the sorted visible lines array
      const sortedVisibleLines = [...visibleLines].sort((a, b) => b - a); // Sort descending
      const downExpansionLine = sortedVisibleLines.length > 0 ? sortedVisibleLines[0] : transcriptBlocks[transcriptBlocks.length - 1].lineNumbers[0];
      result.push({ type: 'message', data: { count: hiddenCount, line: downExpansionLine, direction: "down" } });
    }

    return result;
  }, [transcriptBlocks, visibleLines]);

  const handleExpandClick = (line: number, direction: "up" | "down") => {
    expandContext(name, direction, line);
  }

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
            {searchTerm && transcriptBlocks.filter(block => block.isSearchResult).length === 0 && (
              <div className="text-gray-500 italic">
                No matches found for "{searchTerm}" in this transcript.
              </div>
            )}

          {transcriptBlocks.length > 0 && (<div className="space-y-2">
            {processedBlocks.map((item, index) => (
              <div key={item.type === 'block' ? `${(item.data as TranscriptBlockData).originalIndex}-${index}` : `message-${index}`}>
                {item.type === 'block' ? (
                  <TranscriptBlock
                    startTime={(item.data as TranscriptBlockData).startTime}
                    endTime={(item.data as TranscriptBlockData).endTime}
                    visible={(item.data as TranscriptBlockData).visible}
                    text={(item.data as TranscriptBlockData).text}
                    name={name}
                    isSearchResult={(item.data as TranscriptBlockData).isSearchResult}
                  />
                ) : (
                  <div className="text-gray-500 italic" onClick={() => handleExpandClick((item.data as { count: number, line: number, direction: "up" | "down" }).line, (item.data as { count: number, line: number, direction: "up" | "down" }).direction)}>
                    {(item.data as { count: number, line: number, direction: "up" | "down" }).count} lines hidden
                  </div>
                )}
              </div>
            ))}
          </div>)}

            {/* Thumbnail overlay */}
            {(hoveredTimestamp) && thumbnailUrl && (
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
                  alt={`Frame at ${hoveredTimestamp}`}
                  className="w-full h-auto object-contain"
                  onError={(e) => {
                    e.currentTarget.style.display = 'none';
                  }}
                />
                <div className="text-xs text-gray-600 text-center mt-1">
                  {hoveredTimestamp}
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