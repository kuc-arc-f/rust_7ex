# rata_rag_1

 Version: 0.9.1

 date    : 2026/06/15

 update :

***

Rust Window , RAG Search + SQLite database

* rustc 1.93.0 
* embedding: gemini-embedding-001
* model: gemma-4-E2B
* llama.cpp , llama-server use
* windows11

***
### vector data add

https://github.com/kuc-arc-f/rust_5ex/blob/main/rag_5sql

***
## image

![img1](/images/rata_rag_1.png)

***
## setup

* llama-server start
* port 8090: gemma-4-E2B

```
#gemma-4-E2B
/usr/local/llama-b8642/llama-server -m /var/lm_data/unsloth/gemma-4-E2B-it-Q4_K_S.gguf \
 --chat-template-kwargs '{"enable_thinking": false}' --port 8090 

```

***
### related
https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF

***
* db file copy
* rust_5ex/rag_5sql/example.db , db file copy

***
### env value

```
SET GEMINI_API_KEY="your-key"
```
* PowerShell

```
$Env:GEMINI_API_KEY ="your-key"
```

***
* build
```
cargo build
cargo run
```

***
### blog

https://zenn.dev/knaka0209/scraps/16ff6a2ff913bc


