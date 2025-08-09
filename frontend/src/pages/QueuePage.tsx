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

export default function QueuePage() {
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
    const interval = setInterval(fetchQueueStatus, 1000)
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

  // Cancel current job
  const handleCancelCurrent = async () => {
    if (!queueStatus.current_processing) return

    if (!confirm(`Cancel processing of "${queueStatus.current_processing.path.split('/').pop()}"?`)) {
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
      alert('Failed to cancel current job')
    }
  }

  // Move item up in queue
  const handleMoveUp = async (index: number) => {
    if (index === 0) return // Can't move first item up

    const newQueue = [...queueStatus.queue]
    const [movedItem] = newQueue.splice(index, 1)
    newQueue.splice(index - 1, 0, movedItem)

    await updateQueueOrder(newQueue)
  }

  // Move item down in queue
  const handleMoveDown = async (index: number) => {
    if (index === queueStatus.queue.length - 1) return // Can't move last item down

    const newQueue = [...queueStatus.queue]
    const [movedItem] = newQueue.splice(index, 1)
    newQueue.splice(index + 1, 0, movedItem)

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

  // Get color for process type chip
  const getProcessTypeColor = (processType: string) => {
    const colors: Record<string, string> = {
      'all': 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
      'length': 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
      'partial': 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
    }
    return colors[processType] || 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200'
  }

  if (isLoading) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-10">
        <div className="text-center">
          <div className="text-lg text-muted-foreground">Loading queue...</div>
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-7xl mx-auto px-4 py-10">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate('/')}
            className="p-2 text-muted-foreground hover:text-primary hover:bg-accent rounded transition-colors"
            title="Back to files"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
          </button>
          <h1 className="text-2xl font-bold">Processing Queue</h1>
        </div>
        <div className="text-sm text-muted-foreground">
          Status: <span className="font-medium">{queueStatus.processing_state}</span>
        </div>
      </div>

      {error && (
        <div className="mb-6 p-4 bg-destructive/10 border border-destructive/20 rounded-md">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {/* Currently Processing */}
      {queueStatus.current_processing && (
        <div className="mb-6 p-4 bg-accent/10 border border-accent/20 rounded-md">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold mb-2">Currently Processing</h2>
              <div className="flex items-center gap-3">
                <span className="font-medium">{getDisplayName(queueStatus.current_processing.path)}</span>
                <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${getProcessTypeColor(queueStatus.current_processing.process_type)}`}>
                  {queueStatus.current_processing.process_type}
                </span>
                {queueStatus.current_processing.time && (
                  <span className="text-sm text-muted-foreground">
                    Time: {queueStatus.current_processing.time}
                  </span>
                )}
              </div>
            </div>
            <button
              onClick={handleCancelCurrent}
              className="px-4 py-2 text-sm bg-destructive text-destructive-foreground rounded hover:bg-destructive/90 focus:outline-none focus:ring-1 focus:ring-ring"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Queue Table */}
      <div className="border border-border rounded-md">
        <div className="p-4 border-b border-border">
          <h2 className="text-lg font-semibold">
            Queue ({queueStatus.queue.length} items)
          </h2>
        </div>
        
        {queueStatus.queue.length === 0 ? (
          <div className="p-8 text-center text-muted-foreground">
            No items in queue
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[10%]">Position</TableHead>
                <TableHead className="w-[40%]">File</TableHead>
                <TableHead className="w-[20%]">Process Type</TableHead>
                <TableHead className="w-[15%]">Time</TableHead>
                <TableHead className="w-[15%]">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {queueStatus.queue.map((item, index) => (
                <TableRow key={`${item.path}-${item.process_type}-${index}`}>
                  <TableCell className="font-mono text-sm">
                    #{index + 1}
                  </TableCell>
                  <TableCell className="font-medium">
                    <div className="max-w-xs truncate" title={item.path}>
                      {getDisplayName(item.path)}
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${getProcessTypeColor(item.process_type)}`}>
                      {item.process_type}
                    </span>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {item.time || '-'}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      {/* Move up button */}
                      <button
                        onClick={() => handleMoveUp(index)}
                        disabled={index === 0}
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
                      
                      {/* Remove button */}
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
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </div>
    </div>
  )
} 