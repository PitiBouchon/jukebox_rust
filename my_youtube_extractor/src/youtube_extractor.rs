use crate::youtube_info::*;

use phf::{phf_set, Set};
use reqwest::StatusCode;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub enum ErrorExtractor {
    ErrorParsing(String),
    ErrorReqwest,
}

impl std::fmt::Display for ErrorExtractor {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorExtractor::ErrorParsing(err) => write!(f, "Error parsing : {}", err),
            ErrorExtractor::ErrorReqwest => write!(f, "Error with the reqwest"),
        }
    }
}

// See : https://gist.github.com/sidneys/7095afe4da4ae58694d128b1034e01e2
// or : https://tyrrrz.me/blog/reverse-engineering-youtube/
// m4a  : 139 | 140 | 141 | 256 | 258 | 325 | 328
// webm : 171 | 172 | 249 | 250 | 251
static AUDIO_ITAGS: Set<u32> = phf_set! {
    // mp4
    139u32,
    140u32,
    141u32,
    256u32,
    258u32,
    325u32,
    328u32,
    // webm
    171u32,
    172u32,
    249u32,
    250u32,
    252u32,
};

/// Information of a webpage like search or video pages
pub struct YtPageData {
    pub url: String,
    webpage: String,
    yt_initial_data: Value,
}

impl YtPageData {
    pub async fn new(url: &str) -> Result<Self, ErrorExtractor> {
        let request_url = url.to_owned() + "&bpctr=9999999999&has_verified=1";

        let webpage = get_webpage(&request_url).await?;
        let yt_initial_data = get_var(&webpage, "var ytInitialData")?;

        Ok(Self {
            url: request_url,
            webpage,
            yt_initial_data,
        })
    }

    pub fn videos_search_info(&self) -> Result<Vec<YtVideoPageInfo>, ErrorExtractor> {
        if !self.url.contains("https://www.youtube.com/results") {
            panic!("{} is not a search url", self.url);
        }

        Ok(YtPageData::get_search_videos(&self.yt_initial_data)?
            .iter()
            .filter_map(YtPageData::get_video_info)
            .collect())
    }

    fn get_video_info(v: &Value) -> Option<YtVideoPageInfo> {
        let video_info = match v.get("videoRenderer") {
            None => return None,
            Some(vi) => vi,
        };

        if let Some(v_id) = video_info.get("videoId") {
            let id = v_id.as_str()?.to_string();

            let title = video_info
                .get("title")?
                .get("runs")?
                .get(0)?
                .get("text")?
                .as_str()?
                .to_string();

            let thumbnail = video_info
                .get("thumbnail")?
                .get("thumbnails")?
                .get(0)?
                .get("url")?
                .as_str()?
                .to_string();

            let author_name = video_info
                .get("ownerText")?
                .get("runs")?
                .get(0)?
                .get("text")?
                .as_str()?
                .to_string();

            let author_thumbnail = video_info
                .get("channelThumbnailSupportedRenderers")?
                .get("channelThumbnailWithLinkRenderer")?
                .get("thumbnail")?
                .get("thumbnails")?
                .get(0)?
                .get("url")?
                .as_str()?
                .to_string();

            if let Some(vct) = video_info.get("viewCountText")?.get("simpleText") {
                if let Some(lt) = video_info.get("lengthText") {
                    Some(YtVideoPageInfo {
                        id,
                        short_recap: "".to_string(), // TODO()
                        title,
                        thumbnail,
                        author: YtAuthorInfo {
                            name: author_name,
                            thumbnail: author_thumbnail,
                            tag: "".to_string(), // TODO()
                        },
                        meta_description: "".to_string(), // TODO()
                        duration: lt.get("simpleText")?.as_str()?.to_string(),
                        n_views: vct.as_str()?.to_string(),
                        date: "".to_string(), // TODO()
                    })
                } else {
                    log::debug!("Live video : https://www.youtube.com/watch?v={}", id);
                    None
                }
            } else {
                log::debug!(
                    "Probably live video : https://www.youtube.com/watch?v={}",
                    id
                );
                None
            }

            // let short_recap = video_info
            //     .get("title").unwrap()
            //     .get("accessibility").unwrap()
            //     .get("accessibilityData").unwrap()
            //     .get("label").unwrap()
            //     .as_str().unwrap().to_string();

            // let tag = video_info
            //     .get("ownerBadges").unwrap()
            //     .get(0).unwrap()
            //     .get("metadataBadgeRenderer").unwrap()
            //     .get("accessibilityData").unwrap()
            //     .get("icon").unwrap()
            //     .get("iconType").unwrap()
            //     .as_str().unwrap().to_string();

            // let meta_description = video_info
            //     .get("detailedMetadataSnippets").unwrap()
            //     .get(0).unwrap()
            //     .get("snippetText").unwrap()
            //     .get("runs").unwrap()
            //     .as_array().unwrap()
            //     .iter().map(|e| {e.get("text").unwrap().as_str().unwrap()})
            //     .collect::<Vec<&str>>()
            //     .join(",");

            // let date = video_info
            //     .get("publishedTimeText").unwrap()
            //     .get("simpleText").unwrap()
            //     .as_str().unwrap().to_string();
        } else {
            log::debug!("No videoid in {}", video_info);
            None
        }
    }

    fn get_search_videos(yt_initial_data: &Value) -> Result<&Vec<Value>, ErrorExtractor> {
        yt_initial_data
            .get("contents")
            .ok_or(ErrorExtractor::ErrorParsing(
                "Missing 'contents'".to_string(),
            ))?
            .get("twoColumnSearchResultsRenderer")
            .ok_or(ErrorExtractor::ErrorParsing(
                "Missing 'twoColumnSearchResultsRenderer'".to_string(),
            ))?
            .get("primaryContents")
            .ok_or(ErrorExtractor::ErrorParsing(
                "Missing 'primaryContents'".to_string(),
            ))?
            .get("sectionListRenderer")
            .ok_or(ErrorExtractor::ErrorParsing(
                "Missing 'sectionListRenderer'".to_string(),
            ))?
            .get("contents")
            .ok_or(ErrorExtractor::ErrorParsing(
                "Missing 'contents'".to_string(),
            ))?
            .get(0)
            .ok_or(ErrorExtractor::ErrorParsing("Cannot get(0)".to_string()))?
            .get("itemSectionRenderer")
            .ok_or(ErrorExtractor::ErrorParsing(
                "Missing 'itemSectionRenderer'".to_string(),
            ))?
            .get("contents")
            .ok_or(ErrorExtractor::ErrorParsing(
                "Missing 'contents'".to_string(),
            ))?
            .as_array()
            .ok_or(ErrorExtractor::ErrorParsing("No an array".to_string()))
    }
}

/// Information of a video
struct YtVideoPageData {
    yt_response_data: Value,
    _js_url: String, // TODO("No used")
    cipher_fun: Vec<CipherFunction>,
}

/// Information of a video page (including normal page info + video info)
pub struct YtVideoPage {
    yt_page_data: YtPageData,
    yt_video_data: YtVideoPageData,
}

impl YtVideoPage {
    pub async fn new(url: &str) -> Result<Self, ErrorExtractor> {
        let yt_page_data = YtPageData::new(url).await?;

        let yt_initial_player_response =
            get_var(&yt_page_data.webpage, "var ytInitialPlayerResponse")?;

        let js_url = get_js_url(&yt_page_data.webpage)?;
        let js_code = reqwest::get(&js_url)
            .await
            .map_err(|_| ErrorExtractor::ErrorReqwest)?
            .text()
            .await
            .map_err(|_| ErrorExtractor::ErrorReqwest)?;
        let cipher_fun = get_cipher_fun(&js_code);
        if cipher_fun.is_empty() {
            return Err(ErrorExtractor::ErrorParsing(format!(
                "Can't find Cipher code in {js_url}"
            )));
        }
        if cipher_fun.is_empty() {
            // See also : https://killerplayer.com/decode-cipher-signature-youtube/
            log::warn!("Cannot find Cipher function in {}", js_url);
        }

        Ok(Self {
            yt_page_data,
            yt_video_data: YtVideoPageData {
                yt_response_data: yt_initial_player_response,
                _js_url: js_url,
                cipher_fun,
            },
        })
    }

    pub async fn get_best_audio(&self) -> Result<YtAudioData, ErrorExtractor> {
        let audios = self.audio_urls();
        let best_audio = audios
            .into_iter()
            .max_by(|e1, e2| e1.bitrate.cmp(&e2.bitrate))
            .ok_or(ErrorExtractor::ErrorParsing(format!(
                "No audio found at : {}",
                self.yt_page_data.url
            )))?;

        if let Ok(r) = reqwest::get(&best_audio.url).await {
            if r.status() != StatusCode::from_u16(200).unwrap() {
                return Err(ErrorExtractor::ErrorParsing(format!(
                    "{} status code : {}",
                    best_audio.url,
                    r.status()
                )));
            }
        }

        Ok(best_audio)
    }

    fn get_url(&self, v: &Value) -> Result<YtAudioData, ErrorExtractor> {
        match v.get("url") {
            None => match v.get("signatureCipher") {
                None => panic!("No signatureCipher nor url in Value : {v}"),
                Some(v_sc) => match v_sc.as_str() {
                    None => panic!("Error converting signatureCipher to &str"),
                    Some(sc) => {
                        let s = sc
                            .split('&')
                            .map(|e| {
                                let tmp = e.split('=').collect::<Vec<&str>>();
                                (tmp[0], tmp[1])
                            })
                            .collect::<HashMap<&str, &str>>();
                        let tmp_url = match s.get("url") {
                            None => panic!("No url in signatureCipher"),
                            Some(url) => url.to_string(),
                        };
                        let mut tmp_sig = match s.get("s") {
                            None => panic!("No s ('sig') in signatureCipher"),
                            Some(&sig) => urlencoding::decode(sig)
                                .map_err(|_| {
                                    ErrorExtractor::ErrorParsing(
                                        "Error url decoding signature_Cipher".to_string(),
                                    )
                                })?
                                .into_owned(),
                        };
                        // log::debug!("Before sig : {}", tmp_sig);
                        for a in self.yt_video_data.cipher_fun.iter() {
                            match a {
                                CipherFunction::Swap(i) => {
                                    // log::debug!("Swap {}", i);
                                    swap(&mut tmp_sig, *i)
                                }
                                CipherFunction::Slice(i) => {
                                    // log::debug!("Slice : {}", i);
                                    slice(&mut tmp_sig, *i)
                                }
                                CipherFunction::Reverse => {
                                    // log::debug!("Reverse");
                                    reverse(&mut tmp_sig)
                                }
                            }
                        }
                        // log::debug!("After sig : {}", tmp_sig);

                        let resu = tmp_url + "&sig=" + &*tmp_sig;
                        let resu = match urlencoding::decode(&resu) {
                            Ok(u) => u.into_owned(),
                            Err(why) => {
                                log::warn!("Error decoding (url sig) {} : {}", resu, why);
                                resu
                            }
                        };

                        YtAudioData::new(resu, v)
                    }
                },
            },
            Some(v_url) => match v_url.as_str() {
                None => panic!("Error converting url to &str"),
                Some(url) => {
                    let resu = match urlencoding::decode(url) {
                        Ok(u) => u.into_owned(),
                        Err(why) => {
                            log::warn!("Error decoding {} : {}", url, why);
                            url.to_string()
                        }
                    };

                    YtAudioData::new(resu, v)
                }
            },
        }
    }

    fn get_urls(&self, arr: &[Value]) -> Vec<YtAudioData> {
        let mut urls = Vec::new();
        for v in arr.iter() {
            match v.get("itag") {
                None => log::warn!("No itag in value : {}", v),
                Some(v_itag) => {
                    match v_itag.as_u64() {
                        None => log::warn!("Error converting itag to u64 : {}", v_itag),
                        Some(itag) => {
                            // See : https://gist.github.com/sidneys/7095afe4da4ae58694d128b1034e01e2
                            // or : https://tyrrrz.me/blog/reverse-engineering-youtube/
                            // m4a  : 139 | 140 | 141 | 256 | 258 | 325 | 328
                            // webm : 171 | 172 | 249 | 250 | 251
                            if AUDIO_ITAGS.contains(&(itag as u32)) {
                                match self.get_url(v) {
                                    Err(why) => log::warn!("{}", why),
                                    Ok(yt_audio_data) => urls.push(yt_audio_data),
                                }
                            }
                        }
                    }
                }
            }
        }
        urls
    }

    pub fn audio_urls(&self) -> Vec<YtAudioData> {
        if !self
            .yt_page_data
            .url
            .contains("https://www.youtube.com/watch")
        {
            panic!("{} is not a video page url", self.yt_page_data.url);
        }
        match self.yt_video_data.yt_response_data.get("streamingData") {
            None => {
                log::warn!(
                    "No streamingData found in : {}",
                    self.yt_video_data.yt_response_data
                );
                Vec::new()
            }
            Some(sd) => {
                let mut v1 = match sd.get("adaptiveFormats") {
                    None => {
                        log::info!("No adaptiveFormats in streamingData");
                        Vec::new()
                    }
                    Some(af) => match af.as_array() {
                        None => {
                            log::warn!("Error converting adaptiveFormats to array");
                            Vec::new()
                        }
                        Some(arr) => self.get_urls(arr),
                    },
                };
                let mut v2 = match sd.get("formats") {
                    None => {
                        log::info!("No formats in streamingData");
                        Vec::new()
                    }
                    Some(f) => match f.as_array() {
                        None => {
                            log::warn!("Error converting formats to array");
                            Vec::new()
                        }
                        Some(arr) => self.get_urls(arr),
                    },
                };
                v1.append(&mut v2);
                v1
            }
        }
    }
}

async fn get_webpage(url: &str) -> Result<String, ErrorExtractor> {
    reqwest::get(url)
        .await
        .map_err(|_| ErrorExtractor::ErrorReqwest)?
        .text()
        .await
        .map_err(|_| ErrorExtractor::ErrorReqwest)
}

fn get_var(webpage: &str, var: &str) -> Result<Value, ErrorExtractor> {
    match webpage.find(var) {
        Some(i) => {
            // TODO("May improve this")
            match &webpage[i..].find("</script>") {
                Some(f) => {
                    let var_str = &webpage[i + var.len() + 3..i + f - 1]; // +3 : " = " | -1 : " "
                    match serde_json::from_str(var_str) {
                        Ok(v) => Ok(v),
                        Err(why) => Err(ErrorExtractor::ErrorParsing(format!(
                            "Error parsing {var} to JSON : {why}"
                        ))),
                    }
                }
                None => Err(ErrorExtractor::ErrorParsing(format!(
                    "No </script> found after {var}"
                ))),
            }
        }
        None => Err(ErrorExtractor::ErrorParsing(format!(
            "No {var} found in webpage"
        ))),
    }
}

fn get_js_url(webpage: &str) -> Result<String, ErrorExtractor> {
    match webpage.find("jsUrl") {
        None => Err(ErrorExtractor::ErrorParsing("No jsUrl found".to_string())),
        Some(i) => match webpage[i + 8..].find('"') {
            None => Err(ErrorExtractor::ErrorParsing(
                "No \" found after jsUrl".to_string(),
            )),
            Some(f) => Ok("https://www.youtube.com".to_owned() + &webpage[i + 8..i + f + 8]),
        },
    }
}

#[derive(Debug)]
enum CipherFunction {
    Swap(usize),
    Slice(usize),
    Reverse,
}

fn reverse(s: &mut String) {
    *s = s.chars().rev().collect();
}

fn slice(s: &mut String, i: usize) {
    *s = s.chars().skip(i).collect();
}

/// Swap a char in a String with the first caracter
fn swap(s: &mut String, i: usize) {
    let mut swapped = String::with_capacity(s.len());
    let (first, second) = s.split_at(i);

    let mut c1 = first.chars();
    let f1 = c1.next().unwrap();
    let mut c2 = second.chars();
    let f2 = c2.next().unwrap();

    swapped.push(f2);
    swapped.push_str(&c1.collect::<String>());
    swapped.push(f1);
    swapped.push_str(&c2.collect::<String>());

    *s = swapped;
}

fn get_cipher_fun(js_code: &str) -> Vec<CipherFunction> {
    // TODO("improve this")
    let re1 = regex::Regex::new(
        r#"\b([a-zA-Z0-9$]+)\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#,
    )
    .unwrap();
    let m1 = re1.find(js_code).unwrap();
    let re2 = regex::Regex::new(r#";\s*[a-zA-Z]+."#).unwrap();
    let i1 = m1.end();
    let m2 = re2.find(&js_code[i1..]).unwrap();
    let var_name = m2.as_str();
    let var_name = &var_name[1..m2.end() - 1];
    let re3 = regex::Regex::new(format!(r#"var \s*{var_name}\s*="#).as_str()).unwrap();
    let m3 = re3.find(js_code).unwrap();

    let re4 =
        regex::Regex::new(r#"\{[^\{\}]*\{[^\{\}]*\}[^\{\}]*\{[^\{\}]*\}[^\{\}]*\{[^\{\}]*\}\}"#)
            .unwrap();

    let m4 = re4.find(&js_code[m3.end()..]).unwrap();
    let f = m4.as_str()[1..].split('\n').collect::<Vec<&str>>();
    let re5 = regex::Regex::new(r#"\s*[a-zA-Z0-9]+:"#).unwrap();

    let m_name1 = re5.find(f[0]).unwrap();
    let name1 = &m_name1.as_str()[..m_name1.end() - 1];

    let m_name2 = re5.find(f[1]).unwrap();
    let name2 = &m_name2.as_str()[..m_name2.end() - 1];

    let m_name3 = re5.find(f[2]).unwrap();
    let name3 = &m_name3.as_str()[..m_name3.end() - 1];

    let reverse_name = if f[0].contains("reverse") {
        name1
    } else if f[1].contains("reverse") {
        name2
    } else if f[2].contains("reverse") {
        name3
    } else {
        panic!("Can't find reverse function in js_code");
    };

    let slice_name = if f[0].contains("splice") {
        name1
    } else if f[1].contains("splice") {
        name2
    } else if f[2].contains("splice") {
        name3
    } else {
        panic!("Can't find splice function in js_code");
    };

    let swap_name = if !f[0].contains("splice") && !f[0].contains("reverse") {
        name1
    } else if !f[1].contains("splice") && !f[1].contains("reverse") {
        name2
    } else if !f[2].contains("splice") && !f[2].contains("reverse") {
        name3
    } else {
        panic!("Can't find swap function in js_code");
    };

    if swap_name == slice_name || slice_name == reverse_name || swap_name == reverse_name {
        panic!("Error finding correct cipher function in js_code")
    }

    let i = m1.end();
    let f = js_code[i..].find('}').unwrap();

    let mut cipher_fun = Vec::new();
    for a in js_code[i + 1..i + f].split(';') {
        if let Some(s) = a.find(slice_name) {
            let m = a[s + 5..].find(')').unwrap();
            let n = &a[s + 5..s + m + 5];
            cipher_fun.push(CipherFunction::Slice(n.parse().unwrap()));
        } else if a.contains(reverse_name) {
            cipher_fun.push(CipherFunction::Reverse);
        } else if let Some(s) = a.find(swap_name) {
            let m = a[s + 5..].find(')').unwrap();
            let n = &a[s + 5..s + m + 5];
            cipher_fun.push(CipherFunction::Swap(n.parse().unwrap()));
        }
    }
    cipher_fun
}
