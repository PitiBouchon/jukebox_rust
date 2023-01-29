#[cfg(test)]
use super::*;

// Test lib

#[tokio::test]
async fn test_search() {
    assert!(!search_videos("youtube rewind").await.is_empty());
}

#[tokio::test]
async fn test_url() {
    let url = get_best_audio("YbJOTdZBX1g").await.unwrap().url;
    let resp = reqwest::get(url).await.unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::from_u16(200).unwrap());
}

#[tokio::test]
async fn test_age_restriction() {
    let test_get = get_best_audio("QdabIfmcqSQ").await;
    assert!(test_get.is_err());
}

#[tokio::test]
async fn test_private_video() {
    let test_get = get_best_audio("PA63CIfy2TY").await;
    assert!(test_get.is_err());
}

#[tokio::test]
async fn test_removed_video() {
    let test_get = get_best_audio("j6qaPxf7EV4").await;
    assert!(test_get.is_err());
}

#[tokio::test]
async fn test_unavailible_video() {
    let test_get = get_best_audio("s22bAaMRGic").await;
    assert!(test_get.is_err());
}

#[tokio::test]
async fn test_author_problem_video() {
    let test_get = get_best_audio("OLBTIUzPpEQ").await;
    assert!(test_get.is_err());
}
