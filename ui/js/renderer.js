/**
 * Renderer module - Virtual scrolling and DOM rendering
 */
import { CONFIG } from './config.js';
import { iconManager } from './icons.js';
import {
    escapeHtml,
    formatSize,
    highlightMatches
} from './utils.js';

/**
 * Virtual list renderer
 */
class VirtualRenderer {
    constructor() {
        this._lastStartIndex = -1;
        this._listElement = null;
        this._scrollListener = null;
    }

    /**
     * Initialize renderer with list element
     * @param {HTMLElement} listElement - Results container element
     */
    init(listElement) {
        this._listElement = listElement;
    }

    /**
     * Render visible rows
     * @param {Array} results - Results array
     * @param {number} scrollTop - Scroll position
     * @param {string} query - Current search query
     * @param {number} selectedIndex - Currently selected index
     */
    render(results, scrollTop, query, selectedIndex) {
        if (!this._listElement) return;

        const startIndex = Math.floor(scrollTop / CONFIG.ROW_HEIGHT);
        const endIndex = startIndex + CONFIG.VISIBLE_COUNT;

        // Only re-render if viewport changed significantly
        if (this._lastStartIndex === startIndex) {
            return false; // No re-render needed
        }

        this._lastStartIndex = startIndex;
        
        const offset = startIndex * CONFIG.ROW_HEIGHT;
        const slice = results.slice(startIndex, endIndex);

        this._listElement.innerHTML = this._createContainerHTML(results.length, offset, slice, startIndex, selectedIndex, query);
        
        return true; // Re-render happened
    }

    /**
     * Create container HTML with virtual scroll spacer
     * @param {number} totalCount - Total results count
     * @param {number} offset - Offset in pixels
     * @param {Array} slice - Visible slice of results
     * @param {number} startIndex - Start index
     * @param {number} selectedIndex - Selected index
     * @param {string} query - Search query
     * @returns {string} HTML string
     */
    _createContainerHTML(totalCount, offset, slice, startIndex, selectedIndex, query) {
        const totalHeight = totalCount * CONFIG.ROW_HEIGHT;
        
        return `
            <div style="height:${totalHeight}px;position:relative">
                <div style="transform:translateY(${offset}px)">
                    ${slice.map((file, i) => this._createRowHTML(file, startIndex + i, selectedIndex, query)).join('')}
                </div>
            </div>
        `;
    }

    /**
     * Create a single row HTML
     * @param {Object} file - File object
     * @param {number} index - Row index
     * @param {number} selectedIndex - Selected index
     * @param {string} query - Search query
     * @returns {string} HTML string
     */
    _createRowHTML(file, index, selectedIndex, query) {
        const isActive = index === selectedIndex;
        const emoji = iconManager.getEmoji(file ?? {});
        const name = file?.name ?? 'Unknown';
        const size = typeof file?.size === 'number' ? file.size : 0;
        const path = file?.path ?? '';

        return `
            <div class="row ${isActive ? 'bg-blue-600' : 'hover:bg-zinc-800'}"
                 data-index="${index}"
                 data-path="${escapeHtml(path)}"
                 data-name="${escapeHtml(name)}">
                ${this._createIconCell(emoji)}
                <div class="truncate pr-4">${highlightMatches(name, query)}</div>
                <div class="truncate text-zinc-400 text-xs">${formatSize(size)}</div>
                <div class="truncate text-zinc-400">${escapeHtml(path)}</div>
            </div>
        `;
    }

    /**
     * Create icon cell HTML
     * @param {string|null} cachedIcon - Cached icon data URL
     * @param {string} emoji - Fallback emoji
     * @returns {string} HTML string
     */
    _createIconCell(emoji) {
        // Icons disabled - always show emoji
        return `
            <div class="flex justify-center items-center">
                <span class="icon-emoji">${emoji}</span>
            </div>
        `;
    }

    /**
     * Reset render state (force re-render on next call)
     */
    reset() {
        this._lastStartIndex = -1;
    }

    /**
     * Get visible range info
     * @param {number} scrollTop - Scroll position
     * @returns {Object} { start, end }
     */
    getVisibleRange(scrollTop) {
        const start = Math.floor(scrollTop / CONFIG.ROW_HEIGHT);
        return {
            start,
            end: start + CONFIG.VISIBLE_COUNT
        };
    }
}

/**
 * Column resizer class
 */
class ColumnResizer {
    constructor() {
        this._resizers = new Map();
    }

    /**
     * Setup resizer for a column
     * @param {string} resizerId - Element ID
     * @param {string} cssVarName - CSS variable name
     * @param {number} minWidth - Minimum width
     */
    setup(resizerId, cssVarName, minWidth = 60) {
        const resizer = document.getElementById(resizerId);
        if (!resizer) return;

        let isResizing = false;
        let startX = 0;
        let startWidth = 0;

        resizer.addEventListener('mousedown', (e) => {
            isResizing = true;
            resizer.classList.add('resizing');
            document.body.style.cursor = 'col-resize';
            
            startX = e.pageX;
            startWidth = parseInt(getComputedStyle(document.documentElement).getPropertyValue(cssVarName));

            const onMouseMove = (e) => {
                if (!isResizing) return;
                const newWidth = startWidth + (e.pageX - startX);
                if (newWidth > minWidth) {
                    document.documentElement.style.setProperty(cssVarName, `${newWidth}px`);
                }
            };

            const onMouseUp = () => {
                isResizing = false;
                resizer.classList.remove('resizing');
                document.body.style.cursor = 'default';
                document.removeEventListener('mousemove', onMouseMove);
                document.removeEventListener('mouseup', onMouseUp);
            };

            document.addEventListener('mousemove', onMouseMove);
            document.addEventListener('mouseup', onMouseUp);
        });
    }

    /**
     * Setup all column resizers
     */
    setupAll() {
        this.setup('resizer-name', '--col-name', CONFIG.COLUMN_NAME_MIN_WIDTH);
        this.setup('resizer-size', '--col-size', CONFIG.COLUMN_SIZE_MIN_WIDTH);
    }
}

// Export singleton instances
export const renderer = new VirtualRenderer();
export const columnResizer = new ColumnResizer();
