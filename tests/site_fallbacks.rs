use recipe_scraper::{extract_recipes, extract_recipes_with_url, ExtractionMethod};

const CARD: &str = r#"<div class="wprm-recipe-container">
<h2 class="wprm-recipe-name">Toast</h2>
<div class="wprm-recipe-ingredient">1 slice of bread</div>
<div class="wprm-recipe-instructions-container"><h3>Instructions</h3><p>Toast the bread.</p></div>
</div>"#;

const HEADINGS: &str = r#"<main><h1>Rezept: Suppe</h1>
<h2>Zutaten</h2><p>Salz</p><h2>Zubereitung</h2><p>Umrühren.</p></main>"#;

#[test]
fn generic_ignores_card_classes() {
    assert!(extract_recipes(CARD).is_empty());
}

#[test]
fn generic_ignores_headings() {
    assert!(extract_recipes(HEADINGS).is_empty());
}

#[test]
fn recipetineats_host_variants() {
    for url in [
        "https://recipetineats.com/toast/",
        "http://www.recipetineats.com/toast/",
        "HTTPS://WWW.RECIPETINEATS.COM:443/toast/?print=1#recipe",
        "https://reader:password@recipetineats.com:8443/toast/",
    ] {
        let recipes = extract_recipes_with_url(CARD, url);
        assert_eq!(recipes.len(), 1, "{url}");
        assert_eq!(recipes[0].method, ExtractionMethod::RecipeCard);
        assert_eq!(recipes[0].title, "Toast");
        assert_eq!(recipes[0].ingredients, ["1 slice of bread"]);
        assert_eq!(recipes[0].instructions, ["Toast the bread."]);
    }
}

#[test]
fn bio_mayer_host_variants() {
    for url in [
        "https://bio-mayer.de/suppe/",
        "https://www.bio-mayer.de:443/suppe/",
        "http://reader@BIO-MAYER.DE/suppe/",
    ] {
        let recipes = extract_recipes_with_url(HEADINGS, url);
        assert_eq!(recipes.len(), 1, "{url}");
        assert_eq!(recipes[0].method, ExtractionMethod::Headings);
        assert_eq!(recipes[0].title, "Suppe");
        assert_eq!(recipes[0].ingredients, ["Salz"]);
        assert_eq!(recipes[0].instructions, ["Umrühren."]);
    }
}

#[test]
fn unknown_hosts_no_fallback() {
    for url in [
        "https://example.com/",
        "https://recipetineats.com.example.com/",
        "https://example-recipetineats.com/",
        "https://other.recipetineats.com/",
        "https://recipetineats.com@example.com/",
        "https://bio-mayer.de.example.com/",
        "https://other.bio-mayer.de/",
        "https://bio-mayer.de@example.com/",
        "https://recipetineats.com:invalid/",
        "https://bio-mayer.de:65536/",
        "https://bio-mayer.de:/",
        "https://bio-mayer.de /",
        "ftp://recipetineats.com/",
        "recipetineats.com/",
        "",
    ] {
        assert!(extract_recipes_with_url(CARD, url).is_empty(), "{url}");
        assert!(extract_recipes_with_url(HEADINGS, url).is_empty(), "{url}");
    }
}

#[test]
fn fallbacks_stay_site_specific() {
    assert!(extract_recipes_with_url(CARD, "https://bio-mayer.de/").is_empty());
    assert!(extract_recipes_with_url(HEADINGS, "https://recipetineats.com/").is_empty());
}

#[test]
fn metadata_before_site_fallback() {
    let html = format!(
        r#"<script type="application/ld+json">{{"@type":"Recipe","name":"Metadata soup","recipeIngredient":["salt"],"recipeInstructions":"Stir."}}</script>{CARD}{HEADINGS}"#
    );
    for url in [
        "https://recipetineats.com/",
        "https://bio-mayer.de/",
        "invalid URL",
    ] {
        let recipes = extract_recipes_with_url(&html, url);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].title, "Metadata soup");
        assert_eq!(recipes[0].method, ExtractionMethod::JsonLd);
    }
}

#[test]
fn incomplete_metadata_uses_fallback() {
    let html = format!(
        r#"<script type="application/ld+json">{{"@type":"Recipe","name":"Incomplete","recipeIngredient":["salt"]}}</script>{CARD}"#
    );
    let recipes = extract_recipes_with_url(html, "https://recipetineats.com/");
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Toast");
    assert_eq!(recipes[0].method, ExtractionMethod::RecipeCard);
}

#[test]
fn cards_keep_order() {
    let html = format!(
        "{CARD}<div class='wprm-recipe-container'><h2 class='wprm-recipe-name'>Soup</h2><p class='wprm-recipe-ingredient'>salt</p><div class='wprm-recipe-instructions-container'><p>Stir.</p></div></div>"
    );
    let recipes = extract_recipes_with_url(html, "https://recipetineats.com/");
    assert_eq!(recipes.len(), 2);
    assert_eq!(recipes[0].title, "Toast");
    assert_eq!(recipes[0].ingredients, ["1 slice of bread"]);
    assert_eq!(recipes[1].title, "Soup");
    assert_eq!(recipes[1].ingredients, ["salt"]);
    assert_eq!(recipes[1].instructions, ["Stir."]);
}

#[test]
fn card_wrappers_no_duplicates() {
    let html = format!("<div class='wprm-recipe-container'>{CARD}</div>");
    let recipes = extract_recipes_with_url(html, "https://recipetineats.com/");
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Toast");
    assert_eq!(recipes[0].ingredients, ["1 slice of bread"]);
}

#[test]
fn nested_complete_cards() {
    let html = format!(
        r#"<div class="wprm-recipe-container"><h2 class="wprm-recipe-name">Soup</h2>
        <p class="wprm-recipe-ingredient">salt</p>
        <div class="wprm-recipe-instructions-container"><p>Stir.</p></div>{CARD}</div>"#
    );
    let recipes = extract_recipes_with_url(html, "https://recipetineats.com/");
    assert_eq!(recipes.len(), 2);
    assert_eq!(recipes[0].title, "Soup");
    assert_eq!(recipes[0].ingredients, ["salt"]);
    assert_eq!(recipes[0].instructions, ["Stir."]);
    assert_eq!(recipes[1].title, "Toast");
    assert_eq!(recipes[1].ingredients, ["1 slice of bread"]);
    assert_eq!(recipes[1].instructions, ["Toast the bread."]);
}

#[test]
fn nested_cards_cannot_fill_outer() {
    let html = r#"<div class="wprm-recipe-container"><h2 class="wprm-recipe-name">Outer</h2>
    <p class="wprm-recipe-ingredient">salt</p><div class="wprm-recipe-container">
    <div class="wprm-recipe-instructions-container"><p>Unrelated inner step.</p></div></div></div>"#;
    assert!(extract_recipes_with_url(html, "https://recipetineats.com/").is_empty());
}

#[test]
fn nested_card_text_stays_scoped() {
    let html = r#"<div class="wprm-recipe-container"><h2 class="wprm-recipe-name">Outer</h2>
    <p class="wprm-recipe-ingredient">salt</p><div class="wprm-recipe-instructions-container">
    <div class="wprm-recipe-container"><h2 class="wprm-recipe-name">Inner</h2>
    <p class="wprm-recipe-ingredient">flour</p></div></div></div>"#;
    assert!(extract_recipes_with_url(html, "https://recipetineats.com/").is_empty());
}

#[test]
fn nested_articles_stay_scoped() {
    let html = r#"<article><h1>Rezept: Suppe</h1><h2>Zutaten</h2><p>Salz</p>
    <article><h1>Rezept: Brot</h1><h2>Zubereitung</h2><p>Backen.</p></article></article>"#;
    assert!(extract_recipes_with_url(html, "https://bio-mayer.de/").is_empty());
}

#[test]
fn bio_mayer_separate_articles() {
    let html = r#"<main><article><h1>Rezept: Suppe</h1><h2>Zutaten</h2><p>Salz</p>
    <h2>Zubereitung</h2><p>Umrühren.</p></article>
    <article><h1>Rezept: Brot</h1><h2>Zutaten</h2><p>Mehl</p>
    <h2>Zubereitung</h2><p>Backen.</p></article></main>"#;
    let recipes = extract_recipes_with_url(html, "https://bio-mayer.de/");
    assert_eq!(recipes.len(), 2);
    assert_eq!(recipes[0].title, "Suppe");
    assert_eq!(recipes[0].ingredients, ["Salz"]);
    assert_eq!(recipes[1].title, "Brot");
    assert_eq!(recipes[1].instructions, ["Backen."]);
}
