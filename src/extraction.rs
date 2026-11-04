use scraper::{node::Node, ElementRef, Html, Selector};
use serde::Serialize;

mod metadata;
mod sites;

/// how the recipe was found
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMethod {
    /// schema.org recipe data in JSON-LD
    JsonLd,
    /// schema.org recipe data in microdata
    Microdata,
    /// website recipe card classes
    RecipeCard,
    /// ingredient and preparation headings
    Headings,
}

/// recipe text with normalized whitespace
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExtractedRecipe {
    pub title: String,
    pub description: String,
    pub ingredients: Vec<String>,
    pub instructions: Vec<String>,
    pub method: ExtractionMethod,
}

/// extract recipes with a nonempty title, ingredients and instructions from JSON-LD or Microdata
///
/// does not fetch URLs or run JavaScript or evaluate stylesheets
///
/// ```
/// let html = r#"<script type="application/ld+json">{
///     "@type": "Recipe",
///     "name": "Toast",
///     "recipeIngredient": ["1 slice of bread"],
///     "recipeInstructions": ["Toast the bread."]
/// }</script>"#;
/// let recipes = recipe_scraper::extract_recipes(html);
/// assert_eq!(recipes[0].title, "Toast");
/// assert_eq!(recipes[0].instructions, ["Toast the bread."]);
/// ```
pub fn extract_recipes(html: impl AsRef<str>) -> Vec<ExtractedRecipe> {
    let document = Html::parse_document(html.as_ref());
    metadata::extract(&document)
}

/// try a registered website fallback only when metadata yields no complete recipe
pub fn extract_recipes_with_url(
    html: impl AsRef<str>,
    url: impl AsRef<str>,
) -> Vec<ExtractedRecipe> {
    let document = Html::parse_document(html.as_ref());
    let recipes = metadata::extract(&document);
    if !recipes.is_empty() {
        return recipes;
    }
    sites::extract(&document, url.as_ref())
}

// selecting both a container and its children would repeat their text
fn outermost_elements(elements: Vec<ElementRef<'_>>) -> Vec<ElementRef<'_>> {
    elements
        .iter()
        .copied()
        .filter(|element| {
            !element.ancestors().any(|ancestor| {
                elements
                    .iter()
                    .any(|candidate| candidate.id() == ancestor.id())
            })
        })
        .collect()
}

fn complete_recipe(recipe: ExtractedRecipe) -> Option<ExtractedRecipe> {
    if recipe.title.is_empty() || recipe.ingredients.is_empty() || recipe.instructions.is_empty() {
        None
    } else {
        Some(recipe)
    }
}

fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("static recipe selector")
}

fn is_non_content_tag(name: &str) -> bool {
    matches!(
        name,
        "script"
            | "style"
            | "noscript"
            | "template"
            | "svg"
            | "nav"
            | "footer"
            | "aside"
            | "button"
            | "input"
            | "form"
    )
}

fn is_visible(element: ElementRef<'_>) -> bool {
    !element
        .ancestors()
        .filter_map(ElementRef::wrap)
        .any(is_hidden)
        && !is_hidden(element)
}

fn is_hidden(element: ElementRef<'_>) -> bool {
    is_non_content_tag(element.value().name())
        || element.value().attr("hidden").is_some()
        || element
            .value()
            .attr("aria-hidden")
            .map_or(false, |value| value.eq_ignore_ascii_case("true"))
}

// propagate section boundaries through wrappers so later sections stay out of the result
fn append_text(
    element: ElementRef<'_>,
    output: &mut String,
    stop_at_heading: Option<u8>,
    skip_headings: bool,
    excluded: Option<&Selector>,
) -> bool {
    let name = element.value().name();
    if is_hidden(element) || excluded.map_or(false, |query| query.matches(&element)) {
        return false;
    }
    if let Some(level) = stop_at_heading {
        if heading_level(element).map_or(false, |rank| rank <= level) {
            return true;
        }
    }
    if skip_headings && heading_level(element).is_some() {
        return false;
    }
    // basic elements to watch out for
    let block = matches!(
        name,
        "br" | "p"
            | "div"
            | "li"
            | "ul"
            | "ol"
            | "pre"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "section"
            | "article"
    );
    if block {
        output.push('\n');
    }
    for child in element.children() {
        let reached_heading = match child.value() {
            Node::Text(value) => {
                output.push_str(value);
                false
            }
            Node::Element(_) => append_text(
                ElementRef::wrap(child).unwrap(),
                output,
                stop_at_heading,
                skip_headings,
                excluded,
            ),
            _ => false,
        };
        if reached_heading {
            return true;
        }
    }
    if block {
        output.push('\n');
    }
    false
}

fn element_lines(element: ElementRef<'_>) -> Vec<String> {
    element_lines_excluding(element, None)
}

fn element_lines_excluding(element: ElementRef<'_>, excluded: Option<&Selector>) -> Vec<String> {
    let mut output = String::new();
    append_text(element, &mut output, None, false, excluded);
    normalize_lines(&output)
}

fn normalize_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect()
}

fn html_lines(value: &str) -> Vec<String> {
    element_lines(Html::parse_fragment(value).root_element())
}

fn trim_ingredient_marker(value: &str) -> String {
    value
        .trim_start_matches(|c: char| {
            c.is_whitespace() || matches!(c, '▢' | '□' | '☐' | '✓' | '✔' | '•') //generic markers, might need expansion
        })
        .to_string()
}

fn heading_level(element: ElementRef<'_>) -> Option<u8> {
    match element.value().name() {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}
