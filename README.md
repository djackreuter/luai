# luai

[Blog post](https://blog.shellntel.com/p/luai-an-ai-malware-agent)

## Usage
Copy `.env.example` to `.env` and update the values.
```
OPENAI_API_KEY=""
LUAI_API_KEY="" // This must match API_KEY in luai_web
SERVER_URL="" // Address where luai_web is running. E.g., http://127.0.0.1:5000
```

1. Adjust jitter on line 26 to your liking. This is the number of seconds Luai will wait before contacting the server for tasking.<br>
2. Configure `DEBUG` on line 15 to be `true` or `false`. Verbose info printed if `true` (default).
3. Compile: `cargo build -r`
4. Execute: `.\target\release\luai.exe`
