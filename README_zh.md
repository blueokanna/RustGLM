# RustGLM for ChatGLM Rust SDK - [English Doc](https://github.com/blueokanna/RustGLM/blob/main/README.md)
> 高性能、高品质体验和可靠的 Rust 语言 ChatGLM SDK 自然语言处理功能

## 1. 准备开始

### 1.1 安装 Rust-up 可删减程序（ 👇 此处仅显示 Windows 和 Android 文件）

[Rust-up-Windows-x64-Installation](https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe)

[Rust-up-Windows-x32-Installation](https://static.rust-lang.org/rustup/dist/i686-pc-windows-msvc/rustup-init.exe)

[Rust-up-aarch64-android-Installation](https://static.rust-lang.org/rustup/dist/aarch64-linux-android/rustup-init)

> 如果你是 `Linux` 用户 or `MacOS` 用户, 你可以点击这里进行查看: [用户安装手册](https://forge.rust-lang.org/infra/other-installation-methods.html)

<br>
<br>

1️⃣ 安装后，请使用 `命令行` 检查 Rust 版本：

```
cargo -V
```
or
```
cargo --version
```
<br>
<br>

2️⃣ **然后就可以使用命令将库添加到自己的项目中：**
```
cargo add RustGLM
```
or use
```
RustGLM = "0.1.1"
```

#### 您可能需要的其他 RustGLM 文档： 👉 :link: [RustGLM Documation](https://docs.rs/RustGLM/0.1.1/RustGLM/struct.RustGLM.html)
<br>
<br>

### 1.2 Rust NTP 时间服务器

它通过互联网或局域网上的时间服务器提供高度准确和安全的时间信息，确保所有设备使用相同的时间至关重要。这里的应用是通过以下方式进行 `JWT` 身份验证：

```
pub fn time_sync() -> i64 {
    let client = SntpClient::new();
    let result = client.synchronize("ntp.aliyun.com").unwrap();

    let local_time: DateTime<Local> =
        DateTime::from(result.datetime().into_chrono_datetime().unwrap());

    let milliseconds = local_time.timestamp_millis() as i64;
    return milliseconds;
}
```

### 1.3 保存 API 密钥

保存 api 密钥并将其存储到本地文件中，该文件称为 `chatglm_api_key.txt` 文件：

```
const API_KEY_FILE: &str = "chatglm_api_key.txt";

    pub fn save_api_key(api_key: &str) {
        match File::create(API_KEY_FILE) {
            Ok(mut file) => {
                if let Err(e) = writeln!(file, "{}", api_key) {
                    eprintln!("Failed to write to file: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to create file: {}", e),
        }
    }
```

**加载 API 密钥:**
```
pub fn load_api_key() -> Option<String> {
        match File::open(API_KEY_FILE) {
            Ok(file) => {
                let reader = BufReader::new(file);
                reader.lines().next().map(|line| line.unwrap_or_default())
            }
            Err(_) => None,
        }
    }
```

### 1.4 保存历史聊天记录

保存用户聊天内容和人工智能的历史聊天记录在 `chatglm_history.json` 文件.

```
const HISTORY_FILE: &str = "chatglm_history.json";

 pub fn add_history_to_file(&self, role: &str, content: &str) -> String {
        let json = self.create_json(role, content);

        if let Ok(mut file) = OpenOptions::new().write(true).append(true).open(&self.history_file_path) {
            if let Err(err) = writeln!(file, "{},", json) {
                eprintln!("Failed to write to history file: {}", err);
            }
        } else {
            eprintln!("Failed to open history file for writing");
        }

        json
    }
```
**加载聊天记录在:**
```
pub fn load_history_from_file(&self) -> String {
        if let Ok(file) = File::open(&self.history_file_path) {
            let reader = BufReader::new(file);
            reader.lines().filter_map(Result::ok).collect()
        } else {
            eprintln!("Failed to open history file for reading");
            String::new()
        }
    }
```

<br>
<br>

## 2. 易于使用的 SDK

### 2.1 调用和使用 Rust Crate.io 库
>
> 使用这个 **Rust** 项目调用 **SDK** 的难度较低🤩。下面的示例可以让你输入问题以及关键字，控制台会输出 **ChatGLM** 来回答问题：

🚩**输入关键字： 如果没有其他字符，将切换调用模式**

| 序列号 | 全名 | 关键字 |
| :-------------: | :-------------: | :----- |
| 1 | Server-Sent Events| SSE, sse |
| 2 | Asynchronous | ASYNC, Async, async |
| 3 | Synchronous | SYNC, Sync, sync |


**为自己的项目添加主函数的示例:**
```
//默认使用流式传输调用

#[tokio::main]
async fn main() {
    let mut rust_glm = RustGLM::RustGLM::new().await;
    loop {
        println!("You:");
        let ai_response = rust_glm.rust_chat_glm().await;
        if ai_response.is_empty() {
            break;
        }
        println!("Liliya: {}", rust_glm.get_ai_response());
        println!();
    }
}
```


> 总体下来，这个项目引入的三种请求方式应该还是比较简单的，目前的 **BUG** 会尽量修复🥳，也希望各位开发者对这个项目的支持！再次感谢🎉！
---

## 4.总结
>
> 感谢您打开我的项目，这是一个自主开发的使用 **Rust** 编程语言所开发的项目，目的是针对官方 SDK 的要求扩展不同的代码语言调用。我也在努力开发和更新这个项目，当然，我个人也会继续开发这个项目，我也更坚持开源的原则，让大家都能喜欢我的项目。最后，希望越来越多的人一起参与进来 🚀 感谢您看到最后！ 😆👏

