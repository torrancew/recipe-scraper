use super::super::{
    append_text, complete_recipe, element_lines, heading_level, is_visible, normalize_lines,
    selector, ExtractedRecipe, ExtractionMethod,
};
use scraper::{ElementRef, Html};

pub(super) fn extract(document: &Html) -> Vec<ExtractedRecipe> {
    let mut scopes: Vec<_> = document.select(&selector("article")).collect();
    if scopes.is_empty() {
        scopes = document.select(&selector("main")).collect();
    }
    if scopes.is_empty() {
        scopes = document.select(&selector("body")).collect();
    }
    scopes.into_iter().filter_map(heading_recipe).collect()
}

#[derive(Clone, Copy, PartialEq)]
enum SectionKind {
    Ingredients,
    Preparation,
}

fn heading_kind(element: ElementRef<'_>) -> Option<SectionKind> {
    let value = element_lines(element).join(" ").to_lowercase();
    let first = value
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(':');
    match first {
        "zutaten" => Some(SectionKind::Ingredients),
        "zubereitung" | "zubereitungsschritte" => Some(SectionKind::Preparation),
        _ => None,
    }
}

fn section_lines(heading: ElementRef<'_>, kind: SectionKind) -> Vec<String> {
    let mut output = String::new();
    let nested_scopes = selector("article, main");
    for sibling in heading.next_siblings().filter_map(ElementRef::wrap) {
        if is_visible(sibling)
            && append_text(
                sibling,
                &mut output,
                heading_level(heading),
                kind == SectionKind::Ingredients,
                Some(&nested_scopes),
            )
        {
            break;
        }
    }
    let mut result = normalize_lines(&output);
    if kind == SectionKind::Ingredients {
        result.retain(|line| {
            !line
                .split_whitespace()
                .next()
                .unwrap_or("")
                .eq_ignore_ascii_case("für")
        });
    }
    result
}

fn belongs_to_scope(scope: ElementRef<'_>, element: ElementRef<'_>) -> bool {
    !element
        .ancestors()
        .take_while(|ancestor| ancestor.id() != scope.id())
        .filter_map(ElementRef::wrap)
        .any(|ancestor| matches!(ancestor.value().name(), "article" | "main"))
}

fn heading_recipe(scope: ElementRef<'_>) -> Option<ExtractedRecipe> {
    if !is_visible(scope) {
        return None;
    }
    let headings: Vec<_> = scope
        .select(&selector("h2,h3,h4"))
        .filter(|element| is_visible(*element) && belongs_to_scope(scope, *element))
        .collect();
    let ingredients: Vec<_> = headings
        .iter()
        .copied()
        .filter(|heading| heading_kind(*heading) == Some(SectionKind::Ingredients))
        .collect();
    let preparation: Vec<_> = headings
        .iter()
        .copied()
        .filter(|heading| heading_kind(*heading) == Some(SectionKind::Preparation))
        .collect();
    // multiple heading pairs could belong to separate recipes or navigation
    if ingredients.len() != 1 || preparation.len() != 1 {
        return None;
    }
    let title = scope
        .select(&selector("h1"))
        .find(|element| is_visible(*element) && belongs_to_scope(scope, *element))
        .map(|element| element_lines(element).join(" "))
        .unwrap_or_default();
    let title = title
        .strip_prefix("Rezept:")
        .unwrap_or(&title)
        .trim()
        .to_string();
    complete_recipe(ExtractedRecipe {
        title,
        description: String::new(),
        ingredients: section_lines(ingredients[0], SectionKind::Ingredients),
        instructions: section_lines(preparation[0], SectionKind::Preparation),
        method: ExtractionMethod::Headings,
    })
}
