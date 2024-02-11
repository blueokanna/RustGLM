# RustGLM for ChatGLM Rust SDK
> High-performance, high-quality Experience and Reliable ChatGLM SDK natural language processing in Rust-Language

## 1. Prepare beginning

### 1.1 Install Rust-up excutable programme (👇 Here only display Windows and Android files)

[Rust-up-Windows-x64-Installation](https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe)

[Rust-up-Windows-x32-Installation](https://static.rust-lang.org/rustup/dist/i686-pc-windows-msvc/rustup-init.exe)

[Rust-up-aarch64-android-Installation](https://static.rust-lang.org/rustup/dist/aarch64-linux-android/rustup-init)

> if you are `Linux` user or `MacOS` user, please check here: [Installation-User-Manual](https://forge.rust-lang.org/infra/other-installation-methods.html)

After installation please use `Command Line`  to Check Rust Version:

```
cargo -V
```
or
```
cargo --version
```


### 1.2 NTP Time Server for Rust

It provides highly accurate and secure time information via time servers on the Internet or LAN, and it is critical to ensure that all devices use the same time. The application here is for `JWT` authentication using：

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

### 1.3 Store API Key

Saving Api key and store it in local file which call `chatglm_api_key` txt file:

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
Load ChatGLM API key:

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

### 1.4 Save Chat Content file

User chats and AI replies will be stored in `chatglm_history.txt`.

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
Load History Content from history file:
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


----

## 2. Easy-to-use SDK

### 2.1 Calling and Using the Rust Crate.io Library
>
> Using this rust project **SDK** is less difficult 🤩. The following three examples to let you enter your question and the console will output **ChatGLM** to answer it：

🚩**Enter the keywords: If there are no other characters, it will switch the Calling mode**

> Type the following keywords to switch the Calling mode:

| Number | Full-Name | KeyWords |
| :-------------: | :-------------: | :----- |
| 1 | Server-Sent Events| SSE, sse |
| 2 | Asynchronous | ASYNC, Async, async |
| 3 | Synchronous | SYNC, Sync, sync |

---
**Main Method:**
```
#[tokio::main]
pub async fn main() {
    let mut api_key = api_operation::APIKeys::load_api_key();
    let mut input = String::new();
    let mut require_calling = "SSE".to_string();
    let mut ai_message = String::new();

    if api_key.is_none() {
        println!("Enter your API Key:");
        if let Ok(_) = io::stdin().read_line(&mut input) {
            api_key = Some(input.trim().to_string());
            api_operation::APIKeys::save_api_key(api_key.as_ref().unwrap());
        } else {
            eprintln!("Unable to read user input");
            return;
        }
    }

    if let Some(api_key) = api_key {
        let api_key_instance = api_operation::APIKeys::get_instance(&*api_key);
        let jwt_creator = custom_jwt::CustomJwt::new(api_key_instance.get_user_id(), api_key_instance.get_user_secret());
        let jwt = jwt_creator.create_jwt();

        let jwt_to_verify = jwt.clone();
        let is_valid = jwt_creator.verify_jwt(&jwt_to_verify);

        if is_valid {
            loop {
                println!("Eenter the conversation:");
                let mut user_input = String::new();
                io::stdin().read_line(&mut user_input).expect("Failed to read input");

                match user_input.trim().to_lowercase().as_str() {
                    "sse" => {
                        require_calling = "SSE".to_string();
                        println!("Calling method is SSE");
                        continue;
                    }
                    "async" => {
                        require_calling = "ASYNC".to_string();
                        println!("Calling method is Async");
                        continue;
                    }
                    "sync" => {
                        require_calling = "SYNC".to_string();
                        println!("Calling method is Sync");
                        continue;
                    }
                    _ => {}
                }

                if require_calling == "SSE" || require_calling == "sse" {
                    ai_message = sse_invoke_calling(&jwt, &user_input.trim()).await;
                } else if require_calling == "async" || require_calling == "ASYNC" || require_calling == "Async" {
                    ai_message = async_invoke_calling(&jwt, &user_input.trim()).await;
                } else if require_calling == "sync" || require_calling == "SYNC" || require_calling == "Sync" {
                    ai_message = sync_invoke_calling(&jwt, &user_input.trim()).await;
                }

                println!("Liliya: {}", ai_message);
                println!("\nYou: ");
            }
        } else {
            println!("JWT is NOT valid");
        }
    } else {
        println!("API Key not found or an error occurred while loading.");
    }
}
```


> Overall down, the introduction of this project three ways to request should still be relatively simple, the current **BUG** will try to fix 🥳, but also hope that all the developer of the support of this project! Thanks again 🎉!
---

## 4.Conclusion
>
> Thank you for opening my project, this is a self-developed RustGLM development project, in order to expand different code language calling for the official SDK requirments. I am also working hard to develop and update this project, of course, I personally will continue to develop this project, I also adhere to the principle of open source more, so that everyone can enjoy my project. Finally, I hope more and more people will participate together 🚀 Thank you for seeing the end! 😆👏

