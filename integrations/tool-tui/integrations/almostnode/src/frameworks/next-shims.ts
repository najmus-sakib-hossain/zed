/**
 * Next.js shim constants
 * Static HTML/JS strings used by NextDevServer for browser-side Next.js emulation.
 * These are injected into generated HTML pages as inline scripts or served as virtual modules.
 */

import { REACT_REFRESH_CDN, TAILWIND_CDN_URL } from '../config/cdn';

/**
 * Tailwind CSS CDN script for runtime JIT compilation
 */
export const TAILWIND_CDN_SCRIPT = `<script src="${TAILWIND_CDN_URL}"></script>`;

/**
 * CORS Proxy script - provides proxyFetch function in the iframe
 * Reads proxy URL from localStorage (set by parent window)
 */
export const CORS_PROXY_SCRIPT = `
<script>
  // CORS Proxy support for external API calls
  window.__getCorsProxy = function() {
    return localStorage.getItem('__corsProxyUrl') || null;
  };

  window.__setCorsProxy = function(url) {
    if (url) {
      localStorage.setItem('__corsProxyUrl', url);
    } else {
      localStorage.removeItem('__corsProxyUrl');
    }
  };

  window.__proxyFetch = async function(url, options) {
    const proxyUrl = window.__getCorsProxy();
    if (proxyUrl) {
      const proxiedUrl = proxyUrl + encodeURIComponent(url);
      return fetch(proxiedUrl, options);
    }
    return fetch(url, options);
  };
</script>
`;

/**
 * React Refresh preamble - MUST run before React is loaded
 */
export const REACT_REFRESH_PREAMBLE = `
<script type="module">
// Block until React Refresh is loaded and initialized
const RefreshRuntime = await import('${REACT_REFRESH_CDN}').then(m => m.default || m);

RefreshRuntime.injectIntoGlobalHook(window);
window.$RefreshRuntime$ = RefreshRuntime;
window.$RefreshRegCount$ = 0;

window.$RefreshReg$ = (type, id) => {
  window.$RefreshRegCount$++;
  RefreshRuntime.register(type, id);
};

window.$RefreshSig$ = () => (type) => type;

console.log('[HMR] React Refresh initialized');
</script>
`;

/**
 * HMR client script for Next.js
 */
export const HMR_CLIENT_SCRIPT = `
<script type="module">
(function() {
  const hotModules = new Map();
  const pendingUpdates = new Map();

  window.__vite_hot_context__ = function createHotContext(ownerPath) {
    if (hotModules.has(ownerPath)) {
      return hotModules.get(ownerPath);
    }

    const hot = {
      data: {},
      accept(callback) {
        hot._acceptCallback = callback;
      },
      dispose(callback) {
        hot._disposeCallback = callback;
      },
      invalidate() {
        location.reload();
      },
      prune(callback) {
        hot._pruneCallback = callback;
      },
      on(event, cb) {},
      off(event, cb) {},
      send(event, data) {},
      _acceptCallback: null,
      _disposeCallback: null,
      _pruneCallback: null,
    };

    hotModules.set(ownerPath, hot);
    return hot;
  };

  // Listen for HMR updates via postMessage (works with sandboxed iframes)
  window.addEventListener('message', async (event) => {
    // Filter for HMR messages only
    if (!event.data || event.data.channel !== 'next-hmr') return;
    const { type, path, timestamp } = event.data;

    if (type === 'update') {
      console.log('[HMR] Update:', path);

      if (path.endsWith('.css')) {
        const links = document.querySelectorAll('link[rel="stylesheet"]');
        links.forEach(link => {
          const href = link.getAttribute('href');
          if (href && href.includes(path.replace(/^\\//, ''))) {
            link.href = href.split('?')[0] + '?t=' + timestamp;
          }
        });

        const styles = document.querySelectorAll('style[data-next-dev-id]');
        styles.forEach(style => {
          const id = style.getAttribute('data-next-dev-id');
          if (id && id.includes(path.replace(/^\\//, ''))) {
            import(path + '?t=' + timestamp).catch(() => {});
          }
        });
      } else if (path.match(/\\.(jsx?|tsx?)$/)) {
        await handleJSUpdate(path, timestamp);
      }
    } else if (type === 'full-reload') {
      console.log('[HMR] Full reload');
      location.reload();
    }
  });

  async function handleJSUpdate(path, timestamp) {
    const normalizedPath = path.startsWith('/') ? path : '/' + path;
    const hot = hotModules.get(normalizedPath);

    try {
      if (hot && hot._disposeCallback) {
        hot._disposeCallback(hot.data);
      }

      if (window.$RefreshRuntime$) {
        pendingUpdates.set(normalizedPath, timestamp);

        if (pendingUpdates.size === 1) {
          setTimeout(async () => {
            try {
              for (const [modulePath, ts] of pendingUpdates) {
                const moduleUrl = '.' + modulePath + '?t=' + ts;
                await import(moduleUrl);
              }

              window.$RefreshRuntime$.performReactRefresh();
              console.log('[HMR] Updated', pendingUpdates.size, 'module(s)');

              pendingUpdates.clear();
            } catch (error) {
              console.error('[HMR] Failed to apply update:', error);
              pendingUpdates.clear();
              location.reload();
            }
          }, 30);
        }
      } else {
        console.log('[HMR] React Refresh not available, reloading page');
        location.reload();
      }
    } catch (error) {
      console.error('[HMR] Update failed:', error);
      location.reload();
    }
  }

  console.log('[HMR] Next.js client ready');
})();
</script>
`;

/**
 * Next.js Link shim code
 */
export const NEXT_LINK_SHIM = `
import React from 'react';

const getVirtualBasePath = () => {
  const match = window.location.pathname.match(/^\\/__virtual__\\/\\d+(?:\\/|$)/);
  if (!match) return '';
  return match[0].endsWith('/') ? match[0] : match[0] + '/';
};

const getBasePath = () => window.__NEXT_BASE_PATH__ || '';

const applyVirtualBase = (url) => {
  if (typeof url !== 'string') return url;
  if (!url || url.startsWith('#') || url.startsWith('?')) return url;
  if (/^(https?:)?\\/\\//.test(url)) return url;

  // Apply basePath first
  const bp = getBasePath();
  if (bp && url.startsWith('/') && !url.startsWith(bp + '/') && url !== bp) {
    url = bp + url;
  }

  const base = getVirtualBasePath();
  if (!base) return url;
  if (url.startsWith(base)) return url;
  if (url.startsWith('/')) return base + url.slice(1);
  return base + url;
};

export default function Link({ href, children, ...props }) {
  const handleClick = (e) => {
    console.log('[Link] Click handler called, href:', href);

    if (props.onClick) {
      props.onClick(e);
    }

    // Allow cmd/ctrl click to open in new tab
    if (e.metaKey || e.ctrlKey) {
      console.log('[Link] Meta/Ctrl key pressed, allowing default behavior');
      return;
    }

    if (typeof href !== 'string' || !href || href.startsWith('#') || href.startsWith('?')) {
      console.log('[Link] Skipping navigation for href:', href);
      return;
    }

    if (/^(https?:)?\\/\\//.test(href)) {
      console.log('[Link] External URL, allowing default behavior:', href);
      return;
    }

    e.preventDefault();
    const resolvedHref = applyVirtualBase(href);
    console.log('[Link] Navigating to:', resolvedHref);
    window.history.pushState({}, '', resolvedHref);
    window.dispatchEvent(new PopStateEvent('popstate'));
  };

  return React.createElement('a', { href, onClick: handleClick, ...props }, children);
}

export { Link };
`;

/**
 * Next.js Router shim code
 */
export const NEXT_ROUTER_SHIM = `
import React, { useState, useEffect, createContext, useContext } from 'react';

const RouterContext = createContext(null);

const getVirtualBasePath = () => {
  const match = window.location.pathname.match(/^\\/__virtual__\\/\\d+(?:\\/|$)/);
  if (!match) return '';
  return match[0].endsWith('/') ? match[0] : match[0] + '/';
};

const applyVirtualBase = (url) => {
  if (typeof url !== 'string') return url;
  if (!url || url.startsWith('#') || url.startsWith('?')) return url;
  if (/^(https?:)?\\/\\//.test(url)) return url;

  const base = getVirtualBasePath();
  if (!base) return url;
  if (url.startsWith(base)) return url;
  if (url.startsWith('/')) return base + url.slice(1);
  return base + url;
};

const stripVirtualBase = (pathname) => {
  const match = pathname.match(/^\\/__virtual__\\/\\d+(?:\\/|$)/);
  if (!match) return pathname;
  return '/' + pathname.slice(match[0].length);
};

export function useRouter() {
  const [pathname, setPathname] = useState(
    typeof window !== 'undefined' ? stripVirtualBase(window.location.pathname) : '/'
  );
  const [query, setQuery] = useState({});

  useEffect(() => {
    const updateRoute = () => {
      setPathname(stripVirtualBase(window.location.pathname));
      setQuery(Object.fromEntries(new URLSearchParams(window.location.search)));
    };

    window.addEventListener('popstate', updateRoute);
    updateRoute();

    return () => window.removeEventListener('popstate', updateRoute);
  }, []);

  return {
    pathname,
    query,
    asPath: pathname + window.location.search,
    push: (url, as, options) => {
      if (typeof url === 'string' && /^(https?:)?\\/\\//.test(url)) {
        window.location.href = url;
        return Promise.resolve(true);
      }
      const resolvedUrl = applyVirtualBase(url);
      window.history.pushState({}, '', resolvedUrl);
      window.dispatchEvent(new PopStateEvent('popstate'));
      return Promise.resolve(true);
    },
    replace: (url, as, options) => {
      if (typeof url === 'string' && /^(https?:)?\\/\\//.test(url)) {
        window.location.href = url;
        return Promise.resolve(true);
      }
      const resolvedUrl = applyVirtualBase(url);
      window.history.replaceState({}, '', resolvedUrl);
      window.dispatchEvent(new PopStateEvent('popstate'));
      return Promise.resolve(true);
    },
    prefetch: () => Promise.resolve(),
    back: () => window.history.back(),
    forward: () => window.history.forward(),
    reload: () => window.location.reload(),
    events: {
      on: () => {},
      off: () => {},
      emit: () => {},
    },
    isFallback: false,
    isReady: true,
    isPreview: false,
  };
}

export const Router = {
  events: {
    on: () => {},
    off: () => {},
    emit: () => {},
  },
  push: (url) => {
    if (typeof url === 'string' && /^(https?:)?\\/\\//.test(url)) {
      window.location.href = url;
      return Promise.resolve(true);
    }
    const resolvedUrl = applyVirtualBase(url);
    window.history.pushState({}, '', resolvedUrl);
    window.dispatchEvent(new PopStateEvent('popstate'));
    return Promise.resolve(true);
  },
  replace: (url) => {
    if (typeof url === 'string' && /^(https?:)?\\/\\//.test(url)) {
      window.location.href = url;
      return Promise.resolve(true);
    }
    const resolvedUrl = applyVirtualBase(url);
    window.history.replaceState({}, '', resolvedUrl);
    window.dispatchEvent(new PopStateEvent('popstate'));
    return Promise.resolve(true);
  },
};

export default { useRouter, Router };
`;

/**
 * Next.js Navigation shim code (App Router)
 *
 * This shim provides App Router-specific navigation hooks from 'next/navigation'.
 * These are DIFFERENT from the Pages Router hooks in 'next/router':
 *
 * Pages Router (next/router):
 *   - useRouter() returns { pathname, query, push, replace, events, ... }
 *   - Has router.events for route change subscriptions
 *   - query object contains URL params
 *
 * App Router (next/navigation):
 *   - useRouter() returns { push, replace, back, forward, refresh, prefetch }
 *   - usePathname() for current path
 *   - useSearchParams() for URL search params
 *   - useParams() for dynamic route segments
 *   - No events - use useEffect with pathname/searchParams instead
 *
 * @see https://nextjs.org/docs/app/api-reference/functions/use-router
 */
export const NEXT_NAVIGATION_SHIM = `
import React, { useState, useEffect, useCallback, useMemo } from 'react';

const getVirtualBasePath = () => {
  const match = window.location.pathname.match(/^\\/__virtual__\\/\\d+(?:\\/|$)/);
  if (!match) return '';
  return match[0].endsWith('/') ? match[0] : match[0] + '/';
};

const applyVirtualBase = (url) => {
  if (typeof url !== 'string') return url;
  if (!url || url.startsWith('#') || url.startsWith('?')) return url;
  if (/^(https?:)?\\/\\//.test(url)) return url;

  const base = getVirtualBasePath();
  if (!base) return url;
  if (url.startsWith(base)) return url;
  if (url.startsWith('/')) return base + url.slice(1);
  return base + url;
};

const stripVirtualBase = (pathname) => {
  const match = pathname.match(/^\\/__virtual__\\/\\d+(?:\\/|$)/);
  if (!match) return pathname;
  return '/' + pathname.slice(match[0].length);
};

/**
 * App Router's useRouter hook
 * Returns navigation methods only (no pathname, no query)
 * Use usePathname() and useSearchParams() for URL info
 */
export function useRouter() {
  const push = useCallback((url, options) => {
    if (typeof url === 'string' && /^(https?:)?\\/\\//.test(url)) {
      window.location.href = url;
      return;
    }
    const resolvedUrl = applyVirtualBase(url);
    window.history.pushState({}, '', resolvedUrl);
    window.dispatchEvent(new PopStateEvent('popstate'));
  }, []);

  const replace = useCallback((url, options) => {
    if (typeof url === 'string' && /^(https?:)?\\/\\//.test(url)) {
      window.location.href = url;
      return;
    }
    const resolvedUrl = applyVirtualBase(url);
    window.history.replaceState({}, '', resolvedUrl);
    window.dispatchEvent(new PopStateEvent('popstate'));
  }, []);

  const back = useCallback(() => window.history.back(), []);
  const forward = useCallback(() => window.history.forward(), []);
  const refresh = useCallback(() => window.location.reload(), []);
  const prefetch = useCallback(() => Promise.resolve(), []);

  return useMemo(() => ({
    push,
    replace,
    back,
    forward,
    refresh,
    prefetch,
  }), [push, replace, back, forward, refresh, prefetch]);
}

/**
 * usePathname - Returns the current URL pathname
 * Reactively updates when navigation occurs
 * @example const pathname = usePathname(); // '/dashboard/settings'
 */
export function usePathname() {
  const [pathname, setPathname] = useState(
    typeof window !== 'undefined' ? stripVirtualBase(window.location.pathname) : '/'
  );

  useEffect(() => {
    const handler = () => setPathname(stripVirtualBase(window.location.pathname));
    window.addEventListener('popstate', handler);
    return () => window.removeEventListener('popstate', handler);
  }, []);

  return pathname;
}

/**
 * useSearchParams - Returns the current URL search parameters
 * @example const searchParams = useSearchParams();
 *          const query = searchParams.get('q'); // '?q=hello' -> 'hello'
 */
export function useSearchParams() {
  const [searchParams, setSearchParams] = useState(() => {
    if (typeof window === 'undefined') return new URLSearchParams();
    return new URLSearchParams(window.location.search);
  });

  useEffect(() => {
    const handler = () => {
      setSearchParams(new URLSearchParams(window.location.search));
    };
    window.addEventListener('popstate', handler);
    return () => window.removeEventListener('popstate', handler);
  }, []);

  return searchParams;
}

/**
 * useParams - Returns dynamic route parameters
 * For route /users/[id]/page.jsx with URL /users/123:
 * @example const { id } = useParams(); // { id: '123' }
 *
 * Fetches params from the server's route-info endpoint for dynamic routes.
 */
export function useParams() {
  const [params, setParams] = useState(() => {
    // Check if initial params were embedded by the server
    if (typeof window !== 'undefined' && window.__NEXT_ROUTE_PARAMS__) {
      return window.__NEXT_ROUTE_PARAMS__;
    }
    return {};
  });

  useEffect(() => {
    let cancelled = false;

    const fetchParams = async () => {
      const pathname = stripVirtualBase(window.location.pathname);
      const base = getVirtualBasePath();
      const baseUrl = base ? base.replace(/\\/$/, '') : '';

      try {
        const response = await fetch(baseUrl + '/_next/route-info?pathname=' + encodeURIComponent(pathname));
        const info = await response.json();
        if (!cancelled && info.params) {
          setParams(info.params);
        }
      } catch (e) {
        // Silently fail - static routes won't have params
      }
    };

    fetchParams();

    const handler = () => fetchParams();
    window.addEventListener('popstate', handler);
    return () => {
      cancelled = true;
      window.removeEventListener('popstate', handler);
    };
  }, []);

  return params;
}

/**
 * useSelectedLayoutSegment - Returns the active child segment one level below
 * Useful for styling active nav items in layouts
 * @example For /dashboard/settings, returns 'settings' in dashboard layout
 */
export function useSelectedLayoutSegment() {
  const pathname = usePathname();
  const segments = pathname.split('/').filter(Boolean);
  return segments[0] || null;
}

/**
 * useSelectedLayoutSegments - Returns all active child segments
 * @example For /dashboard/settings/profile, returns ['dashboard', 'settings', 'profile']
 */
export function useSelectedLayoutSegments() {
  const pathname = usePathname();
  return pathname.split('/').filter(Boolean);
}

/**
 * redirect - Programmatic redirect (typically used in Server Components)
 * In this browser implementation, performs immediate navigation
 */
export function redirect(url) {
  if (typeof url === 'string' && /^(https?:)?\\/\\//.test(url)) {
    window.location.href = url;
    return;
  }
  window.location.href = applyVirtualBase(url);
}

/**
 * notFound - Trigger the not-found UI
 * In this browser implementation, throws an error
 */
export function notFound() {
  throw new Error('NEXT_NOT_FOUND');
}

// Re-export Link for convenience (can import from next/navigation or next/link)
export { default as Link } from 'next/link';
`;

/**
 * Next.js Head shim code
 */
export const NEXT_HEAD_SHIM = `
import React, { useEffect } from 'react';

export default function Head({ children }) {
  useEffect(() => {
    // Process children and update document.head
    React.Children.forEach(children, (child) => {
      if (!React.isValidElement(child)) return;

      const { type, props } = child;

      if (type === 'title' && props.children) {
        document.title = Array.isArray(props.children)
          ? props.children.join('')
          : props.children;
      } else if (type === 'meta') {
        const existingMeta = props.name
          ? document.querySelector(\`meta[name="\${props.name}"]\`)
          : props.property
            ? document.querySelector(\`meta[property="\${props.property}"]\`)
            : null;

        if (existingMeta) {
          Object.keys(props).forEach(key => {
            existingMeta.setAttribute(key, props[key]);
          });
        } else {
          const meta = document.createElement('meta');
          Object.keys(props).forEach(key => {
            meta.setAttribute(key, props[key]);
          });
          document.head.appendChild(meta);
        }
      } else if (type === 'link') {
        const link = document.createElement('link');
        Object.keys(props).forEach(key => {
          link.setAttribute(key, props[key]);
        });
        document.head.appendChild(link);
      }
    });
  }, [children]);

  return null;
}
`;

/**
 * Next.js Image shim code
 * Provides a simple img-based implementation of next/image
 */
export const NEXT_IMAGE_SHIM = `
import React from 'react';

function Image({
  src,
  alt = '',
  width,
  height,
  fill,
  loader,
  quality = 75,
  priority,
  loading,
  placeholder,
  blurDataURL,
  unoptimized,
  onLoad,
  onError,
  style,
  className,
  sizes,
  ...rest
}) {
  // Handle src - could be string or StaticImageData object
  const imageSrc = typeof src === 'object' ? src.src : src;

  // Build style object
  const imgStyle = { ...style };
  if (fill) {
    imgStyle.position = 'absolute';
    imgStyle.width = '100%';
    imgStyle.height = '100%';
    imgStyle.objectFit = imgStyle.objectFit || 'cover';
    imgStyle.inset = '0';
  }

  return React.createElement('img', {
    src: imageSrc,
    alt,
    width: fill ? undefined : width,
    height: fill ? undefined : height,
    loading: priority ? 'eager' : (loading || 'lazy'),
    decoding: 'async',
    style: imgStyle,
    className,
    onLoad,
    onError,
    ...rest
  });
}

export default Image;
export { Image };
`;

/**
 * next/dynamic shim - Dynamic imports with loading states
 */
export const NEXT_DYNAMIC_SHIM = `
import React from 'react';

function dynamic(importFn, options = {}) {
  const {
    loading: LoadingComponent,
    ssr = true,
  } = options;

  // Create a lazy component
  const LazyComponent = React.lazy(importFn);

  // Wrapper component that handles loading state
  function DynamicComponent(props) {
    const fallback = LoadingComponent
      ? React.createElement(LoadingComponent, { isLoading: true })
      : null;

    return React.createElement(
      React.Suspense,
      { fallback },
      React.createElement(LazyComponent, props)
    );
  }

  return DynamicComponent;
}

export default dynamic;
export { dynamic };
`;

/**
 * next/script shim - Loads external scripts
 */
export const NEXT_SCRIPT_SHIM = `
import React from 'react';

function Script({
  src,
  strategy = 'afterInteractive',
  onLoad,
  onReady,
  onError,
  children,
  dangerouslySetInnerHTML,
  ...rest
}) {
  React.useEffect(function() {
    if (!src && !children && !dangerouslySetInnerHTML) return;

    var script = document.createElement('script');

    if (src) {
      script.src = src;
      script.async = strategy !== 'beforeInteractive';
    }

    Object.keys(rest).forEach(function(key) {
      script.setAttribute(key, rest[key]);
    });

    if (children) {
      script.textContent = children;
    } else if (dangerouslySetInnerHTML && dangerouslySetInnerHTML.__html) {
      script.textContent = dangerouslySetInnerHTML.__html;
    }

    script.onload = function() {
      if (onLoad) onLoad();
      if (onReady) onReady();
    };
    script.onerror = onError;

    document.head.appendChild(script);

    return function() {
      if (script.parentNode) {
        script.parentNode.removeChild(script);
      }
    };
  }, [src]);

  return null;
}

export default Script;
export { Script };
`;

/**
 * next/font/google shim - Loads Google Fonts via CDN
 * Uses a Proxy to dynamically handle ANY Google Font without hardcoding
 */
export const NEXT_FONT_GOOGLE_SHIM = `
// Track loaded fonts to avoid duplicate style injections
const loadedFonts = new Set();

/**
 * Convert font function name to Google Fonts family name
 * Examples:
 *   DM_Sans -> DM Sans
 *   Open_Sans -> Open Sans
 *   Fraunces -> Fraunces
 */
function toFontFamily(fontName) {
  return fontName.replace(/_/g, ' ');
}

/**
 * Inject font CSS into document
 * - Adds preconnect links for faster font loading
 * - Loads the font from Google Fonts CDN
 * - Creates a CSS class that sets the CSS variable
 */
function injectFontCSS(fontFamily, variableName, weight, style) {
  const fontKey = fontFamily + '-' + (variableName || 'default');
  if (loadedFonts.has(fontKey)) {
    return;
  }
  loadedFonts.add(fontKey);

  if (typeof document === 'undefined') {
    return;
  }

  // Add preconnect links for faster loading (only once)
  if (!document.querySelector('link[href="https://fonts.googleapis.com"]')) {
    const preconnect1 = document.createElement('link');
    preconnect1.rel = 'preconnect';
    preconnect1.href = 'https://fonts.googleapis.com';
    document.head.appendChild(preconnect1);

    const preconnect2 = document.createElement('link');
    preconnect2.rel = 'preconnect';
    preconnect2.href = 'https://fonts.gstatic.com';
    preconnect2.crossOrigin = 'anonymous';
    document.head.appendChild(preconnect2);
  }

  // Build Google Fonts URL
  const escapedFamily = fontFamily.replace(/ /g, '+');

  // Build axis list based on options
  let axisList = '';
  const axes = [];

  // Handle italic style
  if (style === 'italic') {
    axes.push('ital');
  }

  // Handle weight - use specific weight or variable range
  if (weight && weight !== '400' && !Array.isArray(weight)) {
    // Specific weight requested
    axes.push('wght');
    if (style === 'italic') {
      axisList = ':ital,wght@1,' + weight;
    } else {
      axisList = ':wght@' + weight;
    }
  } else if (Array.isArray(weight)) {
    // Multiple weights
    axes.push('wght');
    axisList = ':wght@' + weight.join(';');
  } else {
    // Default: request common weights for flexibility
    axisList = ':wght@400;500;600;700';
  }

  const fontUrl = 'https://fonts.googleapis.com/css2?family=' +
    escapedFamily + axisList + '&display=swap';

  // Add link element for Google Fonts (if not already present)
  if (!document.querySelector('link[href*="family=' + escapedFamily + '"]')) {
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = fontUrl;
    document.head.appendChild(link);
  }

  // Create style element for CSS variable at :root level (globally available)
  // This makes the variable work without needing to apply the class to body
  if (variableName) {
    const styleEl = document.createElement('style');
    styleEl.setAttribute('data-font-var', variableName);
    styleEl.textContent = ':root { ' + variableName + ': "' + fontFamily + '", ' + (fontFamily.includes('Serif') ? 'serif' : 'sans-serif') + '; }';
    document.head.appendChild(styleEl);
  }
}

/**
 * Create a font loader function for a specific font
 */
function createFontLoader(fontName) {
  const fontFamily = toFontFamily(fontName);

  return function(options = {}) {
    const {
      weight,
      style = 'normal',
      subsets = ['latin'],
      variable,
      display = 'swap',
      preload = true,
      fallback = ['sans-serif'],
      adjustFontFallback = true
    } = options;

    // Inject the font CSS
    injectFontCSS(fontFamily, variable, weight, style);

    // Generate class name from variable (--font-inter -> __font-inter)
    const className = variable
      ? variable.replace('--', '__')
      : '__font-' + fontName.toLowerCase().replace(/_/g, '-');

    return {
      className,
      variable: className,
      style: {
        fontFamily: '"' + fontFamily + '", ' + fallback.join(', ')
      }
    };
  };
}

/**
 * Use a Proxy to dynamically create font loaders for ANY font name
 * This allows: import { AnyGoogleFont } from "next/font/google"
 */
const fontProxy = new Proxy({}, {
  get(target, prop) {
    // Handle special properties
    if (prop === '__esModule') return true;
    if (prop === 'default') return fontProxy;
    if (typeof prop !== 'string') return undefined;

    // Create a font loader for this font name
    return createFontLoader(prop);
  }
});

// Export the proxy as both default and named exports
export default fontProxy;

// Re-export through proxy for named imports
export const {
  Fraunces, Inter, DM_Sans, DM_Serif_Text, Roboto, Open_Sans, Lato,
  Montserrat, Poppins, Playfair_Display, Merriweather, Raleway, Nunito,
  Ubuntu, Oswald, Quicksand, Work_Sans, Fira_Sans, Barlow, Mulish, Rubik,
  Noto_Sans, Manrope, Space_Grotesk, Geist, Geist_Mono
} = fontProxy;
`;

/**
 * next/font/local shim - Loads local font files
 * Accepts font source path and creates @font-face declaration + CSS variable
 */
export const NEXT_FONT_LOCAL_SHIM = `
const loadedLocalFonts = new Set();

function localFont(options = {}) {
  const {
    src,
    weight,
    style = 'normal',
    variable,
    display = 'swap',
    fallback = ['sans-serif'],
    declarations = [],
    adjustFontFallback = true
  } = options;

  // Determine font family name from variable or src
  const familyName = variable
    ? variable.replace('--', '').replace(/-/g, ' ')
    : 'local-font-' + Math.random().toString(36).slice(2, 8);

  const fontKey = familyName + '-' + (variable || 'default');
  if (typeof document !== 'undefined' && !loadedLocalFonts.has(fontKey)) {
    loadedLocalFonts.add(fontKey);

    // Build @font-face declarations
    let fontFaces = '';

    if (typeof src === 'string') {
      // Single source
      fontFaces = '@font-face {\\n' +
        '  font-family: "' + familyName + '";\\n' +
        '  src: url("' + src + '");\\n' +
        '  font-weight: ' + (weight || '400') + ';\\n' +
        '  font-style: ' + style + ';\\n' +
        '  font-display: ' + display + ';\\n' +
        '}';
    } else if (Array.isArray(src)) {
      // Multiple sources (different weights/styles)
      fontFaces = src.map(function(s) {
        const path = typeof s === 'string' ? s : s.path;
        const w = (typeof s === 'object' && s.weight) || weight || '400';
        const st = (typeof s === 'object' && s.style) || style;
        return '@font-face {\\n' +
          '  font-family: "' + familyName + '";\\n' +
          '  src: url("' + path + '");\\n' +
          '  font-weight: ' + w + ';\\n' +
          '  font-style: ' + st + ';\\n' +
          '  font-display: ' + display + ';\\n' +
          '}';
      }).join('\\n');
    }

    // Inject font-face CSS
    if (fontFaces) {
      var styleEl = document.createElement('style');
      styleEl.setAttribute('data-local-font', fontKey);
      styleEl.textContent = fontFaces;
      document.head.appendChild(styleEl);
    }

    // Inject CSS variable at :root level
    if (variable) {
      var varStyle = document.createElement('style');
      varStyle.setAttribute('data-font-var', variable);
      varStyle.textContent = ':root { ' + variable + ': "' + familyName + '", ' + fallback.join(', ') + '; }';
      document.head.appendChild(varStyle);
    }
  }

  const className = variable
    ? variable.replace('--', '__')
    : '__font-' + familyName.toLowerCase().replace(/\\s+/g, '-');

  return {
    className,
    variable: className,
    style: {
      fontFamily: '"' + familyName + '", ' + fallback.join(', ')
    }
  };
}

export default localFont;
export { localFont };
`;
