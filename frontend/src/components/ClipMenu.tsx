import React from 'react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from './ui/dropdown-menu';
import { Scissors, Play, X } from 'lucide-react';

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
  onPlayClip?: () => void;
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
  onPlayClip,
  children
}) => {
  // Determine if we should grey out options
  const isSetStartDisabled = clipEnd !== null && selectedTime >= clipEnd;
  const isSetEndDisabled = clipStart !== null && selectedTime <= clipStart;
  const hasClip = clipStart !== null && clipEnd !== null && clipTranscript === currentTranscript;

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
        
        {hasClip && onPlayClip && (
          <DropdownMenuItem 
            onClick={onPlayClip}
            className="flex items-center gap-2"
          >
            <Play size={16} className="text-blue-600" />
            Play Clip
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