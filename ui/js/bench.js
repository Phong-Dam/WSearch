/**
 * Benchmark metrics module
 */
import { CONFIG } from './config.js';

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);

/**
 * Benchmark display class
 */
class BenchmarkDisplay {
    constructor() {
        this._elements = {};
        this._interval = null;
    }

    /**
     * Initialize benchmark display
     */
    init() {
        this._cacheElements();
        this._startPolling();
    }

    /**
     * Cache DOM elements
     * @private
     */
    _cacheElements() {
        this._elements = {
            searchTime: document.getElementById('bmSearchTime'),
            avgSearch: document.getElementById('bmAvgSearch'),
            searchCount: document.getElementById('bmSearchCount'),
            fileCount: document.getElementById('bmFileCount'),
            cacheSize: document.getElementById('bmCacheSize'),
            memory: document.getElementById('bmMemory'),
            fuzzyTime: document.getElementById('bmFuzzyTime'),
            fuzzyCount: document.getElementById('bmFuzzyCount'),
            panel: document.getElementById('benchmarkPanel'),
            toggle: document.getElementById('benchmarkToggle'),
        };
    }

    /**
     * Start polling for metrics
     * @private
     */
    _startPolling() {
        this._updateMetrics();
        this._interval = setInterval(() => this._updateMetrics(), CONFIG.BENCHMARK_INTERVAL_MS);
    }

    /**
     * Update metrics display
     * @private
     */
    async _updateMetrics() {
        try {
            const metrics = await invoke('get_benchmark_metrics');

            if (this._elements.searchTime) {
                this._elements.searchTime.textContent = metrics.last_search_time_ms?.toFixed(2) ?? '-';
            }
            if (this._elements.avgSearch) {
                this._elements.avgSearch.textContent = metrics.avg_search_time_ms?.toFixed(2) ?? '-';
            }
            if (this._elements.searchCount) {
                this._elements.searchCount.textContent = metrics.search_count ?? 0;
            }
            if (this._elements.fileCount) {
                this._elements.fileCount.textContent = (metrics.indexed_file_count ?? 0).toLocaleString();
            }
            if (this._elements.cacheSize) {
                this._elements.cacheSize.textContent = this._formatBytes(metrics.cache_size_bytes ?? 0);
            }
            if (this._elements.memory) {
                this._elements.memory.textContent = (metrics.memory_usage_mb ?? 0).toFixed(1);
            }
            if (this._elements.fuzzyTime) {
                const avgFuzzy = metrics.fuzzy_search_count > 0
                    ? (metrics.total_fuzzy_time_ms / metrics.fuzzy_search_count).toFixed(2)
                    : '-';
                this._elements.fuzzyTime.textContent = avgFuzzy;
            }
            if (this._elements.fuzzyCount) {
                this._elements.fuzzyCount.textContent = metrics.fuzzy_search_count ?? 0;
            }
        } catch (error) {
            // Silently fail - benchmark is not critical
        }
    }

    /**
     * Format bytes to human readable
     * @param {number} bytes
     * @returns {string}
     */
    _formatBytes(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }

    /**
     * Toggle benchmark panel visibility
     */
    toggle() {
        if (this._elements.panel) {
            const isHidden = this._elements.panel.classList.contains('hidden');
            this._elements.panel.classList.toggle('hidden', !isHidden);
        }
    }

    /**
     * Stop polling
     */
    stop() {
        if (this._interval) {
            clearInterval(this._interval);
            this._interval = null;
        }
    }
}

// Export singleton instance
export const benchmarkDisplay = new BenchmarkDisplay();