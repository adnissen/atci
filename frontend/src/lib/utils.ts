import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Adds a timestamp parameter to a URL for cache busting
 * @param url The URL to add the timestamp to
 * @returns The URL with the timestamp parameter added
 */
export function addTimestamp(url: string): string {
  const timestamp = Date.now()
  const separator = url.includes('?') ? '&' : '?'
  return `${url}${separator}t=${timestamp}`
}
