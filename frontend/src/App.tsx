import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from './components/ui/table'
import './App.css'
import TranscriptView from './components/TranscriptView'
import { useState } from 'react'

function App() {
  // Sample data for the table
  type FileRow = {
    name: string
    created_at: string
    line_count?: number
    length?: string
  }
  const files = window.autotranscript_files as FileRow[]
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(new Set())

  return (
    <div className="container mx-auto py-10">
      <h1 className="text-2xl font-bold mb-6">File List</h1>
      <Table className="table-fixed w-full">
        <TableHeader>
          <TableRow>
            <TableHead className="text-center w-1/4">Filename</TableHead>
            <TableHead className="text-center w-1/4">Date</TableHead>
            <TableHead className="text-center w-1/4">Lines</TableHead>
            <TableHead className="text-center w-1/4">Length</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {files.map((file, index) => (
            <>
            <TableRow key={index} onClick={() => {
              setExpandedFiles(prev => {
                const newSet = new Set(prev)
                newSet.has(file.name) ? newSet.delete(file.name) : newSet.add(file.name)
                return newSet
              })
            }}>
              <TableCell className="font-medium w-1/4">
                <a 
                  href={`/transcripts/${file.name}`}
                  className="text-blue-600 hover:text-blue-800 underline"
                  onClick={(e) => e.stopPropagation()}
                >
                  {file.name}
                </a>
              </TableCell>
              <TableCell className="w-1/4">{file.created_at}</TableCell>
              <TableCell className="w-1/4">{file.line_count || 0}</TableCell>
              <TableCell className="w-1/4">{file.length || '0:00'}</TableCell>
            </TableRow>
            <TableRow>
            <TranscriptView
              visible={expandedFiles.has(file.name)}
              name={file.name}
              className="w-full"
            />
            </TableRow>
            </>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

export default App
