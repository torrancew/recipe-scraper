use recipe_scraper::{extract_recipes, ExtractionMethod};
use serde_json::{json, Value};

fn script(value: Value) -> String {
    format!(r#"<script type="application/ld+json">{value}</script>"#)
}

#[test]
fn recipe_in_main_entity() {
    let recipes = extract_recipes(script(json!({
        "@type": "WebPage",
        "mainEntity": {
            "@type": ["Thing", "https://schema.org/Recipe"],
            "name": "スープ", "recipeIngredient": "塩", "recipeInstructions": "混ぜる"
        }
    })));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "スープ");
    assert_eq!(recipes[0].ingredients, ["塩"]);
    assert_eq!(recipes[0].instructions, ["混ぜる"]);
    assert_eq!(recipes[0].method, ExtractionMethod::JsonLd);
}

#[test]
fn graph_refs_across_scripts() {
    let html = script(json!({
        "@type": "WebPage", "mainEntity": {"@id": "#recipe"}
    })) + &script(json!({"@graph": [
        {"@id": "#recipe", "@type": "Recipe", "name": {"@id": "#title"},
         "recipeIngredient": [{"@id": "#salt"}], "recipeInstructions": {"@id": "#steps"}},
        {"@id": "#title", "@value": "Soup"},
        {"@id": "#salt", "@value": "salt"},
        {"@id": "#steps", "@type": "HowToSection", "name": "Cook",
         "itemListElement": [{"@id": "#step"}]},
        {"@id": "#step", "@type": "HowToStep", "text": "Stir."},
        {"@id": "#recipe"}
    ]}));
    let recipes = extract_recipes(html);
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup");
    assert_eq!(recipes[0].ingredients, ["salt"]);
    assert_eq!(recipes[0].instructions, ["Cook", "Stir."]);
}

#[test]
fn mixed_field_values() {
    let recipes = extract_recipes(script(json!({
        "@type": "Recipe", "name": [null, {"@value": "Soup"}],
        "description": {"unexpected": true}, "recipeYield": false,
        "cookTime": ["broken"], "prepTime": {"unexpected": true},
        "recipeIngredient": [null, "<p>▢ salt &amp; pepper</p>", 12, {"@value": "water"}],
        "recipeInstructions": [false, {"@type": "HowToStep", "text": [null, {"@value": "Mix."}]}, null]
    })));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup");
    assert_eq!(recipes[0].description, "");
    assert_eq!(recipes[0].ingredients, ["salt & pepper", "water"]);
    assert_eq!(recipes[0].instructions, ["Mix."]);
}

#[test]
fn invalid_optional_fields() {
    let recipes = extract_recipes(script(json!({
        "@type": "Recipe", "name": "Soup", "description": 12,
        "recipeYield": {"unexpected": true}, "totalTime": ["not a duration"],
        "recipeIngredient": ["salt"], "recipeInstructions": "Mix."
    })));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup");
    assert_eq!(recipes[0].description, "");
}

#[test]
fn nested_steps_keep_order() {
    let recipes = extract_recipes(script(json!({
        "@type": "Recipe", "name": "Soup", "recipeIngredient": ["salt"],
        "recipeInstructions": [
            {"@type": "HowToSection", "name": "Prepare", "itemListElement": [
                {"@type": "HowToStep", "name": "Step 1", "text": "Chop finely."},
                {"@type": "HowToSection", "name": "Sauce", "itemListElement": {
                    "@type": "ListItem", "item": {"@type": "HowToStep", "text": "Stir."}
                }}
            ]},
            {"@type": "HowToSection", "name": "Empty", "itemListElement": []},
            {"@type": "HowToStep", "text": "Serve."}
        ]
    })));
    assert_eq!(recipes.len(), 1);
    assert_eq!(
        recipes[0].instructions,
        [
            "Prepare",
            "Step 1",
            "Chop finely.",
            "Sauce",
            "Stir.",
            "Serve."
        ]
    );
}

#[test]
fn cyclic_refs_keep_valid_steps() {
    let recipes = extract_recipes(script(json!({"@graph": [
        {"@type": "Recipe", "name": "Soup", "recipeIngredient": ["salt"],
         "recipeInstructions": [{"@id": "#cycle"}, "Mix."]},
        {"@id": "#cycle", "@type": "HowToSection", "name": "Empty",
         "itemListElement": [{"@id": "#cycle"}]}
    ]})));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].instructions, ["Mix."]);
}

#[test]
fn step_text_with_empty_list() {
    let recipes = extract_recipes(script(json!({
        "@type": "Recipe", "name": "Soup", "recipeIngredient": ["salt"],
        "recipeInstructions": [
            {"@type": "HowToStep", "text": "Stir.", "itemListElement": []},
            {"@type": "HowToSection", "name": "Empty", "text": "Not a step.", "itemListElement": []}
        ]
    })));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].instructions, ["Stir."]);
}

#[test]
fn microdata_attrs_and_multi_props() {
    let recipes = extract_recipes(
        r#"<div class="arbitrary" itemscope itemtype="https://schema.org/Thing http://schema.org/Recipe">
        <meta itemprop="headline name" content="スープ">
        <meta itemprop="description" content="温かい &amp; 簡単">
        <data itemprop="recipeIngredient" value="▢ 塩">ignored</data>
        <meta itemprop="recipeIngredient" content="水">
        <meta itemprop="recipeInstructions" content="混ぜる">
        </div>"#,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "スープ");
    assert_eq!(recipes[0].description, "温かい & 簡単");
    assert_eq!(recipes[0].ingredients, ["塩", "水"]);
    assert_eq!(recipes[0].instructions, ["混ぜる"]);
    assert_eq!(
        serde_json::to_value(&recipes[0].method).unwrap(),
        "microdata"
    );
}

#[test]
fn microdata_itemref_no_duplicates() {
    let recipes = extract_recipes(
        r#"<div itemscope itemtype="https://schema.org/Recipe" itemref="ingredients steps ingredients">
        <h1 itemprop="name">Soup</h1>
        <div id="ingredients"><span itemprop="recipeIngredient">salt</span></div>
        </div>
        <div id="steps"><p itemprop="recipeInstructions">Mix.</p></div>"#,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].ingredients, ["salt"]);
    assert_eq!(recipes[0].instructions, ["Mix."]);
}

#[test]
fn microdata_scope_isolation() {
    let recipes = extract_recipes(
        r#"<div itemscope itemtype="https://schema.org/Recipe">
        <div itemprop="author" itemscope itemtype="https://schema.org/Person">
        <span itemprop="name">Wrong</span><span itemprop="recipeIngredient">wrong</span>
        <span itemprop="recipeInstructions">Wrong.</span></div>
        <h1 itemprop="name">Soup</h1><p itemprop="recipeIngredient">salt</p>
        <p itemprop="recipeInstructions">Mix.</p>
        <div itemprop="hasPart" itemscope itemtype="https://schema.org/Recipe">
        <h2 itemprop="name">Bread</h2><p itemprop="recipeIngredient">flour</p>
        <p itemprop="recipeInstructions">Bake.</p></div></div>"#,
    );
    assert_eq!(recipes.len(), 2);
    assert_eq!(recipes[0].title, "Soup");
    assert_eq!(recipes[0].ingredients, ["salt"]);
    assert_eq!(recipes[0].instructions, ["Mix."]);
    assert_eq!(recipes[1].title, "Bread");
    assert_eq!(recipes[1].ingredients, ["flour"]);
}

#[test]
fn microdata_section_names() {
    let recipes = extract_recipes(
        r#"<div itemscope itemtype="https://schema.org/Recipe">
        <meta itemprop="name" content="Soup"><meta itemprop="recipeIngredient" content="salt">
        <div itemprop="recipeInstructions" itemscope itemtype="https://schema.org/HowToSection" itemref="last-step">
        <meta itemprop="name" content="Cook">
        <div itemprop="itemListElement" itemscope itemtype="https://schema.org/HowToStep">
        <meta itemprop="name" content="Step 1"><meta itemprop="text" content="Mix.">
        <div itemprop="video" itemscope itemtype="https://schema.org/VideoObject"><span itemprop="name">Wrong</span></div>
        </div></div></div>
        <div id="last-step" itemprop="itemListElement" itemscope itemtype="https://schema.org/HowToStep">
        <meta itemprop="text" content="Serve."></div>"#,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(
        recipes[0].instructions,
        ["Cook", "Step 1", "Mix.", "Serve."]
    );
}

#[test]
fn json_ld_before_microdata() {
    let microdata = r#"<div itemscope itemtype="https://schema.org/Recipe">
        <meta itemprop="name" content="Microdata"><meta itemprop="recipeIngredient" content="salt">
        <meta itemprop="recipeInstructions" content="Mix."></div>"#;
    let recipes = extract_recipes(
        script(json!({
            "@type": "Recipe", "name": "JSON-LD", "recipeIngredient": ["water"], "recipeInstructions": "Boil."
        })) + microdata,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "JSON-LD");
    let recipes =
        extract_recipes(script(json!({"@type": "Recipe", "name": "Incomplete"})) + microdata);
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Microdata");
}

#[test]
fn microdata_needs_type_and_scope() {
    for html in [
        r#"<div itemtype="https://schema.org/Recipe"><h1 itemprop="name">Soup</h1><p itemprop="recipeIngredient">salt</p><p itemprop="recipeInstructions">Mix.</p></div>"#,
        r#"<div itemscope itemtype="https://example.org/Recipe"><h1 itemprop="name">Soup</h1><p itemprop="recipeIngredient">salt</p><p itemprop="recipeInstructions">Mix.</p></div>"#,
    ] {
        assert!(extract_recipes(html).is_empty());
    }
}

#[test]
fn hidden_microdata_properties() {
    let recipes = extract_recipes(
        r#"<div itemscope itemtype="https://schema.org/Recipe">
        <span hidden itemprop="name">Soup</span>
        <span hidden itemprop="recipeIngredient">salt</span>
        <div hidden itemprop="recipeInstructions"><p>Mix.</p><p>Serve.</p></div></div>"#,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup");
    assert_eq!(recipes[0].ingredients, ["salt"]);
    assert_eq!(recipes[0].instructions, ["Mix.", "Serve."]);
}

#[test]
fn hidden_microdata_keeps_paragraphs() {
    let recipes = extract_recipes(
        r#"<div hidden itemscope itemtype="https://schema.org/Recipe">
        <h1 itemprop="name"><span hidden>Soup</span></h1>
        <p itemprop="recipeIngredient"><span aria-hidden="true">salt</span></p>
        <div itemprop="recipeInstructions"><section hidden><p>Mix.</p><p>Serve.</p></section></div></div>"#,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup");
    assert_eq!(recipes[0].ingredients, ["salt"]);
    assert_eq!(recipes[0].instructions, ["Mix.", "Serve."]);
}

#[test]
fn microdata_text_decoded_once() {
    let recipes = extract_recipes(
        r#"<div itemscope itemtype="https://schema.org/Recipe">
        <h1 itemprop="name">Soup &amp;copy;</h1>
        <p itemprop="recipeIngredient">&lt;spice&gt;</p>
        <div itemprop="recipeInstructions"><p>Use &lt;spice&gt;.</p><p>Print &amp;lt;b&amp;gt;.</p></div></div>"#,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup &copy;");
    assert_eq!(recipes[0].ingredients, ["<spice>"]);
    assert_eq!(
        recipes[0].instructions,
        ["Use <spice>.", "Print &lt;b&gt;."]
    );
}

#[test]
fn microdata_attrs_decoded_once() {
    let recipes = extract_recipes(
        r#"<div itemscope itemtype="https://schema.org/Recipe">
        <meta itemprop="name" content="Soup &amp;lt;b&amp;gt;">
        <data itemprop="recipeIngredient" value="&lt;spice&gt;">ignored</data>
        <meta itemprop="recipeInstructions" content="Use &lt;spice&gt;, print &amp;lt;b&amp;gt;."></div>"#,
    );
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup &lt;b&gt;");
    assert_eq!(recipes[0].ingredients, ["<spice>"]);
    assert_eq!(recipes[0].instructions, ["Use <spice>, print &lt;b&gt;."]);
}

#[test]
fn same_id_keeps_recipe_fields() {
    let recipes = extract_recipes(script(json!({"@graph": [
        {"@id": "#recipe", "@type": "Recipe", "name": "Soup",
         "recipeIngredient": ["salt"], "recipeInstructions": "Mix."},
        {"@id": "#recipe", "@type": "Recipe", "url": "https://example.org/soup",
         "image": "image.jpg", "author": "Ann", "description": "Soup"}
    ]})));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].title, "Soup");
    assert_eq!(recipes[0].ingredients, ["salt"]);
    assert_eq!(recipes[0].instructions, ["Mix."]);
}

#[test]
fn refs_use_later_step_definition() {
    let recipes = extract_recipes(script(json!({"@graph": [
        {"@type": "Recipe", "name": "Soup", "recipeIngredient": ["salt"],
         "recipeInstructions": {"@id": "#step"}},
        {"@id": "#step", "@type": "HowToStep"},
        {"@id": "#step", "@type": "HowToStep", "text": "Mix."}
    ]})));
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].instructions, ["Mix."]);
}
