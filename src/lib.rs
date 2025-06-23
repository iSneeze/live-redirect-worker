use scraper::{Html, Selector};
use url::Url as StandardUrl;
use worker::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[event(fetch)]
pub async fn main(req: Request, _env: Env, _ctx: worker::Context) -> Result<Response> {
    let channel_id = req
        .url()?
        .query_pairs()
        .find_map(|(key, value)| {
            if key == "channelID" {
                Some(value.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| Error::from("Bad Request: Missing 'channelID' query parameter"))?;

    let live_chat_url = get_live_chat_url(&channel_id).await.map_err(|e| {
        console_error!("Scraping failed: {:?}", e);
        Error::from("Could not find a live or upcoming stream for this channel.")
    })?;

    console_log!("Redirecting to: {}", &live_chat_url);
    Response::redirect(StandardUrl::parse(&live_chat_url)?)
}

async fn get_live_chat_url(channel_id: &str) -> anyhow::Result<String> {
    let channel_live_url = format!("https://www.youtube.com/channel/{}/live", channel_id);
    console_log!("Visiting: {}", channel_live_url);

    let client = reqwest::Client::new();
    let body = client.get(&channel_live_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .await?
        .text()
        .await?;

    let document = Html::parse_document(&body);
    let selector = Selector::parse(r#"link[rel="canonical"]"#).unwrap();
    let canonical_href = document
        .select(&selector)
        .next()
        .and_then(|el| el.value().attr("href"))
        .ok_or_else(|| {
            console_log!("HTML: {}", document.html());
            anyhow::anyhow!("ScrapeError: Canonical link tag not found")
        })?;

    console_log!("canonical url: {}", canonical_href);
    let canonical_url = StandardUrl::parse(canonical_href)?;
    let video_id = canonical_url
        .query_pairs()
        .find_map(|(key, value)| if key == "v" { Some(value) } else { None })
        .ok_or_else(|| anyhow::anyhow!("ParseError: Video ID not found in canonical URL"))?;

    let live_chat_url = format!(
        "https://www.youtube.com/live_chat?is_popout=1&v={}",
        video_id
    );
    Ok(live_chat_url)
}
