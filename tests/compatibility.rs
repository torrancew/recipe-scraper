use recipe_scraper::{Extract, SchemaOrgEntry, Scrape};
use serde_json::json;

#[test]
fn graph_skips_unrelated_nodes() {
    let html = format!(
        r#"<script type="application/ld+json">{}</script>"#,
        json!({"@context":"https://schema.org", "@graph":[
            {"@type":"Recipe", "name":"First", "description":"Soup",
             "recipeIngredient":["salt"], "recipeInstructions":"Stir."},
            [], null, false, 42, "unrelated", {},
            {"@type":"Organization", "name":"Publisher"},
            {"@type":"Recipe", "name":"Incomplete"},
            {"@type":"Recipe", "name":"Second", "description":"Bread",
             "recipeIngredient":["flour"], "recipeInstructions":"Bake."}
        ]})
    );
    let recipes: Vec<_> = SchemaOrgEntry::scrape_html(html)
        .iter()
        .flat_map(Extract::extract_recipes)
        .collect();
    let names: Vec<_> = recipes.iter().map(|r| r.name().as_str()).collect();
    assert_eq!(names, ["First", "Second"]);
}

#[test]
fn mixed_steps_keep_order() {
    let entry = SchemaOrgEntry::from_json_value(json!({
        "@context":"https://schema.org", "@type":"Recipe", "name":"Salad", "description":"Lunch",
        "recipeIngredient":["lettuce"], "recipeInstructions":[
            {"@type":"HowToStep", "text":"Wash lettuce."},
            {"@type":"HowToSection", "name":"Dressing", "itemListElement":[
                {"@type":"HowToStep", "text":"Whisk."}, "Pour."
            ]},
            {"@type":"HowToStep", "text":"Serve."}
        ]
    }))
    .unwrap();
    let recipes = entry.extract_recipes();
    assert_eq!(recipes.len(), 1);
    let steps: Vec<_> = recipes[0]
        .directions()
        .as_ref()
        .unwrap()
        .directions()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        steps,
        ["Wash lettuce.", "Dressing\nWhisk.\nPour.", "Serve."]
    );
}

#[test]
fn unnamed_nested_sections() {
    let entry = SchemaOrgEntry::from_json_value(json!({
        "@context":"https://schema.org", "@type":"Recipe", "name":"Soup", "description":"Lunch",
        "recipeIngredient":["tomatoes"], "recipeInstructions":[
            "Chop.",
            {"@type":"HowToSection", "itemListElement":[
                "Simmer.", {"@type":"HowToSection", "name":"Finish", "itemListElement":["Season."]}
            ]},
            "Serve."
        ]
    }))
    .unwrap();
    let recipes = entry.extract_recipes();
    assert_eq!(recipes.len(), 1);
    let steps: Vec<_> = recipes[0]
        .directions()
        .as_ref()
        .unwrap()
        .directions()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(steps, ["Chop.", "Simmer.\nFinish\nSeason.", "Serve."]);
}

#[test]
fn section_steps_over_summary() {
    let entry = SchemaOrgEntry::from_json_value(json!({
        "@context":"https://schema.org", "@type":"Recipe", "name":"Soup", "description":"Lunch",
        "recipeIngredient":["tomatoes"], "recipeInstructions":[
            "Chop.",
            {"@type":"HowToSection", "name":"Cook", "text":"Summary only", "itemListElement":["Simmer."]}
        ]
    })).unwrap();
    let recipes = entry.extract_recipes();
    assert_eq!(recipes.len(), 1);
    let steps: Vec<_> = recipes[0]
        .directions()
        .as_ref()
        .unwrap()
        .directions()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(steps, ["Chop.", "Cook\nSimmer."]);
}
