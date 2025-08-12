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
  expandAll?: ((filename: string) => void) | undefined;
  onEditSuccess?: () => void;
  isSmallScreen?: boolean;
  onSetRightPaneUrl?: (url: string) => void;
  clipStart?: number | null;
  clipEnd?: number | null;
  clipTranscript?: string | null;
  onSetClipStart?: (time: number) => void;
  onSetClipEnd?: (time: number) => void;
  onClearClip?: () => void;
  onPlayClip?: () => void;
}

// Extend Window interface for our custom handlers
declare global {
  interface Window {
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
  expandContext,
  expandAll = undefined,
  onEditSuccess,
  isSmallScreen = false,
  onSetRightPaneUrl,
  clipStart,
  clipEnd,
  clipTranscript,
  onSetClipStart,
  onSetClipEnd,
  onClearClip,
  onPlayClip
}) => {
  if (!visible) {
    return null;
  }



  // Check if a line contains two timestamps in the format "time1 --> time2"
  const hasTimeRange = (line: string): { hasRange: boolean; time1?: string; time2?: string } => {
    const timestampRegex = /\d{2}:\d{2}:\d{2}\.\d{3}/g;
    const timestamps = line.match(timestampRegex);
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
    const lines = text.split('\n').flatMap(line => line.split('\r'));
    const allBlocks: TranscriptBlockData[] = [];
    const searchTermLower = searchTerm.toLowerCase();

    // Check if we should show all lines (visibleLines is [-1] or [])
    const showAllLines = (visibleLines.length === 1 && visibleLines[0] === -1) || visibleLines.length === 0;

    // First pass: create all blocks with their original indices and line numbers
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      const timeRangeInfo = hasTimeRange(line);
      
      if (timeRangeInfo.hasRange && timeRangeInfo.time1 && timeRangeInfo.time2) {
        // Check if start and end times are different
        // This is a timestamp line, get the next line as text
        const nextLine = i < lines.length - 1 ? lines[i + 1] : '';
        const textLower = nextLine.toLowerCase();
        const isSearchResult = searchTerm ? textLower.includes(searchTermLower) : false;
        const lineNumbers = [i + 1, i + 2]; // Timestamp line and text line
        const isVisible = showAllLines || lineNumbers.some(lineNum => visibleLines.includes(lineNum));
        
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
      } else {
        // Regular text line (including empty lines)
        const textLower = line.toLowerCase();
        const isSearchResult = searchTerm ? textLower.includes(searchTermLower) : false;
        const lineNumbers = [i + 1];
        const isVisible = showAllLines || lineNumbers.some(lineNum => visibleLines.includes(lineNum));
        
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

  // Parse the transcript into blocks
  const transcriptBlocks = React.useMemo(() => 
    parseTranscriptBlocks(text, visibleLines), 
    [text, visibleLines]
  );

  // Process blocks to add "x lines..." messages between visible blocks
  const processedBlocks = React.useMemo(() => {
    // Check if we should show all lines (visibleLines is [-1])
    const showAllLines = visibleLines.length === 1 && visibleLines[0] === -1;
    
    if (visibleLines.length === 0 || showAllLines) {
      // If no visible lines filter or showAllLines is true, return all blocks as is
      return transcriptBlocks.map(block => ({ type: 'block' as const, data: block }));
    }

    const result: Array<{ type: 'block' | 'message'; data: TranscriptBlockData | { count: number, line: number, direction: "up" | "down" } }> = [];
    let hiddenCount = 0;

    for (let i = 0; i < transcriptBlocks.length; i++) {
      const block = transcriptBlocks[i];
      
      if (block.visible) {
        // If we have accumulated hidden blocks, add a "lines above" message
        if (hiddenCount > 0) {
          result.push({ type: 'message', data: { count: hiddenCount, line: block.lineNumbers[block.lineNumbers.length - 1], direction: "up" } });
          hiddenCount = 0;
        }
        // Add the visible block
        result.push({ type: 'block', data: block });
        
        // Look ahead to see if there are hidden blocks after this visible block
        let lookAheadHiddenCount = 0;
        
        for (let j = i + 1; j < transcriptBlocks.length; j++) {
          const lookAheadBlock = transcriptBlocks[j];
          if (lookAheadBlock.visible) {
            break;
          } else {
            lookAheadHiddenCount += lookAheadBlock.lineNumbers.length;
          }
        }
        
        // If there are hidden blocks before the next visible block, add a "lines below" message
        if (lookAheadHiddenCount > 0) {
          result.push({ type: 'message', data: { count: lookAheadHiddenCount, line: block.lineNumbers[block.lineNumbers.length - 1], direction: "down" } });
        }
      } else {
        // Count hidden blocks
        hiddenCount += block.lineNumbers.length;
      }
    }

    return result;
  }, [transcriptBlocks, visibleLines]);

  const handleExpandClick = (line: number, direction: "up" | "down") => {
    expandContext(name, direction, line);
  }

  const handleExpandAll = () => {
    if (expandAll) {
      expandAll(name);
    }
  }

  return (
    <div className={`w-full ${isSmallScreen ? 'px-4 py-4 bg-transparent' : 'p-3 sm:p-6 bg-card border border-border'} ${className}`}>
      <div className="space-y-4">
        {loading && (
          <div className="text-muted-foreground">Loading transcript...</div>
        )}
        {error && (
          <div className="text-destructive">Error: {error}</div>
        )}
        {!loading && !error && (
          <div className="text-muted-foreground text-left relative">
            {searchTerm && transcriptBlocks.filter(block => block.isSearchResult).length === 0 && (
              <div className="text-muted-foreground italic">
                No matches found for "{searchTerm}" in this transcript.
              </div>
            )}

          {transcriptBlocks.length > 0 && (<div className="space-y-0.5">
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
                    lineNumbers={(item.data as TranscriptBlockData).lineNumbers}
                    onEditSuccess={onEditSuccess}
                    fullTranscript={text}
                    isSmallScreen={isSmallScreen}
                    onSetRightPaneUrl={onSetRightPaneUrl}
                    clipStart={clipStart}
                    clipEnd={clipEnd}
                    clipTranscript={clipTranscript}
                    onSetClipStart={onSetClipStart}
                    onSetClipEnd={onSetClipEnd}
                    onClearClip={onClearClip}
                    onPlayClip={onPlayClip}
                  />
                ) : (
                  <div className="text-muted-foreground italic">
                    <span 
                      className="cursor-pointer hover:text-primary hover:underline"
                      onClick={() => handleExpandClick((item.data as { count: number, line: number, direction: "up" | "down" }).line, (item.data as { count: number, line: number, direction: "up" | "down" }).direction)}
                    >
                      [{(item.data as { count: number, line: number, direction: "up" | "down" }).count} lines {(item.data as { count: number, line: number, direction: "up" | "down" }).direction === "up" ? "above" : "below"}]
                    </span>
                    {" or "}
                    <span 
                      className="cursor-pointer hover:text-primary hover:underline"
                      onClick={handleExpandAll}
                    >
                      [expand all]
                    </span>
                  </div>
                )}
              </div>
            ))}
          </div>)}
          </div>
        )}
      </div>
    </div>
  );
};

export default TranscriptView; 