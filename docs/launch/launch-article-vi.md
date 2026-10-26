# Đã chạy được. Vậy điều gì đã thay đổi?

**Evidence state: available adapted draft; publication is deferred until the P25 go/no-go approval.**

What Changed là ứng dụng Windows ghi lại lịch sử thay đổi ngay trên máy. Khi
một dịch vụ không còn trả lời hoặc một dự án bỗng lỗi, bạn mô tả triệu chứng và
xem các bằng chứng được xếp hạng để tự kiểm tra.

Ứng dụng nối các tín hiệu có cấu trúc — cấu hình, dịch vụ, tiến trình và cổng
đang lắng nghe — trong một dòng thời gian. Mục điều tra giải thích vì sao một
thay đổi được xếp hạng cao; nó không khẳng định tương quan là nguyên nhân.

Không có tài khoản, tải lịch sử lên mạng hay telemetry khi chạy. Giá trị nhạy
cảm được thay bằng `[REDACTED]` trước khi lưu. Ứng dụng không giữ hash hay dấu
vết suy ra từ bí mật để so sánh sau khi khởi động lại.

Đây là bản preview cho Windows, không yêu cầu quyền quản trị và không gọi mạng
khi chạy. Một số giới hạn được nói rõ: tiến trình sống rất ngắn có thể bị bỏ
qua, vài kênh Windows cần quyền cao hơn, và đường chạy Docker thật chưa được
kiểm chứng trên máy phát triển. Installer chưa ký số nên SmartScreen có thể
hiện “Unknown publisher”. Xem [tài liệu tiếng Anh chính thức](../wiki-index.md)
để biết chi tiết kỹ thuật.
