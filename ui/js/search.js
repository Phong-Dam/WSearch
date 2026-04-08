/**
 * Search module - Handles file searching
 */
import { CONFIG } from './config.js';
import { debounce, sortResults } from './utils.js';
import { state } from './state.js';
import { renderer } from './renderer.js';
import { lazyIconLoader } from './icons.js';

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);

/**
 * Search manager class
 */
class SearchManager {
    constructor() {
        this._searchId = 0;
        this._debouncedSearch = null;
    }

    /**
     * Initialize search with input element
     * @param {HTMLInputElement} inputElement - Search input element
     */
    init(inputElement) {
        this._inputElement = inputElement;
        this._debouncedSearch = debounce(
            this._performSearch.bind(this),
            CONFIG.SEARCH_DEBOUNCE_MS
        );

        inputElement.addEventListener('input', (e) => {
            this._debouncedSearch(e.target.value);
        });

        inputElement.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                this._performSearch(inputElement.value);
            }
        });
    }

    /**
     * Perform search
     * @param {string} query - Search query
     * @private
     */
    async _performSearch(query) {
        const trimmedQuery = query.trim();

        // Clear results if empty query
        if (!trimmedQuery) {
            state.set({
                results: [],
                query: ''
            });
            renderer.reset();
            renderer.render([], 0, '', 0);
            return;
        }

        state.set({ 
            query: trimmedQuery,
            isSearching: true 
        });

        try {
            const id = ++this._searchId;
            const useFuzzy = state.get('useFuzzy');
            
            const response = await invoke('search_files', {
                query: trimmedQuery,
                searchId: id,
                useFuzzy
            });

            // Ignore stale results
            if (response.search_id !== this._searchId) {
                return;
            }

            // Sort results
            const sortedResults = sortResults(
                response.results,
                state.get('sortField'),
                state.get('sortDirection')
            );
            
            state.set({
                results: sortedResults,
                selectedIndex: 0,
                isSearching: false
            });

            renderer.reset();
            renderer.render(sortedResults, 0, trimmedQuery, 0);

            // Trigger icon loading for visible rows
            this._loadVisibleIcons();

        } catch (error) {
            console.error('Search error:', error);
            state.set({
                results: [],
                isSearching: false,
                error: error.message || 'Search failed'
            });
            renderer.render([], 0, '', 0);
        }
    }

    /**
     * Load icons for visible rows
     * @private
     */
    _loadVisibleIcons() {
        const results = state.get('results');
        if (results.length === 0) return;

        const listElement = document.getElementById('results');
        if (!listElement) return;

        const scrollTop = listElement.scrollTop;
        const { start, end } = renderer.getVisibleRange(scrollTop);

        lazyIconLoader.loadVisible(
            listElement,
            CONFIG.ROW_HEIGHT,
            scrollTop,
            listElement.clientHeight,
            results,
            start,
            end
        );
    }

    /**
     * Get current search ID
     * @returns {number}
     */
    getSearchId() {
        return this._searchId;
    }
}

/**
 * Index status checker
 */
class IndexStatusChecker {
    constructor() {
        this._interval = null;
        this._lastCount = 0;
        this._onStatusChange = null;
        this._hasReachedComplete = false;
    }

    /**
     * Start checking index status
     * @param {Function} callback - Callback(status: {count, status})
     */
    start(callback) {
        this._onStatusChange = callback;
        this._lastCount = 0;
        this._hasReachedComplete = false;
        this._checkStatus();
        this._interval = setInterval(
            () => this._checkStatus(),
            CONFIG.INDEX_CHECK_INTERVAL_MS
        );
    }

    /**
     * Stop checking index status
     */
    stop() {
        if (this._interval) {
            clearInterval(this._interval);
            this._interval = null;
        }
    }

    /**
     * Check index status
     * @private
     */
    async _checkStatus() {
        try {
            const [count, isIndexing, isLoadingCache] = await invoke('get_index_status');
            this._processStatus(count, isIndexing, isLoadingCache);
        } catch (error) {
            console.error('Failed to check index status:', error);
        }
    }

    _processStatus(count, isIndexing, isLoadingCache) {
        const lastCount = this._lastCount;
        this._lastCount = count;

        let status;

        // Once we've reached complete state, never go back
        if (this._hasReachedComplete) {
            status = 'complete';
        } else if (isLoadingCache) {
            status = 'loading';
        } else if (isIndexing) {
            status = 'indexing';
        } else {
            this._hasReachedComplete = true;
            status = 'complete';
        }

        if (this._onStatusChange) {
            this._onStatusChange({ count, status, lastCount });
        }
    }
}

// Export singleton instances
export const searchManager = new SearchManager();
export const indexStatusChecker = new IndexStatusChecker();
