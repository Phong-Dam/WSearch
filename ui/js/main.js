/**
 * WSearch - Main Application Entry Point
 */
import { CONFIG, CSS_VARS, LOADING_MESSAGES } from './config.js';
import { state } from './state.js';
import { renderer, columnResizer } from './renderer.js';
import { searchManager, indexStatusChecker } from './search.js';
import { keyboardHandler } from './keyboard.js';
import { contextMenu } from './context.js';
import { toastManager } from './toast.js';
import { lazyIconLoader } from './icons.js';
import { benchmarkDisplay } from './bench.js';

/**
 * Application class
 */
class WSearchApp {
    constructor() {
        this._loadingScreen = null;
        this._mainContent = null;
        this._loadingText = null;
        this._loadingCount = null;
    }

    /**
     * Initialize the application
     */
    async init() {
        // Cache DOM elements
        this._cacheElements();
        
        // Initialize CSS variables
        this._initCSSVariables();
        
        // Initialize components
        toastManager.init();
        renderer.init(document.getElementById('results'));
        columnResizer.setupAll();
        
        // Setup event listeners
        this._setupEventListeners();
        
        // Initialize search
        searchManager.init(document.getElementById('search'));
        
        // Initialize keyboard navigation
        keyboardHandler.init(document.getElementById('results'));
        
        // Initialize context menu
        contextMenu.init(document.getElementById('results'));

        // Initialize benchmark display
        benchmarkDisplay.init();

        // Load fuzzy preference from localStorage
        this._loadPreferences();
        
        // Start index status monitoring
        this._startIndexMonitoring();
        
        // Initial render
        renderer.render([], 0, '', 0);
    }

    /**
     * Cache frequently used DOM elements
     * @private
     */
    _cacheElements() {
        this._loadingScreen = document.getElementById('loadingScreen');
        this._mainContent = document.getElementById('mainContent');
        this._loadingText = document.getElementById('loadingText');
        this._loadingCount = document.getElementById('loadingCount');
    }

    /**
     * Initialize CSS custom properties
     * @private
     */
    _initCSSVariables() {
        Object.entries(CSS_VARS).forEach(([varName, value]) => {
            document.documentElement.style.setProperty(varName, value);
        });
    }

    /**
     * Setup event listeners
     * @private
     */
    _setupEventListeners() {
        // Scroll event for virtual scrolling
        const listElement = document.getElementById('results');
        listElement.addEventListener('scroll', () => {
            const scrollTop = listElement.scrollTop;
            const results = state.get('results');
            const query = state.get('query');
            const selectedIndex = state.get('selectedIndex');

            renderer.render(results, scrollTop, query, selectedIndex);

            // Load icons for visible rows
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
        });

        // Click event for opening files
        listElement.addEventListener('click', (e) => {
            const row = e.target.closest('.row');
            if (!row) return;

            const index = parseInt(row.dataset.index);
            const file = state.get('results')[index];

            if (file?.path) {
                window.__TAURI__.core.invoke('open_path', { path: file.path });
                window.__TAURI__.core.invoke('record_open', { path: file.path });
            }
        });

        // Fuzzy toggle
        const fuzzyCheckbox = document.getElementById('useFuzzy');
        if (fuzzyCheckbox) {
            fuzzyCheckbox.addEventListener('change', (e) => {
                state.set({ useFuzzy: e.target.checked });
                localStorage.setItem('useFuzzy', e.target.checked);

                // Re-search if there's a query
                const query = state.get('query');
                if (query) {
                    searchManager.init(document.getElementById('search'));
                }
            });
        }

        // Benchmark toggle
        const benchmarkToggle = document.getElementById('benchmarkToggle');
        if (benchmarkToggle) {
            benchmarkToggle.addEventListener('click', () => {
                benchmarkDisplay.toggle();
            });
        }
    }

    /**
     * Load user preferences from localStorage
     * @private
     */
    _loadPreferences() {
        const savedFuzzy = localStorage.getItem('useFuzzy');
        if (savedFuzzy !== null) {
            state.set({ useFuzzy: savedFuzzy !== 'false' });
        }

        const fuzzyCheckbox = document.getElementById('useFuzzy');
        if (fuzzyCheckbox) {
            fuzzyCheckbox.checked = state.get('useFuzzy');
        }
    }

    /**
     * Start index status monitoring
     * @private
     */
    _startIndexMonitoring() {
        indexStatusChecker.start(({ count, status }) => {
            this._updateLoadingUI(count, status);
        });
    }

    /**
     * Update loading screen UI
     * @param {number} count - File count
     * @param {string} status - Status: 'loading', 'indexing', 'complete', 'idle'
     * @private
     */
    _updateLoadingUI(count, status) {
        switch (status) {
            case 'loading':
                this._showLoadingScreen(LOADING_MESSAGES.LOADING_CACHE, LOADING_MESSAGES.WAITING);
                break;
                
            case 'indexing':
                // If count is 0, show initializing message
                // If count > 0, show indexing message
                if (count === 0) {
                    this._showLoadingScreen(LOADING_MESSAGES.INITIALIZING, LOADING_MESSAGES.WAITING);
                } else {
                    this._showLoadingScreen(
                        LOADING_MESSAGES.INDEXING,
                        `Đã tìm thấy ${count.toLocaleString()} files`
                    );
                }
                break;
                
            case 'complete':
                this._completeLoading(count);
                break;
        }
    }

    /**
     * Show loading screen
     * @param {string} text - Loading text
     * @param {string} countText - Count text
     * @private
     */
    _showLoadingScreen(text, countText) {
        this._loadingText.textContent = text;
        this._loadingCount.textContent = countText;
        this._loadingScreen.classList.remove('hidden');
        this._mainContent.classList.add('hidden');
        state.set({ isIndexing: true });
    }

    /**
     * Complete loading and show main content
     * @param {number} count - Total file count
     * @private
     */
    _completeLoading(count) {
        if (state.get('isIndexing')) {
            this._loadingText.textContent = LOADING_MESSAGES.COMPLETE;
            this._loadingCount.textContent = `Tổng cộng ${count.toLocaleString()} files`;
            
            setTimeout(() => {
                this._loadingScreen.classList.add('hidden');
                this._mainContent.classList.remove('hidden');
                document.getElementById('search')?.focus();
                state.set({ isIndexing: false });
            }, CONFIG.LOADING_SCREEN_DELAY_MS);
        }
    }
}

// Export app class
export { WSearchApp };

// Auto-initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    const app = new WSearchApp();
    app.init();
    
    // Also expose to window for debugging
    window.__wsearchApp = app;
});
