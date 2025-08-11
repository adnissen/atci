export default function RightPanePlaceholder() {
  return (
    <div className="w-full h-full flex items-center justify-center p-8" style={{ backgroundColor: '#0b0b0b' }}>
      <div className="text-center text-white">
        <div className="mb-6">
          <svg className="w-16 h-16 mx-auto text-white/60" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
          </svg>
        </div>
        <h3 className="text-xl font-medium mb-3 text-white">Clip Editor</h3>
        <p className="text-white/80 max-w-md">
          Click on any timestamp in a transcript to open the clip editor and create video clips from specific moments.
        </p>
      </div>
    </div>
  )
}
