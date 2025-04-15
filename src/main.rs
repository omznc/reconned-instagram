use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use futures::future::join_all;
use chrono::{DateTime, Utc};
use std::time::Duration;
use std::env;

// The expected token is now loaded from environment variable
fn get_auth_token() -> String {
    env::var("AUTH_TOKEN").unwrap_or_else(|_| {
        eprintln!("WARNING: AUTH_TOKEN environment variable not set, using default value");
        "secret_token".to_string()
    })
}

#[derive(Serialize)]
struct InstagramPost {
    image_url: String,
    video_preview_url: Option<String>,
    direct_link: String,
    date: String,
}

#[derive(Serialize)]
struct InstagramUserPosts {
    username: String,
    full_name: String,
    biography: String,
    profile_pic_url: String,
    is_private: bool,
    is_verified: bool,
    followers_count: i64,
    following_count: i64,
    posts_count: i64,
    posts: Vec<InstagramPost>,
}

// Use this structure to parse the endpoint query parameters.
// It supports both a single username and a comma‑separated list.
#[derive(Deserialize)]
struct QueryParams {
    token: String,
    // if provided, the "usernames" parameter contains a comma-separated list.
    usernames: Option<String>,
    // alternative single username parameter.
    username: Option<String>,
}

async fn fetch_instagram_posts(client: &Client, username: &str) -> Result<InstagramUserPosts, reqwest::Error> {
    // Direct approach to fetch posts without relying on user ID first
    let url = format!("https://www.instagram.com/api/v1/users/web_profile_info/?username={}", username);
    
    println!("Fetching Instagram data for user: {}", username);
    
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:137.0) Gecko/20100101 Firefox/137.0")
        .header("Accept", "*/*")
        .header("Accept-Language", "en-US,en;q=0.5")
        .header("X-IG-App-ID", "936619743392459") // Instagram App ID
        .header("X-ASBD-ID", "359341")
        .header("X-IG-WWW-Claim", "0")
        .header("X-Web-Device-Id", "D08769DB-E84E-4D0D-AF5D-C16D7ED28411") // This could be randomized in production
        .header("X-Web-Session-ID", "session") // This could be randomized in production
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Sec-GPC", "1")
        .timeout(Duration::from_secs(15))
        .send()
        .await?;
    
    let status = resp.status();    
    if !status.is_success() {
        return Ok(InstagramUserPosts {
            username: username.to_string(),
            full_name: String::new(),
            biography: String::new(),
            profile_pic_url: String::new(),
            is_private: false,
            is_verified: false,
            followers_count: 0,
            following_count: 0,
            posts_count: 0,
            posts: Vec::new(),
        });
    }
    
    // Get the response body as text first for debugging
    let body_text = resp.text().await?;
    
    // Try to parse the JSON
    let data = match serde_json::from_str::<serde_json::Value>(&body_text) {
        Ok(json) => json,
        Err(_) => {
            return Ok(InstagramUserPosts {
                username: username.to_string(),
                full_name: String::new(),
                biography: String::new(),
                profile_pic_url: String::new(),
                is_private: false,
                is_verified: false,
                followers_count: 0,
                following_count: 0,
                posts_count: 0,
                posts: Vec::new(),
            });
        }
    };
    
    // Extract user information
    let user_data = data.get("data").and_then(|d| d.get("user"));
    
    // Extract user profile information
    let full_name = user_data
        .and_then(|u| u.get("full_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
        
    let biography = user_data
        .and_then(|u| u.get("biography"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
        
    let profile_pic_url = user_data
        .and_then(|u| u.get("profile_pic_url"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
        
    let is_private = user_data
        .and_then(|u| u.get("is_private"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
        
    let is_verified = user_data
        .and_then(|u| u.get("is_verified"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
        
    // Get follower and following counts
    let followers_count = user_data
        .and_then(|u| u.get("edge_followed_by"))
        .and_then(|f| f.get("count"))
        .and_then(|c| c.as_i64())
        .unwrap_or(0);
        
    let following_count = user_data
        .and_then(|u| u.get("edge_follow"))
        .and_then(|f| f.get("count"))
        .and_then(|c| c.as_i64())
        .unwrap_or(0);
    
    let mut posts = Vec::new();
    let mut posts_count = 0;
    
    // Extract posts from the response based on the actual structure
    // The structure follows: data.user.edge_owner_to_timeline_media.edges[].node
    if let Some(user_data) = user_data {
        if let Some(media) = user_data.get("edge_owner_to_timeline_media") {
            // Get total posts count
            posts_count = media.get("count")
                .and_then(|c| c.as_i64())
                .unwrap_or(0);
                
            if let Some(edges) = media.get("edges") {
                if let Some(edges_array) = edges.as_array() {                    
                    for edge in edges_array.iter().take(7) {
                        if let Some(node) = edge.get("node") {
                            // Extract image URL
                            let image_url = node.get("display_url")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            
                            // Extract video preview if available
                            let video_preview_url = if node.get("is_video").and_then(|v| v.as_bool()).unwrap_or(false) {
                                Some(image_url.clone())
                            } else {
                                None
                            };
                            
                            // Extract shortcode for direct link
                            let shortcode = node.get("shortcode")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            
                            let direct_link = format!("https://www.instagram.com/p/{}/", shortcode);
                            
                            // Extract timestamp
                            let timestamp = node.get("taken_at_timestamp")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);
                            
                            let date = if timestamp > 0 {
                                DateTime::<Utc>::from_timestamp(timestamp, 0)
                                    .map(|dt| dt.to_string())
                                    .unwrap_or_else(|| String::from("Unknown date"))
                            } else {
                                String::from("Unknown date")
                            };
                            
                            posts.push(InstagramPost {
                                image_url,
                                video_preview_url,
                                direct_link,
                                date,
                            });
                        }
                    }
                }
            }
        }
    }
    
    
    Ok(InstagramUserPosts {
        username: username.to_string(),
        full_name,
        biography,
        profile_pic_url,
        is_private,
        is_verified,
        followers_count,
        following_count,
        posts_count,
        posts,
    })
}

// TypeScript return type:
// export type InstagramApiResponse = InstagramUserPosts[];
async fn instagram_handler(query: web::Query<QueryParams>) -> impl Responder {
    // Validate token
    if query.token != get_auth_token() {
        return HttpResponse::Unauthorized().body("Invalid token");
    }

    // Determine the list of usernames to query.
    let usernames: Vec<String> = if let Some(usernames_str) = &query.usernames {
        usernames_str.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else if let Some(username) = &query.username {
        vec![username.clone()]
    } else {
        return HttpResponse::BadRequest().body("No username provided");
    };

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    // Process each username concurrently.
    let fetches = usernames.iter()
        .map(|uname| fetch_instagram_posts(&client, uname));
    let results = join_all(fetches).await;
    
    // Collect the results. If a call fails, you can decide on a fallback action.
    // Here, we report an empty list for usernames that failed.
    let mut users_posts = Vec::new();
    for (i, res) in results.into_iter().enumerate() {
        match res {
            Ok(data) => users_posts.push(data),
            Err(_) => {

                users_posts.push(InstagramUserPosts { 
                    username: usernames.get(i).unwrap_or(&String::from("unknown")).clone(),
                    full_name: String::new(),
                    biography: String::new(),
                    profile_pic_url: String::new(),
                    is_private: false,
                    is_verified: false,
                    followers_count: 0,
                    following_count: 0,
                    posts_count: 0,
                    posts: vec![] 
                });
            }
        }
    }

    HttpResponse::Ok().json(users_posts)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting Instagram API server on http://0.0.0.0:8080");
    // Bind the server to all interfaces on port 8080 for container compatibility
    HttpServer::new(|| {
        App::new()
            .route("/api/instagram_posts", web::get().to(instagram_handler))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
