
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from './components/ui/table'
import './App.css'

function App() {
  // Sample data for the table
  type FileRow = {
    name: string
    created_at: string
    line_count?: number
    length?: string
  }
  const files = window.autotranscript_files as FileRow[]

  return (
    <div className="container mx-auto py-10">
      <h1 className="text-2xl font-bold mb-6">File List</h1>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="text-center">Filename</TableHead>
            <TableHead className="text-center">Date</TableHead>
            <TableHead className="text-center">Lines</TableHead>
            <TableHead className="text-center">Length</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {files.map((file, index) => (
            <TableRow key={index}>
              <TableCell className="font-medium">{file.name}</TableCell>
              <TableCell>{file.created_at}</TableCell>
              <TableCell>{file.line_count || 0}</TableCell>
              <TableCell>{file.length || '0:00'}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

export default App
