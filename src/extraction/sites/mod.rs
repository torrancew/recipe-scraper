use super::ExtractedRecipe;
use scraper::Html;

mod bio_mayer;
mod recipetineats;

pub(super) fn extract(document: &Html, url: &str) -> Vec<ExtractedRecipe> {
    let host = match hostname(url) {
        Some(host) => host.to_ascii_lowercase(),
        None => return Vec::new(),
    };
    let host = host.strip_prefix("www.").unwrap_or(&host);
    match host {
        "recipetineats.com" => recipetineats::extract(document),
        "bio-mayer.de" => bio_mayer::extract(document),
        _ => Vec::new(),
    }
}

fn hostname(url: &str) -> Option<&str> {
    let (scheme, rest) = url.trim().split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let authority = rest.split(&['/', '?', '#'][..]).next()?;
    if authority
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return None;
    }
    let host_port = authority.rsplit('@').next()?;
    if let Some((host, port)) = host_port.rsplit_once(':') {
        if port.is_empty() || !port.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        port.parse::<u16>().ok()?;
        Some(host)
    } else {
        Some(host_port)
    }
}
