/**
 * Icon loading and caching module
 */
import { CONFIG, UNIQUE_ICON_EXTENSIONS } from './config.js';
import { getIconCacheKey, debounce } from './utils.js';

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);

/**
 * Icon manager class
 */
class IconManager {
    constructor() {
        this._memoryCache = new Map();
        this._isLoading = false;
        this._pendingLoads = new Set();
        
        // Clear old localStorage cache on load
        this._clearOldCache();
    }

    /**
     * Get emoji fallback for file type
     * @param {Object} file - File object with is_dir property
     * @returns {string} Emoji character
     */
    getEmoji(file) {
        return file?.is_dir ? '📁' : '📄';
    }

    /**
     * Load icon for a specific file
     * @param {string} path - File path
     * @param {string} extension - File extension
     * @param {HTMLImageElement} imgElement - Image element to update
     */
    async loadIcon(path, extension, imgElement) {
        const cacheKey = getIconCacheKey(path, extension, UNIQUE_ICON_EXTENSIONS);
        
        // Return from memory cache if available
        if (this._memoryCache.has(cacheKey)) {
            const iconData = this._memoryCache.get(cacheKey);
            imgElement.src = iconData;
            imgElement.style.display = 'block';
            this._hideEmojiPlaceholder(imgElement);
            return;
        }

        // Skip if already loading this icon
        if (this._pendingLoads.has(cacheKey)) {
            return;
        }

        this._pendingLoads.add(cacheKey);

        try {
            const iconData = await invoke('get_file_icon', { path });
            
            if (iconData) {
                this._memoryCache.set(cacheKey, iconData);
                imgElement.src = iconData;
                imgElement.style.display = 'block';
                this._hideEmojiPlaceholder(imgElement);
            }
        } catch (error) {
            // Keep emoji placeholder on error
        } finally {
            this._pendingLoads.delete(cacheKey);
        }
    }

    /**
     * Hide emoji placeholder in icon cell
     * @param {HTMLImageElement} imgElement - Image element
     */
    _hideEmojiPlaceholder(imgElement) {
        const emoji = imgElement.parentElement?.querySelector('.icon-emoji');
        if (emoji) {
            emoji.style.display = 'none';
        }
    }

    /**
     * Check if icon is cached
     * @param {string} path - File path
     * @param {string} extension - File extension
     * @returns {boolean}
     */
    isCached(path, extension) {
        const cacheKey = getIconCacheKey(path, extension, UNIQUE_ICON_EXTENSIONS);
        return this._memoryCache.has(cacheKey);
    }

    /**
     * Get cached icon data URL
     * @param {string} path - File path
     * @param {string} extension - File extension
     * @returns {string|null} Cached icon URL or null
     */
    getCachedIcon(path, extension) {
        const cacheKey = getIconCacheKey(path, extension, UNIQUE_ICON_EXTENSIONS);
        return this._memoryCache.get(cacheKey) || null;
    }

    /**
     * Clear memory cache
     */
    clearMemoryCache() {
        this._memoryCache.clear();
    }

    /**
     * Clear old localStorage cache if version changed
     */
    _clearOldCache() {
        if (typeof localStorage !== 'undefined') {
            if (localStorage.getItem(CONFIG.CACHE_KEY) !== CONFIG.CACHE_VERSION) {
                localStorage.setItem(CONFIG.CACHE_KEY, CONFIG.CACHE_VERSION);
            }
        }
    }
}

/**
 * Lazy icon loader for virtual scrolling
 */
class LazyIconLoader {
    constructor(iconManager) {
        this._iconManager = iconManager;
        this._isLoading = false;
    }

    /**
     * Load icons for visible rows only
     * @param {HTMLElement} list - Results list element
     * @param {number} rowHeight - Row height in pixels
     * @param {number} viewportTop - Scroll position top
     * @param {number} viewportHeight - Viewport height
     * @param {Array} files - All files array
     * @param {number} startIndex - Starting index for visible rows
     * @param {number} endIndex - Ending index for visible rows
     */
    loadVisible(list, rowHeight, viewportTop, viewportHeight, files, startIndex, endIndex) {
        if (this._isLoading) return;
        this._isLoading = true;

        requestAnimationFrame(() => {
            const viewportBottom = viewportTop + viewportHeight;
            
            for (let i = startIndex; i < endIndex && i < files.length; i++) {
                const file = files[i];
                const rowTop = i * rowHeight;
                const rowBottom = rowTop + rowHeight;

                // Only load if row is visible
                if (rowBottom >= viewportTop && rowTop <= viewportBottom) {
                    const row = list.querySelector(`[data-index="${i}"]`);
                    if (row) {
                        const ext = file.path.split('.').pop().toLowerCase();
                        if (!this._iconManager.isCached(file.path, ext)) {
                            const img = row.querySelector('.icon-img');
                            if (img) {
                                this._iconManager.loadIcon(file.path, ext, img);
                            }
                        }
                    }
                }
            }

            this._isLoading = false;
        });
    }
}

// Export singleton instances
export const iconManager = new IconManager();
export const lazyIconLoader = new LazyIconLoader(iconManager);

// Debounced version for scroll events
export const debouncedLazyLoad = debounce(
    (list, rowHeight, viewportTop, viewportHeight, files, startIndex, endIndex) => {
        lazyIconLoader.loadVisible(list, rowHeight, viewportTop, viewportHeight, files, startIndex, endIndex);
    },
    CONFIG.ICON_LOAD_DEBOUNCE_MS
);
