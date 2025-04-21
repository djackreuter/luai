use dotenvy::dotenv;
use mlua::{Lua, StdLib};
use openai_api_rs::v1::{api::OpenAIClient, chat_completion::{self, ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse}};

#[tokio::main]
async fn main() {
    let debug: bool = true;
    dotenv().unwrap();

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
        content: chat_completion::Content::Text(String::from("List running processes on the system")),
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

    let result: ChatCompletionResponse = client.chat_completion(req).await.unwrap();

    let choice: &ChatCompletionChoice = &result.choices[0];
    let generated_lua: String = choice.message.content.to_owned().unwrap();

    if debug {
        println!("{generated_lua}");
    }

    process_lua(generated_lua);
}

fn process_lua(script: String) {
    unsafe {
        let lua: Lua = Lua::unsafe_new();
        lua.load_std_libs(StdLib::ALL).unwrap();

        lua.load(script).exec().unwrap();
    }
}
