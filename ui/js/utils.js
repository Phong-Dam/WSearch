/**
 * Utility functions
 * Pure helper functions with no side effects
 */
import { SIZE_UNITS } from './config.js';

/**
 * Creates a debounced version of a function
 * @param {Function} fn - Function to debounce
 * @param {number} delay - Delay in milliseconds
 * @returns {Function} Debounced function
 */
export function debounce(fn, delay) {
    let timer;
    return (...args) => {
        clearTimeout(timer);
        timer = setTimeout(() => fn(...args), delay);
    };
}

/**
 * Escapes HTML special characters to prevent XSS
 * @param {string} text - Text to escape
 * @returns {string} Escaped text
 */
export function escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Formats file size in human-readable format
 * @param {number} bytes - Size in bytes
 * @returns {string} Formatted size string
 */
export function formatSize(bytes) {
    if (bytes === 0) return '-';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    const size = bytes / Math.pow(1024, i);
    return size.toFixed(i > 0 ? 1 : 0) + ' ' + SIZE_UNITS[i];
}

/**
 * Gets file extension from path
 * @param {string} path - File path
 * @returns {string} Lowercase extension or empty string
 */
export function getFileExtension(path) {
    if (!path) return '';
    const parts = path.split('.');
    return parts.length > 1 ? parts.pop().toLowerCase() : '';
}

/**
 * Highlights query matches in text
 * @param {string} text - Text to highlight
 * @param {string} query - Query to highlight
 * @returns {string} HTML with highlighted matches
 */
export function highlightMatches(text, query) {
    if (!query || !text) return escapeHtml(text);
    const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`(${escaped})`, 'gi');
    return escapeHtml(text).replace(regex, '<span class="text-yellow-400">$1</span>');
}

/**
 * Generates cache key for icon caching
 * @param {string} path - File path
 * @param {string} extension - File extension
 * @param {Set} uniqueExts - Set of extensions with unique icons
 * @returns {string} Cache key
 */
export function getIconCacheKey(path, extension, uniqueExts) {
    if (uniqueExts.has(extension)) {
        return `file:${path}`;
    }
    return `ext:${extension || '_folder_'}`;
}

/**
 * Converts RGBA base64 to PNG data URL (used by icon loader)
 * @param {string} iconData - Icon data string
 * @returns {string|null} PNG data URL or null if invalid
 */
export function parseIconData(iconData) {
    if (!iconData || !iconData.includes(',')) return null;
    try {
        const parts = iconData.split(',');
        if (parts.length < 2) return null;
        const base64Data = parts[1];
        const size = parseInt(parts[2]) || 32;
        const rawData = atob(base64Data);
        const bytes = new Uint8ClampedArray(rawData.length);
        for (let i = 0; i < rawData.length; i++) {
            bytes[i] = rawData.charCodeAt(i);
        }
        const canvas = document.createElement('canvas');
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext('2d');
        const imageData = new ImageData(bytes, size, size);
        ctx.putImageData(imageData, 0, 0);
        return canvas.toDataURL('image/png');
    } catch {
        return null;
    }
}

/**
 * Sorts results by open_count, then by sort field
 * @param {Array} results - Results to sort
 * @param {string} sortField - Field to sort by
 * @param {number} sortDir - Sort direction (1 or -1)
 * @returns {Array} Sorted results
 */
export function sortResults(results, sortField, sortDir) {
    return [...results].sort((a, b) => {
        const aOpen = a.open_count ?? 0;
        const bOpen = b.open_count ?? 0;
        if (aOpen !== bOpen) {
            return bOpen - aOpen;
        }
        let valA = a[sortField];
        let valB = b[sortField];
        if (sortField === 'size') {
            return sortDir * ((valA ?? 0) - (valB ?? 0));
        }
        if (typeof valA === 'string') {
            return sortDir * valA.localeCompare(valB ?? '');
        }
        return 0;
    });
}
