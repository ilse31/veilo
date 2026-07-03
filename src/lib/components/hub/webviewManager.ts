import { openServiceWebview, hideServiceWebview, resizeServiceWebview } from '$lib/tauri'
import type { Service } from '$lib/tauri'

export type { Service }

const SERVICE_URLS: Record<Service, string> = {
  gpt:       'https://chatgpt.com',
  claude:    'https://claude.ai',
  translate: 'https://translate.google.com',
}

// Custom URLs (overridden from settings)
const customUrls: Partial<Record<Service, string>> = {}

export function setServiceUrl(service: Service, url: string) {
  customUrls[service] = url
}

function getUrl(service: Service): string {
  return customUrls[service] || SERVICE_URLS[service]
}

// Track which services have open (created) webviews
const opened = new Set<Service>()

/**
 * Show the real webview for a given service.
 * Calculates the content area bounds from the DOM element.
 */
export async function showService(service: Service, contentEl: HTMLElement): Promise<void> {
  const rect = contentEl.getBoundingClientRect()
  const url = getUrl(service)

  console.log(`[webviewManager] showService="${service}" url=${url}`)
  console.log(`[webviewManager]   rect: x=${rect.left} y=${rect.top} w=${rect.width} h=${rect.height}`)

  if (rect.width === 0 || rect.height === 0) {
    console.error(`[webviewManager] ABORT: zero-size content area for "${service}"`)
    throw new Error(`content area is zero-size (${rect.width}×${rect.height})`)
  }

  try {
    await openServiceWebview(service, url, rect.left, rect.top, rect.width, rect.height)
    opened.add(service)
    console.log(`[webviewManager] ✓ "${service}" window created/shown`)
  } catch (err) {
    console.error(`[webviewManager] ✗ openServiceWebview failed for "${service}":`, err)
    throw err
  }
}

/**
 * Hide the webview for a given service (keeps it alive).
 */
export async function hideService(service: Service): Promise<void> {
  if (opened.has(service)) {
    await hideServiceWebview(service)
  }
}

/**
 * Resize an already-open service webview to match the content area.
 */
export async function resizeService(service: Service, contentEl: HTMLElement): Promise<void> {
  if (!opened.has(service)) return
  const rect = contentEl.getBoundingClientRect()
  if (rect.width === 0 || rect.height === 0) return
  await resizeServiceWebview(service, rect.left, rect.top, rect.width, rect.height)
}

/** Hide all open service webviews (e.g. when switching to a non-webview page). */
export async function hideAllServices(): Promise<void> {
  await Promise.all([...opened].map((s) => hideServiceWebview(s).catch(() => {})))
}

export const SERVICES: Service[] = ['gpt', 'claude', 'translate']
