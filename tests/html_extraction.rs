use recipe_scraper::{extract_recipes, extract_recipes_with_url, ExtractionMethod};
use serde_json::{json, Value};

fn json_ld(instructions: Value, ingredients: Value) -> String {
    format!(
        r#"<script type="application/ld+json">{}</script>"#,
        json!({
            "@context":"https://schema.org", "@type":"Recipe", "name":"Soup",
            "description":"Lunch", "recipeIngredient":ingredients, "recipeInstructions":instructions
        })
    )
}

#[test]
fn section_headings() {
    let recipes = extract_recipes(json_ld(
        json!([
            {"name":"Prepare", "itemListElement":[{"text":"Chop."}]},
            {"name":"Cook", "itemListElement":[{"text":"Simmer."}]}
        ]),
        json!(["tomatoes"]),
    ));
    assert_eq!(recipes.len(), 1);
    assert_eq!(
        recipes[0].instructions,
        ["Prepare", "Chop.", "Cook", "Simmer."]
    );
}

#[test]
fn skip_blank_ingredients() {
    let recipes = extract_recipes(json_ld(
        json!("Mix."),
        json!(["  flour  ", "", " \n ", "salt"]),
    ));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].ingredients, ["flour", "salt"]);
}

#[test]
fn step_names_no_duplicates() {
    let recipes = extract_recipes(json_ld(
        json!([
            {"@type":"HowToStep", "name":"Paso 1", "text":"Mezclar."},
            {"@type":"HowToStep", "name":"Cocinar.", "text":"Cocinar."}
        ]),
        json!(["salt"]),
    ));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].instructions, ["Paso 1", "Mezclar.", "Cocinar."]);
}

#[test]
fn html_text_keeps_paragraphs() {
    let recipes = extract_recipes(json_ld(
        json!("<p>Mix <strong>gently</strong> &amp; slowly.</p><p>Bake.</p>"),
        json!(["salt &amp; pepper"]),
    ));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].ingredients, ["salt & pepper"]);
    assert_eq!(recipes[0].instructions, ["Mix gently & slowly.", "Bake."]);
}

#[test]
fn german_columns_and_breaks() {
    let recipes = extract_recipes_with_url(
        r#"<main><h1>Rezept: Gemüseauflauf</h1>
        <div><div><h2>Zutaten</h2><p>Für 4 Personen:<br>450g Kartoffeln<br>250g Kohl<br>Gewürze: Salz, Pfeffer</p></div>
        <div><h2>Zubereitung<br>Gemüseauflauf</h2><pre><p>Kartoffeln kochen.</p><p>Kohl anbraten.</p><p>Überbacken.</p></pre></div></div>
        <footer><h2>Kontakt</h2><p>Keine Kochanweisung.</p></footer></main>"#,
        "https://bio-mayer.de/gemueseauflauf/",
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].method, ExtractionMethod::Headings);
    assert_eq!(recipes[0].title, "Gemüseauflauf");
    assert_eq!(
        recipes[0].ingredients,
        ["450g Kartoffeln", "250g Kohl", "Gewürze: Salz, Pfeffer"]
    );
    assert_eq!(
        recipes[0].instructions,
        ["Kartoffeln kochen.", "Kohl anbraten.", "Überbacken."]
    );
}

#[test]
fn headings_stop_at_next_section() {
    let recipes = extract_recipes_with_url(
        r#"<article><h1>Soup</h1><h2>Zutaten</h2><ul><li>2 tomatoes</li><li>salt</li></ul>
        <h2>Zubereitung</h2><ol><li>Chop.</li><li>Simmer.</li></ol>
        <h2>Nährwerte</h2><p>Not a cooking step.</p></article>"#,
        "https://bio-mayer.de/suppe/",
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].ingredients, ["2 tomatoes", "salt"]);
    assert_eq!(recipes[0].instructions, ["Chop.", "Simmer."]);
}

#[test]
fn multi_lang_headers_no_generic_no_fallback() {
    for (ingredients, instructions) in [
        ("Ingredients", "Instructions"),
        ("Zutaten", "Zubereitung"),
        ("Ingredientes", "Elaboración"),
        ("Ingrediënten", "Bereiding"),
        ("Ingrédients", "Préparation"),
    ] {
        let html = format!("<article><h1>Recipe</h1><h2>{ingredients}</h2><ul><li>salt</li></ul><h2>{instructions}</h2><p>Mix.</p></article>");
        assert!(extract_recipes(&html).is_empty());
        assert!(extract_recipes_with_url(&html, "https://example.com/").is_empty());
    }
}

#[test]
fn card_content_stays_scoped() {
    let recipes = extract_recipes_with_url(
        r#"<html><body><h1>Blog title</h1><p>Unrelated text</p>
        <div class="wprm-recipe-container"><span itemprop="name">Author name</span><h2 class="wprm-recipe-name">Pasta</h2>
        <div class="wprm-recipe-ingredients-container"><div class="wprm-recipe-ingredient"><span>▢</span><span>2</span> <span>tbsp</span> oil</div></div>
        <div class="wprm-recipe-instructions-container"><h3>Instructions</h3><h4>Sauce</h4><p>Mix.</p><button>Start timer</button><p>Cook.</p></div></div>
        <p>Subscribe to the newsletter.</p></body></html>"#,
        "https://www.recipetineats.com/pasta/",
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].method, ExtractionMethod::RecipeCard);
    assert_eq!(recipes[0].title, "Pasta");
    assert_eq!(recipes[0].ingredients, ["2 tbsp oil"]);
    assert_eq!(recipes[0].instructions, ["Sauce", "Mix.", "Cook."]);
}

#[test]
fn metadata_before_fallbacks() {
    let html = json_ld(json!("Stir."), json!(["salt"])) +
        "<article><h1>Other</h1><h2>Zutaten</h2><p>flour</p><h2>Zubereitung</h2><p>Bake.</p></article>";
    let recipes = extract_recipes_with_url(html, "https://bio-mayer.de/");
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].method, ExtractionMethod::JsonLd);
    assert_eq!(recipes[0].title, "Soup");
}

#[test]
fn reject_incomplete_or_navigation() {
    for html in [
        "<article><h1>Soup</h1><h2>Zutaten</h2><p>salt</p></article>",
        "<main><h1>Index</h1><nav><h2>Zutaten</h2><p>salt</p><h2>Zubereitung</h2><p>Click here.</p></nav></main>",
        "<article><h1>Two recipes</h1><h2>Zutaten</h2><p>salt</p><h2>Zutaten</h2><p>flour</p><h2>Zubereitung</h2><p>Mix.</p></article>",
    ] {
        assert!(extract_recipes(html).is_empty());
        assert!(extract_recipes_with_url(html, "https://bio-mayer.de/").is_empty());
    }
}

#[test]
fn empty_sections_no_recipe() {
    let recipes = extract_recipes(json_ld(
        json!([{"name":"Cook", "itemListElement":[]}]),
        json!(["salt"]),
    ));
    assert!(recipes.is_empty());
}

#[test]
fn article_is_not_recipe() {
    let html = format!(
        r#"<script type="application/ld+json">{}</script>"#,
        json!({
            "@context":"https://schema.org", "@type":"Article", "name":"About recipes",
            "description":"A tutorial", "recipeIngredient":["salt"], "recipeInstructions":"Example text."
        })
    );
    assert!(extract_recipes(html).is_empty());
}

#[test]
fn overlapping_fields_no_duplicates() {
    let recipes = extract_recipes_with_url(
        r#"<div class="wprm-recipe-container"><h1 class="wprm-recipe-name">Soup</h1>
        <ul><li class="wprm-recipe-ingredient"><span class="wprm-recipe-ingredient" itemprop="recipeIngredient">salt</span></li></ul>
        <div class="wprm-recipe-instructions-container"><ol class="wprm-recipe-instructions-container"><li itemprop="recipeInstructions">Mix.</li><li itemprop="recipeInstructions">Bake.</li></ol></div></div>"#,
        "https://recipetineats.com/soup/",
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].ingredients, ["salt"]);
    assert_eq!(recipes[0].instructions, ["Mix.", "Bake."]);
}

#[test]
fn nested_cards_no_duplicates() {
    let recipes = extract_recipes(
        r#"<div class="recipe-card"><div itemscope itemtype="https://schema.org/Recipe">
        <h1 itemprop="name">Soup</h1><p itemprop="recipeIngredient">salt</p><p itemprop="recipeInstructions">Mix.</p>
        </div></div>"#,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup");
    assert_eq!(recipes[0].method, ExtractionMethod::Microdata);
}

#[test]
fn optional_description() {
    let html = format!(
        r#"<script type="application/ld+json">{}</script>"#,
        json!({
            "@type":"Recipe", "name":"Soup", "recipeIngredient":["salt"], "recipeInstructions":"Mix."
        })
    );
    let recipes = extract_recipes(html);
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].description, "");
}

#[test]
fn microdata_title_not_author() {
    let recipes = extract_recipes(
        r#"<div itemscope itemtype="https://schema.org/Recipe">
        <div itemprop="author" itemscope itemtype="https://schema.org/Person"><span itemprop="name">Ann</span></div>
        <h1 itemprop="name">Soup</h1><p itemprop="recipeIngredient">salt</p><p itemprop="recipeInstructions">Mix.</p></div>"#,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup");
}

#[test]
fn wrapped_sections_stop_at_heading() {
    let recipes = extract_recipes_with_url(
        r#"<article><h1>Soup</h1><h2>Zutaten</h2><p>salt</p>
        <h2>Zubereitung</h2><div><ol><li>Mix.</li><li>Bake.</li></ol><h2>Nährwerte</h2><p>Unrelated.</p></div></article>"#,
        "https://bio-mayer.de/suppe/",
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].instructions, ["Mix.", "Bake."]);
}

#[test]
fn skip_hidden_page_text() {
    let recipes = extract_recipes_with_url(
        r#"<article><h1>Soup</h1><h2>Zutaten</h2><p>salt</p>
        <p hidden>unrelated ingredient</p><h2>Zubereitung</h2><p>Mix.</p><p aria-hidden="true">Duplicate hidden step.</p></article>"#,
        "https://bio-mayer.de/suppe/",
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].ingredients, ["salt"]);
    assert_eq!(recipes[0].instructions, ["Mix."]);
}
