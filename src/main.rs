use std::{collections::HashMap, error::Error};

use dotenvy::dotenv;
use mlua::{Lua, StdLib};
use openai_api_rs::v1::{api::OpenAIClient, chat_completion::{self, ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse}};
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

    let instruction: String = get_instruction().await;
    if instruction == String::new() {
        println!("No Instruction");
        return;
    }

    let attempts: i32 = 1;

    let result: String = ai_gen_lua(&instruction, attempts).await;

    println!("Lua result: {result}");

    send_result(&result, attempts).await;

}

async fn ai_gen_lua(instruction: &String, mut attempts: i32) -> String {
    match ai_execute(instruction).await {
        Ok(r) => {
            return String::from(r);
        },
        Err(_e) => {
            if attempts < 3 {
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
    You are a senior Red Team operator and experienced developer. Your job is to output a Lua script to accomplish the task you are given. The script will be executed with LuaJIT, so you have full access to Lua's FFI library. The script will run on a Windows system. Do not include "local ffi = require("ffi")" in any of the code. It has already been included and the "ffi" variable is available for use. You are to make all ffi calls with "ffi.C" convention.

	It is imperative that you write accurate and error free code. Think step by step through the process in order to accomplish the task.

	Do not add any comments to the code. Do not print any text. Do not print your thought process. Only print code.

    To avoid errors, use the following type definitions when necessary for translating Lua to C types:
    typedef void* HANDLE;
    typedef void* PVOID;
    typedef void* LPVOID;
    typedef uint16_t WORD;
    typedef unsigned long DWORD;
    typedef const char* LPCSTR;
    typedef int BOOL;
    typedef unsigned long long ULONG_PTR;
    typedef char TCHAR;
    typedef size_t SIZE_T;
    typedef unsigned short wchar_t;
    typedef DWORD (*LPTHREAD_START_ROUTINE)(LPVOID);

    Define all types you will use at the beginning of the file before defining any structs or functions.
    Store the result of the script in a string variable named "result" and return that variable at the end of the script.
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
        String::from("o3-mini"),
        vec![
            system_message,
            user_message
        ]
    );

    println!("[+] Processing");

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