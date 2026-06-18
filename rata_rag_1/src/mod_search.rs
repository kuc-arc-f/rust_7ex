use anyhow::{Context, Result};
use dotenvy::dotenv;
use reqwest::Client;
use reqwest::Error;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use rusqlite::{ffi::sqlite3_auto_extension, Connection, params};
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

fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f64, Box<dyn std::error::Error>> {
    if a.len() != b.len() {
        return Err(Box::new(VectorLengthError));
    }

    let mut dot_product = 0.0_f64;
    let mut a_magnitude = 0.0_f64;
    let mut b_magnitude = 0.0_f64;

    for i in 0..a.len() {
        dot_product += (a[i] * b[i]) as f64;
        a_magnitude += (a[i] * a[i]) as f64;
        b_magnitude += (b[i] * b[i]) as f64;
    }

    if a_magnitude == 0.0 || b_magnitude == 0.0 {
        return Ok(0.0);
    }

    Ok(dot_product / (a_magnitude.sqrt() * b_magnitude.sqrt()))
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
//pub async fn db_search(query_embedding: &[f32], k: usize, query: String) -> rusqlite::Result<Vec<SearchResult>> {
pub async fn db_search(
    query_embedding: &[f32], k: usize, query: String
) -> String {
    let mut ret: String = "".to_string();
    let db_url = super::DB_FILE;

    let conn = Connection::open(db_url).unwrap();
    //println!("#db_search-start");
    let items : Vec<SearchResult>  = Vec::new();

    #[derive(Debug)]
    struct FloatData {
        id: String,
        content: String,
        embeddings: Vec<f32>
    }
    #[derive(Debug)]
    struct ScoreData {
        id: String,
        content: String,
        score: f64
    }    

    // ---- SELECT 全件取得 ----
    //println!("\nSELECT 全件:");
    let mut stmt = conn.prepare(
        "SELECT id, name, content, embeddings FROM document",
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        let id: String      = row.get(0)?;
        let name: String    = "".to_string();
        let content: String = row.get(2)?;
        let blob: Vec<u8>   = row.get(3)?;  // BLOB は Vec<u8> で受け取る
        Ok((id, name, content, blob))
    }).unwrap();

    let mut vecItems = Vec::new();
    for row in rows {
        let (id, name, content, blob) = row.unwrap();
        let embeddings = blob_to_vec_f32(&blob);
        //println!("  id={}, name={}, content={}, embeddings={:?}",
        //    id, name, content, embeddings);
        vecItems.push(FloatData {
            id,
            content,
            embeddings: embeddings,
        });
    }
    let mut scoreItems = Vec::<ScoreData>::new();
    for row_item in &vecItems {
        let distance = cosine_similarity(&query_embedding, &row_item.embeddings).unwrap();
        if distance > 0.6 {
            //println!("id={}, distance={} \n", row_item.id, distance);
            scoreItems.push(ScoreData {
                id: row_item.id.clone(),
                content: row_item.content.clone(),
                score: distance,
            });
        }
    }
    // score の降順ソート
    scoreItems.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut outItems = Vec::<ScoreData>::new(); 
    let top_k = 1;
    let mut outCount = 0;
    for row_item in &scoreItems {
        //println!("id={}, score={} \n",row_item.id,  row_item.score);
        if outCount < top_k {
            outItems.push(ScoreData {
                id: row_item.id.clone(),
                content: row_item.content.clone(),
                score: row_item.score,
            });            
        }
        outCount += 1;
    }
    let mut matches : String = "".to_string();
    for row_item in &outItems {
        //println!("id={}, score={} \n",row_item.id,  row_item.score);
        let content_str = format!("{}\n\n", &row_item.content);
        matches.push_str(&content_str.clone().to_string());
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
 
    //return Ok(ret);
    return ret;
}