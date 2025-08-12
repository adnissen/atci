import React from 'react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from './ui/dropdown-menu';
import { Scissors, X } from 'lucide-react';

interface ClipMenuProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  selectedTime: number;
  clipStart: number | null;
  clipEnd: number | null;
  clipTranscript: string | null;
  currentTranscript: string;
  onSetClipStart: (time: number) => void;
  onSetClipEnd: (time: number) => void;
  onClearClip: () => void;
  onClipBlock?: (startTime: number, endTime: number) => void;
  blockStartTime?: number;
  blockEndTime?: number;
  children: React.ReactNode;
}

const ClipMenu: React.FC<ClipMenuProps> = ({
  open,
  onOpenChange,
  selectedTime,
  clipStart,
  clipEnd,
  clipTranscript,
  currentTranscript,
  onSetClipStart,
  onSetClipEnd,
  onClearClip,
  onClipBlock,
  blockStartTime,
  blockEndTime,
  children
}) => {
  // Determine if we should grey out options
  // If clicking on a different transcript, allow all options (cross-transcript selection)
  const isDifferentTranscript = clipTranscript !== null && clipTranscript !== currentTranscript;
  const isSetStartDisabled = !isDifferentTranscript && clipEnd !== null && selectedTime >= clipEnd;
  const isSetEndDisabled = !isDifferentTranscript && clipStart !== null && selectedTime <= clipStart;

  return (
    <DropdownMenu modal={false} open={open} onOpenChange={onOpenChange}>
      <DropdownMenuTrigger asChild>
        {children}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" side="right" sideOffset={8}>
        <DropdownMenuItem 
          onClick={() => onSetClipStart(selectedTime)}
          disabled={isSetStartDisabled}
          className={`flex items-center gap-2 ${isSetStartDisabled ? 'opacity-50 cursor-not-allowed' : ''}`}
        >
          <Scissors size={16} className="text-green-600" />
          Set Clip Start
        </DropdownMenuItem>
        
        <DropdownMenuItem 
          onClick={() => onSetClipEnd(selectedTime)}
          disabled={isSetEndDisabled}
          className={`flex items-center gap-2 ${isSetEndDisabled ? 'opacity-50 cursor-not-allowed' : ''}`}
        >
          <Scissors size={16} className="text-red-600" />
          Set Clip End
        </DropdownMenuItem>
        
        {onClipBlock && blockStartTime !== undefined && blockEndTime !== undefined && (
          <DropdownMenuItem 
            onClick={() => onClipBlock(blockStartTime, blockEndTime)}
            className="flex items-center gap-2"
          >
            <Scissors size={16} className="text-purple-600" />
            Clip Block
          </DropdownMenuItem>
        )}
        

        
        {(clipStart !== null || clipEnd !== null) && (
          <DropdownMenuItem 
            onClick={onClearClip}
            className="flex items-center gap-2"
          >
            <X size={16} className="text-gray-600" />
            Clear Clip
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default ClipMenu;
