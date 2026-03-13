# WSearch

**English** | **[Tiếng Việt](README.vi.md)**

> **⚠️ WARNING: RESEARCH PROJECT**  
> This application is developed **FOR RESEARCH AND EDUCATIONAL PURPOSES ONLY**.  
> **DO NOT USE** in production environments or for commercial purposes.  
> The author assumes no responsibility for any damage arising from the use of this software.

A fast and efficient desktop file search application for Windows built with Tauri and Rust.

## About

WSearch is a desktop file search tool designed to help you quickly find files and folders on your computer. With an intuitive interface and fast search capabilities, WSearch saves you time when you need to find files.

**Note:** This is an experimental project for learning purposes about Tauri, Rust, and Windows API. The code may contain bugs, lack security features, or not be fully optimized.

## Key Features

- **Fast Search**: Binary search + substring matching algorithm
- **Fuzzy Search**: Toggleable fuzzy search via UI
- **Native Windows Icons**: Display real file icons from Windows system (32x32)
- **Lazy Loading**: Load icons dynamically on scroll for optimized performance
- **Virtual Scrolling**: Render only visible items on screen
- **Smart Caching**: 
  - Gzip compression for file cache (~40-60% size reduction)
  - Icon cache with differentiated strategy (by extension vs by path)
- **User-friendly Interface**: Dark mode with Tailwind CSS
- **Global Hotkeys**: Quick app launch from anywhere
- **Loading Screen**: Display status while loading cache/indexing disk
- **Track Opened Files**: Record open_count to prioritize results

## Technology Stack

- **Frontend**: Vanilla JavaScript, Tailwind CSS
- **Backend**: Rust (Tauri 2.10.3)
- **Windows API**: SHGetFileInfoW, DrawIconEx for icon extraction
- **Parallel Processing**: Rayon for multi-threading
- **Compression**: flate2 (gzip) for cache
- **Serialization**: bincode
- **File Walking**: jwalk for fast directory traversal

## System Requirements

- Windows 10/11
- ~50-100MB RAM (depending on number of indexed files)
- ~5-20MB disk space for cache (compressed)

## Known Limitations and Issues

- **Performance**: Initial indexing may take 10-30 seconds depending on file count
- **Security**: No authentication or encryption mechanism
- **Stability**: May crash when handling files/folders with special permissions
- **Testing**: No comprehensive unit tests or integration tests yet
- **Icon cache**: May consume significant RAM if opening many different .exe files
- **Thread safety**: Potential undiscovered race conditions
- **Error handling**: Some errors not fully handled

## Build from Source

```bash
# Install Rust (if not already installed)
# https://rustup.rs/

# Clone repository
git clone <repo-url>
cd wsearch

# Build release
cargo build --release

# Run application
cargo tauri dev
```

## License and Disclaimer

**DISCLAIMER:**

THIS SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.

IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

**THIS PROJECT IS FOR EDUCATIONAL AND RESEARCH PURPOSES ONLY.**

---

**Author**: [Phong-Dam](https://github.com/Phong-Dam)
**Purpose**: Learning desktop app development, Windows API, and Rust programming
