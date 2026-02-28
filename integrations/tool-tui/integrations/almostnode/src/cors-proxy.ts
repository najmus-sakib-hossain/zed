/**
 * CORS Proxy Utility
 *
 * Provides optional CORS proxy support for fetching external APIs
 * that don't allow browser origins.
 *
 * No default proxy is configured for security reasons.
 * Users must explicitly set a proxy URL if needed.
 *
 * Example usage:
 *   setCorsProxy('https://corsproxy.io/?');
 *   const response = await proxyFetch('https://api.example.com/data');
 */

// No default proxy - must be explicitly set
let proxyUrl: string | null = null;

/**
 * Set the CORS proxy URL
 * @param url - Proxy URL (e.g., 'https://corsproxy.io/?')
 *              The target URL will be appended as an encoded parameter.
 *              Set to null to disable proxy and use direct fetch.
 */
export function setCorsProxy(url: string | null): void {
  proxyUrl = url;
  if (url) {
    console.log(`[cors-proxy] Proxy configured: ${url}`);
  } else {
    console.log('[cors-proxy] Proxy disabled, using direct fetch');
  }
}

/**
 * Get the current CORS proxy URL
 * @returns The configured proxy URL, or null if not set
 */
export function getCorsProxy(): string | null {
  return proxyUrl;
}

/**
 * Check if a proxy is configured
 */
export function hasProxy(): boolean {
  return proxyUrl !== null;
}

/**
 * Fetch with optional CORS proxy
 *
 * If a proxy is configured, the request goes through the proxy.
 * Otherwise, a direct fetch is performed (may hit CORS issues).
 *
 * @param url - The target URL to fetch
 * @param options - Standard fetch options
 * @returns Promise<Response>
 */
export async function proxyFetch(
  url: string,
  options?: RequestInit
): Promise<Response> {
  if (proxyUrl) {
    // Route through proxy
    const proxiedUrl = proxyUrl + encodeURIComponent(url);
    return fetch(proxiedUrl, options);
  }

  // No proxy configured - direct fetch
  return fetch(url, options);
}

/**
 * Build a proxied URL without fetching
 * Useful for displaying the URL or using with other fetch mechanisms
 */
export function buildProxyUrl(url: string): string {
  if (proxyUrl) {
    return proxyUrl + encodeURIComponent(url);
  }
  return url;
}
