import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuItem,
} from '../components/ui/dropdown-menu'
import { ChevronLeft } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { addTimestamp } from '../lib/utils'

type QueueItem = {
  process_type: string
  path: string
  time?: string
}

type QueueStatus = {
  queue: QueueItem[]
  current_processing?: QueueItem | null
  processing_state: string
}

interface QueuePageProps {
  onClose?: () => void;
}

export default function QueuePage({ onClose }: QueuePageProps = {}) {
  const navigate = useNavigate()
  const [queueStatus, setQueueStatus] = useState<QueueStatus>({
    queue: [],
    current_processing: null,
    processing_state: 'idle'
  })
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  
  // Fetch queue status
  const fetchQueueStatus = async () => {
    try {
      const response = await fetch(addTimestamp('/api/queue/status'))
      if (response.ok) {
        const data = await response.json()
        setQueueStatus(data)
        setError(null)
      } else {
        throw new Error(`Failed to fetch queue status: ${response.status}`)
      }
    } catch (err) {
      console.error('Error fetching queue status:', err)
      setError(err instanceof Error ? err.message : 'Failed to fetch queue status')
    } finally {
      setIsLoading(false)
    }
  }

  // Poll for updates every second
  useEffect(() => {
    fetchQueueStatus()
    const interval = setInterval(fetchQueueStatus, 2000)
    return () => clearInterval(interval)
  }, [])

  // Remove item from queue
  const handleRemoveItem = async (item: QueueItem) => {
    if (!confirm(`Remove "${item.path.split('/').pop()}" from queue?`)) {
      return
    }

    try {
      const response = await fetch('/api/queue/remove', {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          process_type: item.process_type,
          path: item.path,
          time: item.time
        })
      })

      if (response.ok) {
        // Refresh queue status
        await fetchQueueStatus()
      } else {
        const errorData = await response.json().catch(() => ({ message: 'Unknown error' }))
        alert(`Failed to remove item: ${errorData.message}`)
      }
    } catch (err) {
      console.error('Error removing item:', err)
      alert('Failed to remove item from queue')
    }
  }



  // Cancel processing job from queue table
  const handleCancelProcessing = async (item: QueueItem) => {
    if (!confirm(`Cancel processing of "${item.path.split('/').pop()}"?`)) {
      return
    }

    try {
      const response = await fetch('/api/queue/cancel-current', {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        }
      })

      if (response.ok) {
        // Refresh queue status
        await fetchQueueStatus()
      } else {
        const errorData = await response.json().catch(() => ({ message: 'Unknown error' }))
        alert(`Failed to cancel job: ${errorData.message}`)
      }
    } catch (err) {
      console.error('Error cancelling job:', err)
      alert('Failed to cancel processing job')
    }
  }

  // Move item up in queue
  const handleMoveUp = async (index: number) => {
    if (index <= 1) return // Can't move first item (processing) or second item up above processing

    const newQueue = [...queueStatus.queue]
    const [movedItem] = newQueue.splice(index, 1)
    newQueue.splice(index - 1, 0, movedItem)

    await updateQueueOrder(newQueue)
  }

  // Move item down in queue
  const handleMoveDown = async (index: number) => {
    if (index === 0 || index === queueStatus.queue.length - 1) return // Can't move processing item or last item down

    const newQueue = [...queueStatus.queue]
    const [movedItem] = newQueue.splice(index, 1)
    newQueue.splice(index + 1, 0, movedItem)

    await updateQueueOrder(newQueue)
  }

  // Send item to top of queue
  const handleSendToTop = async (index: number) => {
    if (index <= 1) return // Can't move processing item or item already at position #2

    const newQueue = [...queueStatus.queue]
    const [movedItem] = newQueue.splice(index, 1)
    // Insert at position 1 (after the processing item at position 0)
    newQueue.splice(1, 0, movedItem)

    await updateQueueOrder(newQueue)
  }

  // Send item to bottom of queue
  const handleSendToBottom = async (index: number) => {
    if (index === 0 || index === queueStatus.queue.length - 1) return // Can't move processing item or item already at bottom

    const newQueue = [...queueStatus.queue]
    const [movedItem] = newQueue.splice(index, 1)
    newQueue.push(movedItem)

    await updateQueueOrder(newQueue)
  }

  // Update queue order
  const updateQueueOrder = async (newQueue: QueueItem[]) => {
    try {
      const response = await fetch('/api/queue/reorder', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          queue: newQueue.map(item => ({
            process_type: item.process_type,
            path: item.path,
            time: item.time
          }))
        })
      })

      if (response.ok) {
        // Refresh queue status
        await fetchQueueStatus()
      } else {
        const errorData = await response.json().catch(() => ({ message: 'Unknown error' }))
        alert(`Failed to reorder queue: ${errorData.message}`)
      }
    } catch (err) {
      console.error('Error reordering queue:', err)
      alert('Failed to reorder queue')
    }
  }

  // Get display name for file path
  const getDisplayName = (path: string) => {
    return path.split('/').pop()?.replace(/\.(mp4|MP4)$/, '') || path
  }



  const handleBack = () => {
    if (onClose) {
      onClose();
    } else {
      navigate('/');
    }
  };

  if (isLoading) {
    return (
      <div className="h-full overflow-auto">
        <div className="text-center py-8">
          <div className="text-lg text-muted-foreground">Loading queue...</div>
        </div>
      </div>
    )
  }

  return (
    <div className="h-full overflow-auto">
      <div className="bg-card border border-border rounded-lg p-6 h-full">
        <div className="flex items-center justify-between mb-6">
        <h2 className="text-lg font-semibold">Queue</h2>
          <div className="flex items-center gap-4">

          </div>
          {onClose && (
            <button
              onClick={handleBack}
              className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              <ChevronLeft className="h-4 w-4" />
              Close
            </button>
          )}
        </div>

      {error && (
        <div className="mb-6 p-4 bg-destructive/10 border border-destructive/20 rounded-md">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {/* Queue Table */}
      <div className="border border-border rounded-md">
        
        {queueStatus.queue.length === 0 ? (
          <div className="p-8 text-center text-muted-foreground">
            No items in queue
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[15%] text-left">Position</TableHead>
                <TableHead className="w-[65%] text-left">File</TableHead>
                <TableHead className="w-[20%] text-left">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {queueStatus.queue.map((item, index) => (
                <TableRow key={`${item.path}-${item.process_type}-${index}`}>
                  <TableCell className="font-mono text-sm text-left">
                    {index === 0 ? (
                      <span className="inline-flex items-center gap-2">
                        <span className="text-orange-600 dark:text-orange-400">Processing</span>
                        <div className="w-2 h-2 bg-orange-500 rounded-full animate-pulse"></div>
                      </span>
                    ) : (
                      `#${index + 1}`
                    )}
                  </TableCell>
                  <TableCell className="font-medium text-left">
                    <div className="max-w-xs truncate" title={item.path}>
                      {getDisplayName(item.path)}
                    </div>
                  </TableCell>
                  <TableCell className="text-left">
                    {index === 0 ? (
                      <button
                        onClick={() => handleCancelProcessing(item)}
                        className="p-1.5 border border-red-500 text-red-500 bg-transparent rounded hover:bg-red-50 hover:border-red-600 hover:text-red-600 dark:hover:bg-red-950/20 focus:outline-none focus:ring-1 focus:ring-red-500 transition-colors"
                        title="Cancel processing"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                        </svg>
                      </button>
                    ) : (
                      <div className="flex items-center gap-1">
                        {/* Move up button */}
                        <button
                          onClick={() => handleMoveUp(index)}
                          disabled={index === 1} // Disable for position #2 since it can't move above processing item
                          className="p-1 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                          title="Move up"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                          </svg>
                        </button>
                        
                        {/* Move down button */}
                        <button
                          onClick={() => handleMoveDown(index)}
                          disabled={index === queueStatus.queue.length - 1}
                          className="p-1 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                          title="Move down"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                          </svg>
                        </button>
                        
                        {/* More actions dropdown */}
                        <DropdownMenu modal={false}>
                          <DropdownMenuTrigger asChild>
                            <button
                              className="p-1 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors"
                              title="More actions"
                            >
                              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                              </svg>
                            </button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem
                              onClick={() => handleSendToTop(index)}
                              disabled={index === 1} // Can't send to top above processing item
                              className="focus:text-primary"
                            >
                              <span>Send to top</span>
                              <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 10l7-7m0 0l7 7m-7-7v18" />
                              </svg>
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              onClick={() => handleSendToBottom(index)}
                              disabled={index === queueStatus.queue.length - 1}
                              className="focus:text-primary"
                            >
                              <span>Send to bottom</span>
                              <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
                              </svg>
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              onClick={() => handleRemoveItem(item)}
                              className="text-destructive focus:text-destructive"
                            >
                              <span>Remove from queue</span>
                              <svg className="w-4 h-4 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                              </svg>
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </div>
      </div>
    </div>
  )
} 