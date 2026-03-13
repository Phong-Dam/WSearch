# WSearch

**[English](README.md)** | **Tiếng Việt**

> **⚠️ CẢNH BÁO: DỰ ÁN NGHIÊN CỨU**  
> Ứng dụng này được phát triển **CHỈ VÌ MỤC ĐÍCH NGHIÊN CỨU VÀ HỌC TẬP**.  
> **KHÔNG SỬ DỤNG** trong môi trường sản xuất hoặc cho mục đích thương mại.  
> Tác giả không chịu trách nhiệm về bất kỳ thiệt hại nào phát sinh từ việc sử dụng phần mềm này.

Một ứng dụng tìm kiếm file nhanh và hiệu quả cho Windows được xây dựng bằng Tauri và Rust.

## Về WSearch

WSearch là một công cụ tìm kiếm file desktop được thiết kế để giúp bạn nhanh chóng tìm thấy các file và thư mục trên máy tính của mình. Với giao diện trực quan và khả năng tìm kiếm nhanh, WSearch giúp bạn tiết kiệm thời gian khi cần tìm file.

**Lưu ý:** Đây là dự án thử nghiệm nhằm mục đích học tập về Tauri, Rust, và Windows API. Code có thể chứa lỗi, thiếu tính năng bảo mật, hoặc không được tối ưu hóa đầy đủ.

## Tính năng chính

- **Tìm kiếm nhanh**: Thuật toán binary search + substring matching
- **Fuzzy Search**: Hỗ trợ tìm kiếm mờ có thể bật/tắt qua giao diện
- **Icon Windows gốc**: Hiển thị icon thực tế của file từ hệ thống Windows (32x32)
- **Lazy Loading**: Tải icon động khi scroll để tối ưu hiệu suất
- **Virtual Scrolling**: Render chỉ các item hiển thị trên màn hình
- **Cache thông minh**: 
  - Gzip compression cho file cache (giảm ~40-60% dung lượng)
  - Icon cache với chiến lược phân biệt (theo extension vs theo path)
- **Giao diện thân thiện**: Dark mode với Tailwind CSS
- **Phím tắt toàn cục**: Mở ứng dụng nhanh từ bất kỳ đâu
- **Loading Screen**: Hiển thị trạng thái khi đang load cache/index disk
- **Theo dõi file được mở**: Ghi lại open_count để ưu tiên kết quả

## Công nghệ sử dụng

- **Frontend**: Vanilla JavaScript, Tailwind CSS
- **Backend**: Rust (Tauri 2.10.3)
- **Windows API**: SHGetFileInfoW, DrawIconEx cho icon extraction
- **Parallel Processing**: Rayon cho xử lý đa luồng
- **Compression**: flate2 (gzip) cho cache
- **Serialization**: bincode
- **File Walking**: jwalk cho duyệt thư mục nhanh

## Yêu cầu hệ thống

- Windows 10/11
- ~50-100MB RAM (tùy số lượng file được index)
- ~5-20MB disk cho cache (đã nén)

## Hạn chế và vấn đề đã biết

- **Hiệu suất**: Indexing lần đầu có thể mất 10-30 giây tùy số lượng file
- **Bảo mật**: Không có cơ chế xác thực hoặc mã hóa
- **Ổn định**: Có thể crash khi xử lý file/folder với quyền hạn đặc biệt
- **Testing**: Chưa có unit tests hoặc integration tests đầy đủ
- **Icon cache**: Có thể tốn nhiều RAM nếu mở nhiều file .exe khác nhau
- **Thread safety**: Có thể có race conditions chưa được phát hiện
- **Error handling**: Một số lỗi chưa được xử lý đầy đủ

## Build từ source

```bash
# Cài đặt Rust (nếu chưa có)
# https://rustup.rs/

# Clone repository
git clone <repo-url>
cd wsearch

# Build release
cargo build --release

# Chạy ứng dụng
cargo tauri dev
```

## Giấy phép và Miễn trừ trách nhiệm

**MIỄN TRỪ TRÁCH NHIỆM:**

PHẦN MỀM NÀY ĐƯỢC CUNG CẤP "NGUYÊN TRẠNG", KHÔNG CÓ BẤT KỲ BẢO ĐẢM NÀO, RÕ RÀNG HAY NGỤ Ý, BAO GỒM NHƯNG KHÔNG GIỚI HẠN Ở CÁC BẢO ĐẢM VỀ KHẢ NĂNG THƯƠNG MẠI, PHÙ HỢP CHO MỘT MỤC ĐÍCH CỤ THỂ VÀ KHÔNG VI PHẠM.

TRONG BẤT KỲ TRƯỜNG HỢP NÀO, TÁC GIẢ HOẶC CHỦ SỞ HỮU BẢN QUYỀN KHÔNG CHỊU TRÁCH NHIỆM CHO BẤT KỲ KHIẾU NẠI, THIỆT HẠI HOẶC TRÁCH NHIỆM PHÁP LÝ NÀO, DÙ TRONG HỢP ĐỒNG, TỘI LỖI HAY CÁCH KHÁC, PHÁT SINH TỪ, NGOÀI HOẶC LIÊN QUAN ĐẾN PHẦN MỀM HOẶC VIỆC SỬ DỤNG HOẶC CÁC GIAO DỊCH KHÁC TRONG PHẦN MỀM.

**DỰ ÁN NÀY CHỈ DÀNH CHO MỤC ĐÍCH GIÁO DỤC VÀ NGHIÊN CỨU.**

---

**Tác giả**: [Phong-Dam](https://github.com/Phong-Dam)  
**Mục đích**: Học tập về desktop app development, Windows API, và Rust programming
