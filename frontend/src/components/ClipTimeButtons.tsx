import React from 'react'
import { Button } from './ui/button'

interface TimeButtonConfig {
  text: string
  onClick: () => void
  color?: string
  group?: number
}

interface ClipTimeButtonsProps {
  buttons: TimeButtonConfig[]
}

const ClipTimeButtons: React.FC<ClipTimeButtonsProps> = ({ buttons }) => {
  // Group buttons by their group property (default to group 0 if not specified)
  const groupedButtons = buttons.reduce((groups, button, index) => {
    const group = button.group ?? 0
    if (!groups[group]) {
      groups[group] = []
    }
    groups[group].push({ ...button, originalIndex: index })
    return groups
  }, {} as Record<number, Array<TimeButtonConfig & { originalIndex: number }>>)

  // Sort groups by group number
  const sortedGroupKeys = Object.keys(groupedButtons).map(Number).sort((a, b) => a - b)

  return (
    <div className="mt-2 space-y-1">
      {sortedGroupKeys.map((groupKey) => (
        <div key={groupKey} className="flex items-center gap-1 justify-center flex-wrap">
          {groupedButtons[groupKey].map((button) => (
            <Button
              key={button.originalIndex}
              variant="outline"
              size="sm"
              onClick={button.onClick}
              className="h-7 px-2 text-xs font-mono hover:bg-muted/80 transition-colors"
              style={button.color ? {
                borderColor: button.color,
                color: button.color
              } : undefined}
            >
              {button.text}
            </Button>
          ))}
        </div>
      ))}
    </div>
  )
}

export default ClipTimeButtons
