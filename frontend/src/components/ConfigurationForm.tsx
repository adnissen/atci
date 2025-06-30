import React, { useState, useEffect } from 'react'

interface ConfigData {
  watch_directory?: string
  whispercli_path?: string  
  model_path?: string
  valid?: boolean
  config_file_path?: string | null
}

interface ConfigurationFormProps {
  onConfigComplete: () => void
}

const ConfigurationForm: React.FC<ConfigurationFormProps> = ({ onConfigComplete }) => {
  const [config, setConfig] = useState<ConfigData>({})
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Form state
  const [watchDirectory, setWatchDirectory] = useState('')
  const [whispercliPath, setWhispercliPath] = useState('')
  const [modelPath, setModelPath] = useState('')

  useEffect(() => {
    fetchConfig()
  }, [])

  const fetchConfig = async () => {
    try {
      const response = await fetch('/api/config')
      if (response.ok) {
        const configData = await response.json()
        setConfig(configData)
        setWatchDirectory(configData.watch_directory || '')
        setWhispercliPath(configData.whispercli_path || '')
        setModelPath(configData.model_path || '')
      } else {
        setError('Failed to fetch configuration')
      }
    } catch (err) {
      setError('Error fetching configuration')
      console.error('Config fetch error:', err)
    } finally {
      setLoading(false)
    }
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaving(true)
    setError(null)

    try {
      const response = await fetch('/api/config', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          watch_directory: watchDirectory.trim(),
          whispercli_path: whispercliPath.trim(),
          model_path: modelPath.trim()
        })
      })

      if (response.ok) {
        const updatedConfig = await response.json()
        setConfig(updatedConfig)
        
        if (updatedConfig.valid) {
          onConfigComplete()
        } else {
          setError('Configuration saved but some values are still invalid')
        }
      } else {
        const errorData = await response.json()
        setError(errorData.error || 'Failed to save configuration')
      }
    } catch (err) {
      setError('Error saving configuration')
      console.error('Config save error:', err)
    } finally {
      setSaving(false)
    }
  }

  const openDirectoryPicker = async (field: 'watch_directory' | 'whispercli_path' | 'model_path') => {
    // For now, we'll use a simple text input. In a real app, you might want to integrate
    // with the File System Access API for directory/file picking
    const value = field === 'watch_directory' 
      ? prompt('Enter the full path to the directory containing your video files:')
      : field === 'whispercli_path'
      ? prompt('Enter the full path to the whisper-cli executable:')
      : prompt('Enter the full path to the Whisper model file:')

    if (value) {
      if (field === 'watch_directory') {
        setWatchDirectory(value)
      } else if (field === 'whispercli_path') {
        setWhispercliPath(value)
      } else {
        setModelPath(value)
      }
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading configuration...</p>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8">
      <div className="sm:mx-auto sm:w-full sm:max-w-md">
        <h2 className="mt-6 text-center text-3xl font-extrabold text-gray-900">
          Configuration Required
        </h2>
        <p className="mt-2 text-center text-sm text-gray-600">
          Please configure the required paths to get started
        </p>
        {config.config_file_path && (
          <p className="mt-2 text-center text-xs text-gray-500">
            Config file: {config.config_file_path}
          </p>
        )}
      </div>

      <div className="mt-8 sm:mx-auto sm:w-full sm:max-w-md">
        <div className="bg-white py-8 px-4 shadow sm:rounded-lg sm:px-10">
          {error && (
            <div className="mb-4 bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
              {error}
            </div>
          )}

          <form onSubmit={handleSave} className="space-y-6">
            <div>
              <label htmlFor="watch_directory" className="block text-sm font-medium text-gray-700">
                Watch Directory
              </label>
              <div className="mt-1 flex">
                <input
                  type="text"
                  id="watch_directory"
                  value={watchDirectory}
                  onChange={(e) => setWatchDirectory(e.target.value)}
                  placeholder="/path/to/your/videos"
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-l-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                  required
                />
                <button
                  type="button"
                  onClick={() => openDirectoryPicker('watch_directory')}
                  className="px-3 py-2 border border-l-0 border-gray-300 bg-gray-50 rounded-r-md hover:bg-gray-100 focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                >
                  📁
                </button>
              </div>
              <p className="mt-1 text-sm text-gray-500">
                Directory where video files are stored and transcripts will be saved
              </p>
            </div>

            <div>
              <label htmlFor="whispercli_path" className="block text-sm font-medium text-gray-700">
                Whisper CLI Path
              </label>
              <div className="mt-1 flex">
                <input
                  type="text"
                  id="whispercli_path"
                  value={whispercliPath}
                  onChange={(e) => setWhispercliPath(e.target.value)}
                  placeholder="/usr/local/bin/whisper-cli"
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-l-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                  required
                />
                <button
                  type="button"
                  onClick={() => openDirectoryPicker('whispercli_path')}
                  className="px-3 py-2 border border-l-0 border-gray-300 bg-gray-50 rounded-r-md hover:bg-gray-100 focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                >
                  📁
                </button>
              </div>
              <p className="mt-1 text-sm text-gray-500">
                Path to the whisper-cli executable
              </p>
            </div>

            <div>
              <label htmlFor="model_path" className="block text-sm font-medium text-gray-700">
                Model Path
              </label>
              <div className="mt-1 flex">
                <input
                  type="text"
                  id="model_path"
                  value={modelPath}
                  onChange={(e) => setModelPath(e.target.value)}
                  placeholder="/path/to/whisper.cpp/model.bin"
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-l-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                  required
                />
                <button
                  type="button"
                  onClick={() => openDirectoryPicker('model_path')}
                  className="px-3 py-2 border border-l-0 border-gray-300 bg-gray-50 rounded-r-md hover:bg-gray-100 focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                >
                  📁
                </button>
              </div>
              <p className="mt-1 text-sm text-gray-500">
                Path to the Whisper model file (.bin)
              </p>
            </div>

            <div>
              <button
                type="submit"
                disabled={saving}
                className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {saving ? (
                  <>
                    <svg className="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                    </svg>
                    Saving...
                  </>
                ) : (
                  'Save Configuration'
                )}
              </button>
            </div>
          </form>

          <div className="mt-6">
            <div className="relative">
              <div className="absolute inset-0 flex items-center">
                <div className="w-full border-t border-gray-300" />
              </div>
              <div className="relative flex justify-center text-sm">
                <span className="px-2 bg-white text-gray-500">Configuration Info</span>
              </div>
            </div>

            <div className="mt-4 text-sm text-gray-600">
              <p className="mb-2">
                <strong>Configuration file:</strong> {config.config_file_path || 'None found - will create .atconfig'}
              </p>
              <p className="mb-2">
                The configuration will be saved to a <code>.atconfig</code> file in the current directory.
              </p>
              <p>
                If you run the application from different directories, you can create separate config files
                or place a global one in your home directory.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default ConfigurationForm