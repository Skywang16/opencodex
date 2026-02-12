/**
 * Image processing utility functions
 */

export interface ProcessedImage {
  dataUrl: string
  mimeType: string
  fileName: string
  fileSize: number
}

/**
 * Convert file to base64 data URL
 */
export const fileToDataUrl = (file: File): Promise<string> => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result as string)
    reader.onerror = reject
    reader.readAsDataURL(file)
  })
}

/**
 * Compress image (if exceeds maximum size)
 */
export const compressImage = async (
  dataUrl: string,
  maxWidth: number = 2048,
  maxHeight: number = 2048,
  quality: number = 0.9
): Promise<string> => {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.onload = () => {
      let { width, height } = img

      // Calculate scaling ratio
      if (width > maxWidth || height > maxHeight) {
        const ratio = Math.min(maxWidth / width, maxHeight / height)
        width = Math.floor(width * ratio)
        height = Math.floor(height * ratio)
      }

      // Create canvas for compression
      const canvas = document.createElement('canvas')
      canvas.width = width
      canvas.height = height

      const ctx = canvas.getContext('2d')
      if (!ctx) {
        reject(new Error('Failed to get canvas context'))
        return
      }

      ctx.drawImage(img, 0, 0, width, height)

      // Convert to base64
      const compressedDataUrl = canvas.toDataURL('image/jpeg', quality)
      resolve(compressedDataUrl)
    }
    img.onerror = reject
    img.src = dataUrl
  })
}

/**
 * Process image file: read, compress, and return processed data
 */
export const processImageFile = async (file: File): Promise<ProcessedImage> => {
  // Check file type
  if (!file.type.startsWith('image/')) {
    throw new Error('File is not an image')
  }

  // Read file
  const dataUrl = await fileToDataUrl(file)

  // Compress image (if needed)
  const compressedDataUrl = await compressImage(dataUrl)

  return {
    dataUrl: compressedDataUrl,
    mimeType: 'image/jpeg', // Unified as JPEG after compression
    fileName: file.name,
    fileSize: file.size,
  }
}

/**
 * Get image from clipboard
 */
export const getImageFromClipboard = async (event: ClipboardEvent): Promise<File | null> => {
  const items = event.clipboardData?.items
  if (!items) return null

  for (const item of Array.from(items)) {
    if (item.type.startsWith('image/')) {
      const file = item.getAsFile()
      return file
    }
  }

  return null
}

/**
 * Validate image file
 */
export const validateImageFile = (file: File): { valid: boolean; error?: string } => {
  // Check file type
  const validTypes = ['image/jpeg', 'image/jpg', 'image/png', 'image/gif', 'image/webp']
  if (!validTypes.includes(file.type)) {
    return {
      valid: false,
      error: 'Unsupported image format. Please use JPEG, PNG, GIF, or WebP.',
    }
  }

  // Check file size (max 20MB)
  const maxSize = 20 * 1024 * 1024
  if (file.size > maxSize) {
    return {
      valid: false,
      error: 'Image file is too large. Maximum size is 20MB.',
    }
  }

  return { valid: true }
}
