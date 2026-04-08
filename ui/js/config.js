/**
 * Application configuration constants
 * Single source of truth for all magic numbers and settings
 */
export const CONFIG = Object.freeze({
    // UI Dimensions
    ROW_HEIGHT: 32,
    VISIBLE_COUNT: 70,
    ICON_SIZE: 32,

    // Timing (milliseconds)
    SEARCH_DEBOUNCE_MS: 40,
    ICON_LOAD_DEBOUNCE_MS: 150,
    RENDER_DELAY_MS: 50,
    INDEX_CHECK_INTERVAL_MS: 500,
    LOADING_SCREEN_DELAY_MS: 1000,
    BENCHMARK_INTERVAL_MS: 1000,

    // Column widths
    COLUMN_ICON_WIDTH: 32,
    COLUMN_NAME_MIN_WIDTH: 100,
    COLUMN_SIZE_MIN_WIDTH: 60,

    // Cache
    CACHE_VERSION: 'v2',
    CACHE_KEY: 'iconCacheVersion',
});

/**
 * Column definitions
 * Single source of truth for column configuration
 */
export const COLUMNS = Object.freeze({
    ICON: { key: 'icon', width: 'var(--col-icon)', sortable: false },
    NAME: { key: 'name', label: 'Name', dataKey: 'name', width: 'var(--col-name)', sortable: true },
    SIZE: { key: 'size', label: 'Size', dataKey: 'size', width: 'var(--col-size)', sortable: true },
    PATH: { key: 'path', label: 'Path', dataKey: 'path', width: 'var(--col-path)', sortable: true },
});

/**
 * File extensions that have unique icons per file
 * (don't cache by extension alone)
 */
export const UNIQUE_ICON_EXTENSIONS = new Set([
    'exe', 'lnk', 'dll', 'ico', 'scr', 'cpl', 'msc'
]);

/**
 * Size units for formatting
 */
export const SIZE_UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/**
 * CSS Custom Properties defaults
 */
export const CSS_VARS = Object.freeze({
    '--col-icon': '32px',
    '--col-name': '260px',
    '--col-size': '80px',
    '--col-path': '1fr',
});

/**
 * Context menu items configuration
 */
export const CONTEXT_MENU_ITEMS = Object.freeze([
    { id: 'ctxOpen', label: 'Open File', icon: '📂' },
    { id: 'ctxShowFolder', label: 'Show in Folder', icon: '📁' },
    { id: 'ctxCopyPath', label: 'Copy Path', icon: '📋' },
    { id: 'ctxCopyName', label: 'Copy Name', icon: '📝' },
]);

/**
 * Loading screen messages
 */
export const LOADING_MESSAGES = Object.freeze({
    INITIALIZING: 'Khởi tạo...',
    LOADING_CACHE: 'Đang tải cache...',
    INDEXING: 'Đang quét ổ đĩa...',
    COMPLETE: 'Hoàn tất!',
    WAITING: 'Vui lòng đợi trong giây lát...',
    FOUND_FILES: 'Đã tìm thấy %d files',
});

/**
 * Default state values
 */
export const DEFAULT_STATE = Object.freeze({
    sortField: 'name',
    sortDirection: 1,
    useFuzzy: true,
    isIndexing: true,
    selectedIndex: 0,
    lastIndexCount: 0,
});
