use super::super::{
    complete_recipe, element_lines_excluding, is_visible, outermost_elements, selector,
    trim_ingredient_marker, ExtractedRecipe, ExtractionMethod,
};
use scraper::{ElementRef, Html};

pub(super) fn extract(document: &Html) -> Vec<ExtractedRecipe> {
    document
        .select(&selector(".wprm-recipe-container"))
        .filter_map(recipe_card)
        .collect()
}

fn belongs_to_card(scope: ElementRef<'_>, element: ElementRef<'_>) -> bool {
    !element
        .ancestors()
        .take_while(|ancestor| ancestor.id() != scope.id())
        .filter_map(ElementRef::wrap)
        .any(|ancestor| {
            ancestor
                .value()
                .classes()
                .any(|class| class == "wprm-recipe-container")
        })
}

fn card_text(scope: ElementRef<'_>, query: &str) -> String {
    scope
        .select(&selector(query))
        .find(|element| is_visible(*element) && belongs_to_card(scope, *element))
        .map(|element| card_lines(element).join(" "))
        .unwrap_or_default()
}

fn card_lines(element: ElementRef<'_>) -> Vec<String> {
    element_lines_excluding(element, Some(&selector(".wprm-recipe-container")))
}

fn recipe_card(scope: ElementRef<'_>) -> Option<ExtractedRecipe> {
    if !is_visible(scope) {
        return None;
    }
    let ingredients = scope
        .select(&selector(".wprm-recipe-ingredient"))
        .filter(|element| is_visible(*element) && belongs_to_card(scope, *element))
        .collect();
    let ingredients = outermost_elements(ingredients)
        .into_iter()
        .map(|element| trim_ingredient_marker(&card_lines(element).join(" ")))
        .filter(|line| !line.is_empty())
        .collect();
    let containers = scope
        .select(&selector(".wprm-recipe-instructions-container"))
        .filter(|element| is_visible(*element) && belongs_to_card(scope, *element))
        .collect();
    let mut instructions: Vec<_> = outermost_elements(containers)
        .into_iter()
        .flat_map(card_lines)
        .collect();
    if instructions
        .first()
        .map_or(false, |line| line.eq_ignore_ascii_case("instructions"))
    {
        instructions.remove(0);
    }
    complete_recipe(ExtractedRecipe {
        title: card_text(scope, ".wprm-recipe-name"),
        description: card_text(scope, ".wprm-recipe-summary"),
        ingredients,
        instructions,
        method: ExtractionMethod::RecipeCard,
    })
}
