# Sổ tay Vận hành DX

Tài liệu này dành cho các operator chịu trách nhiệm duy trì tính sẵn sàng, tình trạng bảo mật và xử lý sự cố.

Cập nhật lần cuối: **2026-02-18**.

## Phạm vi

Dùng tài liệu này cho các tác vụ vận hành day-2:

- khởi động và giám sát runtime
- kiểm tra sức khoẻ và chẩn đoán hệ thống
- triển khai an toàn và rollback
- phân loại và khôi phục sau sự cố

Nếu đây là lần cài đặt đầu tiên, hãy bắt đầu từ [one-click-bootstrap.md](one-click-bootstrap.md).

## Các chế độ Runtime

| Chế độ | Lệnh | Khi nào dùng |
|---|---|---|
| Foreground runtime | `dx daemon` | gỡ lỗi cục bộ, phiên ngắn |
| Foreground gateway only | `dx gateway` | kiểm thử webhook endpoint |
| User service | `dx service install && dx service start` | runtime được quản lý liên tục bởi operator |

## Checklist Cơ bản cho Operator

1. Xác thực cấu hình:

```bash
dx status
```

1. Kiểm tra chẩn đoán:

```bash
dx doctor
dx channel doctor
```

1. Khởi động runtime:

```bash
dx daemon
```

1. Để chạy như user session service liên tục:

```bash
dx service install
dx service start
dx service status
```

## Tín hiệu Sức khoẻ và Trạng thái

| Tín hiệu | Lệnh / File | Kỳ vọng |
|---|---|---|
| Tính hợp lệ của config | `dx doctor` | không có lỗi nghiêm trọng |
| Kết nối channel | `dx channel doctor` | các channel đã cấu hình đều khoẻ mạnh |
| Tóm tắt runtime | `dx status` | provider/model/channels như mong đợi |
| Heartbeat/trạng thái daemon | `~/.dx/daemon_state.json` | file được cập nhật định kỳ |

## Log và Chẩn đoán

### macOS / Windows (log của service wrapper)

- `~/.dx/logs/daemon.stdout.log`
- `~/.dx/logs/daemon.stderr.log`

### Linux (systemd user service)

```bash
journalctl --user -u dx.service -f
```

## Quy trình Phân loại Sự cố (Fast Path)

1. Chụp trạng thái hệ thống:

```bash
dx status
dx doctor
dx channel doctor
```

1. Kiểm tra trạng thái service:

```bash
dx service status
```

1. Nếu service không khoẻ, khởi động lại sạch:

```bash
dx service stop
dx service start
```

1. Nếu các channel vẫn thất bại, kiểm tra allowlist và thông tin xác thực trong `~/.dx/config.toml`.

2. Nếu liên quan đến gateway, kiểm tra cài đặt bind/auth (`[gateway]`) và khả năng tiếp cận cục bộ.

## Quy trình Thay đổi An toàn

Trước khi áp dụng thay đổi cấu hình:

1. sao lưu `~/.dx/config.toml`
2. chỉ áp dụng một thay đổi logic tại một thời điểm
3. chạy `dx doctor`
4. khởi động lại daemon/service
5. xác minh bằng `status` + `channel doctor`

## Quy trình Rollback

Nếu một lần triển khai gây ra suy giảm hành vi:

1. khôi phục `config.toml` trước đó
2. khởi động lại runtime (`daemon` hoặc `service`)
3. xác nhận khôi phục qua `doctor` và kiểm tra sức khoẻ channel
4. ghi lại nguyên nhân gốc rễ và biện pháp khắc phục sự cố

## Tài liệu Liên quan

- [one-click-bootstrap.md](one-click-bootstrap.md)
- [troubleshooting.md](troubleshooting.md)
- [config-reference.md](config-reference.md)
- [commands-reference.md](commands-reference.md)
