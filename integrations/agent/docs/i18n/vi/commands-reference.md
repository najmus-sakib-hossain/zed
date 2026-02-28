# Tham khảo lệnh DX

Dựa trên CLI hiện tại (`dx --help`).

Xác minh lần cuối: **2026-02-28**.

## Lệnh cấp cao nhất

| Lệnh | Mục đích |
|---|---|
| `onboard` | Khởi tạo workspace/config nhanh hoặc tương tác |
| `agent` | Chạy chat tương tác hoặc chế độ gửi tin nhắn đơn |
| `gateway` | Khởi động gateway webhook và HTTP WhatsApp |
| `daemon` | Khởi động runtime có giám sát (gateway + channels + heartbeat/scheduler tùy chọn) |
| `service` | Quản lý vòng đời dịch vụ cấp hệ điều hành |
| `doctor` | Chạy chẩn đoán và kiểm tra trạng thái |
| `status` | Hiển thị cấu hình và tóm tắt hệ thống |
| `cron` | Quản lý tác vụ định kỳ |
| `models` | Làm mới danh mục model của provider |
| `providers` | Liệt kê ID provider, bí danh và provider đang dùng |
| `channel` | Quản lý kênh và kiểm tra sức khỏe kênh |
| `integrations` | Kiểm tra chi tiết tích hợp |
| `skills` | Liệt kê/cài đặt/gỡ bỏ skills |
| `migrate` | Nhập dữ liệu từ runtime khác (hiện hỗ trợ OpenClaw) |
| `config` | Xuất schema cấu hình dạng máy đọc được |
| `completions` | Tạo script tự hoàn thành cho shell ra stdout |
| `hardware` | Phát hiện và kiểm tra phần cứng USB |
| `peripheral` | Cấu hình và nạp firmware thiết bị ngoại vi |

## Nhóm lệnh

### `onboard`

- `dx onboard`
- `dx onboard --interactive`
- `dx onboard --channels-only`
- `dx onboard --api-key <KEY> --provider <ID> --memory <sqlite|lucid|markdown|none>`
- `dx onboard --api-key <KEY> --provider <ID> --model <MODEL_ID> --memory <sqlite|lucid|markdown|none>`

### `agent`

- `dx agent`
- `dx agent -m "Hello"`
- `dx agent --provider <ID> --model <MODEL> --temperature <0.0-2.0>`
- `dx agent --peripheral <board:path>`

### `gateway` / `daemon`

- `dx gateway [--host <HOST>] [--port <PORT>] [--new-pairing]`
- `dx daemon [--host <HOST>] [--port <PORT>]`

`--new-pairing` sẽ xóa toàn bộ token đã ghép đôi và tạo mã ghép đôi mới khi gateway khởi động.

### `service`

- `dx service install`
- `dx service start`
- `dx service stop`
- `dx service restart`
- `dx service status`
- `dx service uninstall`

### `cron`

- `dx cron list`
- `dx cron add <expr> [--tz <IANA_TZ>] <command>`
- `dx cron add-at <rfc3339_timestamp> <command>`
- `dx cron add-every <every_ms> <command>`
- `dx cron once <delay> <command>`
- `dx cron remove <id>`
- `dx cron pause <id>`
- `dx cron resume <id>`

### `models`

- `dx models refresh`
- `dx models refresh --provider <ID>`
- `dx models refresh --force`

`models refresh` hiện hỗ trợ làm mới danh mục trực tiếp cho các provider: `openrouter`, `openai`, `anthropic`, `groq`, `mistral`, `deepseek`, `xai`, `together-ai`, `gemini`, `ollama`, `llamacpp`, `sglang`, `vllm`, `astrai`, `venice`, `fireworks`, `cohere`, `moonshot`, `glm`, `zai`, `qwen`, `volcengine` (alias `doubao`/`ark`), `siliconflow` và `nvidia`.

### `channel`

- `dx channel list`
- `dx channel start`
- `dx channel doctor`
- `dx channel bind-telegram <IDENTITY>`
- `dx channel add <type> <json>`
- `dx channel remove <name>`

Lệnh trong chat khi runtime đang chạy (Telegram/Discord):

- `/models`
- `/models <provider>`
- `/model`
- `/model <model-id>`

Channel runtime cũng theo dõi `config.toml` và tự động áp dụng thay đổi cho:
- `default_provider`
- `default_model`
- `default_temperature`
- `api_key` / `api_url` (cho provider mặc định)
- `reliability.*` cài đặt retry của provider

`add/remove` hiện chuyển hướng về thiết lập có hướng dẫn / cấu hình thủ công (chưa hỗ trợ đầy đủ mutator khai báo).

### `integrations`

- `dx integrations info <name>`

### `skills`

- `dx skills list`
- `dx skills install <source>`
- `dx skills remove <name>`

`<source>` chấp nhận git remote (`https://...`, `http://...`, `ssh://...` và `git@host:owner/repo.git`) hoặc đường dẫn cục bộ.

Skill manifest (`SKILL.toml`) hỗ trợ `prompts` và `[[tools]]`; cả hai được đưa vào system prompt của agent khi chạy, giúp model có thể tuân theo hướng dẫn skill mà không cần đọc thủ công.

### `migrate`

- `dx migrate openclaw [--source <path>] [--dry-run]`

### `config`

- `dx config schema`

`config schema` xuất JSON Schema (draft 2020-12) cho toàn bộ hợp đồng `config.toml` ra stdout.

### `completions`

- `dx completions bash`
- `dx completions fish`
- `dx completions zsh`
- `dx completions powershell`
- `dx completions elvish`

`completions` chỉ xuất ra stdout để script có thể được source trực tiếp mà không bị lẫn log/cảnh báo.

### `hardware`

- `dx hardware discover`
- `dx hardware introspect <path>`
- `dx hardware info [--chip <chip_name>]`

### `peripheral`

- `dx peripheral list`
- `dx peripheral add <board> <path>`
- `dx peripheral flash [--port <serial_port>]`
- `dx peripheral setup-uno-q [--host <ip_or_host>]`
- `dx peripheral flash-nucleo`

## Kiểm tra nhanh

Để xác minh nhanh tài liệu với binary hiện tại:

```bash
dx --help
dx <command> --help
```
