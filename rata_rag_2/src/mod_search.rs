use anyhow::{Context, Result};
use dotenvy::dotenv;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, Distance, Filter, PointStruct, ScalarQuantizationBuilder,
    SearchParamsBuilder, SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant, QdrantError};
use reqwest::Client;
use reqwest::Error;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
//use rusqlite::{ffi::sqlite3_auto_extension, Connection, params};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::env;

/// リクエストボディの構造体
#[derive(Serialize)]
struct EmbedContentRequest {
    model: String,
    content: Content,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}


#[derive(Debug)]
pub struct SearchResult {
    id:       i64,
    title:    String,
    content:  String,
    source:   Option<String>,
    distance: f64,
}

/// レスポンスボディの構造体
#[derive(Deserialize, Debug)]
struct EmbedContentResponse {
    embedding: Embedding,
}

#[derive(Deserialize, Debug)]
struct Embedding {
    values: Vec<f32>,
}

#[derive(Debug)]
struct VectorLengthError;

impl fmt::Display for VectorLengthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "vectors must have the same length")
    }
}
impl std::error::Error for VectorLengthError {}

/// Gemini Embedding API を呼び出してベクトルを取得する関数
pub async fn get_embedding(api_key: &str, text: &str) -> anyhow::Result<Vec<f32>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent?key={}",
        api_key
    );

    let request_body = EmbedContentRequest {
        model: "models/gemini-embedding-001".to_string(),
        content: Content {
            parts: vec![Part {
                text: text.to_string(),
            }],
        },
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("APIへのリクエスト送信に失敗しました")?;


    // HTTPステータスコードの確認
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("APIエラー ({}): {}", status, error_text);
    }

    // レスポンスをデシリアライズ
    let embed_response: EmbedContentResponse = response
        .json()
        .await
        .context("レスポンスのJSONパースに失敗しました")?;

    //return embed_response.embedding.values;
    Ok(embed_response.embedding.values)
}
/// BLOB (バイト列) を Vec<f32> に変換
fn blob_to_vec_f32(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect()
}


/**
*
* @param
*
* @return
*/
async fn send_post(input : String) -> String{
    let mut ret = "".to_string();
    #[derive(Serialize)]
    struct Message {
        role: String,
        content: String,
    }

    #[derive(Serialize)]
    struct ChatRequest {
        model: String,
        messages: Vec<Message>,
        temperature: f32,
    }
    #[derive(Debug, Deserialize)]
    struct ChatResponse {
        choices: Vec<Choice>,
    }

    #[derive(Debug, Deserialize)]
    struct Choice {
        message: MessageContent,
    }

    #[derive(Debug, Deserialize)]
    struct MessageContent {
        role: String,
        content: String,
    }
    let client = Client::new();
    let request_body = ChatRequest {
        model: "qwen3.5-2b".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: input.to_string(),
            }
        ],
        temperature: 0.7,
    };
    let response = client
        .post("http://localhost:8090/v1/chat/completions")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await.unwrap();

    let result: ChatResponse = response.json().await.unwrap();

    if let Some(choice) = result.choices.first() {
        //println!("AI: {}", choice.message.content);
        ret = choice.message.content.clone();
        return ret;

    } 
    return ret;
}

/// KNN検索 → 上位K件のドキュメントを返す
pub async fn db_search(
    query_embedding: &[f32], k: usize, query: String
) -> String {
    let mut ret: String = "".to_string();
    let clientQdrant = Qdrant::from_url("http://localhost:6334").build().unwrap();

    let input_f32 = query_embedding;
    //println!("input_f32.len={}", input_f32.len());
    let search_result = clientQdrant
        .search_points(
            SearchPointsBuilder::new(super::COLLECT_NAME, input_f32, 1)
                .with_payload(true)
                .params(SearchParamsBuilder::default().exact(true)),
        )
        .await.unwrap();

    let resplen = search_result.result.len();
    //println!("#list-start={}", resplen);
    let mut matches : String = "".to_string();    
    for row_resp in &search_result.result {
        //println!("score={}\n", &row_resp.score);
        let content = &row_resp.payload["content"];
        let content_str = format!("{}\n\n", content);
        if row_resp.score > 0.6f32 {
            matches.push_str(&content_str.clone().to_string());            
        }
    }
    let mut out_str : String = "".to_string();
    if matches.len() > 0 {
        out_str = format!("context: {}\n", matches);
        let out_add2 = format!("user query: {}\n" , query);
        out_str.push_str(&out_add2);
    }else {
        out_str = format!("user query: {}\n", query);
    } 
    let send_text = format!("日本語で、回答して欲しい。\n 要約して欲しい。\n\n{}", out_str);
    //println!("send_text={}\n", send_text);
    let resp = send_post(send_text).await;
    ret = resp;
 
    return ret;
}