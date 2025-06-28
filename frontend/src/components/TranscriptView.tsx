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
  isSearchResult?: boolean;
  contextBefore?: number;
  contextAfter?: number;
  originalIndex?: number;
  hasMoreBefore?: boolean;
  hasMoreAfter?: boolean;
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
  const [expandedContext, setExpandedContext] = React.useState<Record<number, {before: number, after: number}>>({});

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

  // Handle context expansion
  const handleExpandContext = (blockIndex: number, direction: 'before' | 'after') => {
    setExpandedContext(prev => {
      const current = prev[blockIndex] || { before: 0, after: 0 };
      const newContext = { ...current };
      
      if (direction === 'before') {
        newContext.before = Math.min(newContext.before + 5, 20); // Max 20 lines before
      } else {
        newContext.after = Math.min(newContext.after + 5, 20); // Max 20 lines after
      }
      
      return { ...prev, [blockIndex]: newContext };
    });
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
    const allBlocks: TranscriptBlockData[] = [];
    const searchTermLower = searchTerm.toLowerCase();

    // First pass: create all blocks with their original indices
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
            
            allBlocks.push({
              startTime: timeRangeInfo.time1,
              endTime: timeRangeInfo.time2,
              text: nextLine,
              visible: !searchTerm || isSearchResult,
              isSearchResult,
              originalIndex: allBlocks.length
            });
            
            // Skip the next line since we've already processed it
            i++;
          }
        } else {
          // Same start and end time, treat as regular text
          const textLower = line.toLowerCase();
          const isSearchResult = searchTerm ? textLower.includes(searchTermLower) : false;
          
          allBlocks.push({
            text: line,
            visible: !searchTerm || isSearchResult,
            isSearchResult,
            originalIndex: allBlocks.length
          });
        }
      } else {
        // Regular text line
        const textLower = line.toLowerCase();
        const isSearchResult = searchTerm ? textLower.includes(searchTermLower) : false;
        
        allBlocks.push({
          text: line,
          visible: !searchTerm || isSearchResult,
          isSearchResult,
          originalIndex: allBlocks.length
        });
      }
    }

    // If no search term, return all blocks as visible
    if (!searchTerm) {
      return allBlocks;
    }

    // Second pass: add context blocks around search results
    const resultBlocks: TranscriptBlockData[] = [];

    for (let i = 0; i < allBlocks.length; i++) {
      const block = allBlocks[i];
      
      if (block.isSearchResult) {
        const contextBefore = expandedContext[block.originalIndex!]?.before || 0;
        const contextAfter = expandedContext[block.originalIndex!]?.after || 0;
        
        // Check if there are more blocks available before the current context
        // When no context is expanded, check if there are any blocks before this search result
        const hasMoreBefore = contextBefore > 0 ? i > contextBefore : i > 1;
        
        // Check if there are more blocks available after the current context
        // When no context is expanded, check if there are any blocks after this search result
        const hasMoreAfter = contextAfter > 0 ? i + contextAfter + 1 < allBlocks.length : i + 1 < allBlocks.length;
        
        // Add context blocks before this search result
        if (contextBefore > 0) {
          const startIndex = Math.max(0, i - contextBefore);
          for (let j = startIndex; j < i; j++) {
            const contextBlock = { ...allBlocks[j], visible: true, isSearchResult: false };
            resultBlocks.push(contextBlock);
          }
        }
        
        // Add the search result block with context availability info
        const searchResultBlock = {
          ...block,
          hasMoreBefore,
          hasMoreAfter
        };
        resultBlocks.push(searchResultBlock);
        
        // Add context blocks after this search result
        if (contextAfter > 0) {
          const endIndex = Math.min(allBlocks.length, i + contextAfter + 1);
          for (let j = i + 1; j < endIndex; j++) {
            const contextBlock = { ...allBlocks[j], visible: true, isSearchResult: false };
            resultBlocks.push(contextBlock);
          }
        }
      }
    }

    return resultBlocks;
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
    parseTranscriptBlocks(text, searchTerm), 
    [text, searchTerm, expandedContext]
  );

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
            {transcriptBlocks.map((block, index) => (
              <div key={`${block.originalIndex}-${index}`}>
                {/* Up caret for expanding context before */}
                {block.isSearchResult && block.hasMoreBefore && (
                      <div className="flex justify-center mb-2">
                        <button
                          onClick={() => handleExpandContext(block.originalIndex!, 'before')}
                          className="text-gray-500 hover:text-blue-600 transition-colors p-2 rounded-full hover:bg-blue-50"
                          title="Show more context above"
                        >
                          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                          </svg>
                        </button>
                      </div>
                    )}
                    
                    {/* Show indicator when no more context above */}
                    {block.isSearchResult && !block.hasMoreBefore && (
                      <div className="flex justify-center mb-2">
                        <div className="text-xs text-gray-400 px-2 py-1 bg-gray-50 rounded-full">
                          Beginning of transcript
                        </div>
                      </div>
                    )}
              </div>
            ))}
          </div>)}

            {transcriptBlocks.length > 0 && (
              <div className="space-y-2">
                {transcriptBlocks.map((block, index) => (
                  <div key={`${block.originalIndex}-${index}`}>
                    
                    
                    <TranscriptBlock
                      startTime={block.startTime}
                      endTime={block.endTime}
                      visible={block.visible}
                      text={block.text}
                      name={name}
                      isSearchResult={block.isSearchResult}
                    />
                    
                    
                  </div>
                ))}
              </div>
            )}


          {transcriptBlocks.length > 0 && (<div className="space-y-2">
            {transcriptBlocks.map((block, index) => (
              <div key={`${block.originalIndex}-${index}`}>
                {/* Show indicator when no more context below */}
                    {block.isSearchResult && !block.hasMoreAfter && (
                      <div className="flex justify-center mt-2">
                        <div className="text-xs text-gray-400 px-2 py-1 bg-gray-50 rounded-full">
                          End of transcript
                        </div>
                      </div>
                    )}
                    
                    {/* Down caret for expanding context after */}
                    {block.isSearchResult && block.hasMoreAfter && (
                      <div className="flex justify-center mt-2">
                        <button
                          onClick={() => handleExpandContext(block.originalIndex!, 'after')}
                          className="text-gray-500 hover:text-blue-600 transition-colors p-2 rounded-full hover:bg-blue-50"
                          title="Show more context below"
                        >
                          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                          </svg>
                        </button>
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