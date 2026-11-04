# recipe-scraper

[![GitHub](https://img.shields.io/crates/l/recipe-scraper)](https://github.com/torrancew/recipe-scraper)
[![crates.io](https://img.shields.io/crates/d/recipe-scraper)](https://crates.io/crates/recipe-scraper)
[![docs.rs](https://docs.rs/recipe-scraper/badge.svg)](https://docs.rs/recipe-scraper)

`recipe-scraper` is a Rust library for scraping structured recipe data from the web

It provides a simple set of APIs to find and parse compliant recipe formats

## Support

`recipe-scraper` is fairly pragmatic, and extracts minimal data from recipes.
The normalized API returns the title, description, ingredients, instructions and extraction method.

And supports the following structured recipe formats:

- [schema.org Recipe (in JSON-LD form)](https://schema.org/Recipe) -- Tested against a fairly wide selection of sites. Not perfect but pretty effective in the face of various real-world edge cases.
- schema.org Recipe in Microdata form, through `extract_recipes`

`recipe-scraper` provides methods that operate on HTML strings, as well as directly on JSON. It explicitly does **not** provide HTTP client functionality.

## Usage & Examples

Scraping a recipe from an HTML string (which can be obtained via `reqwest` or `ureq`, etc):

```rust
use recipe_scraper::{Extract, SchemaOrgEntry, Scrape};

let schema_entries = SchemaOrgEntry::scrape_html(&html);
if let Some(first_valid_recipe) = schema_entries.iter().flat_map(Extract::extract_recipes).next() {
  println!("Found recipe!: {first_valid_recipe:?}")
}
```

For normalized text, `extract_recipes` tries JSON-LD first and Microdata if no complete JSON-LD recipe is found.
Complete means a nonempty title, ingredient list and instructions.

```rust
use recipe_scraper::extract_recipes;

for recipe in extract_recipes(&html) {
  println!("{}: {:?}", recipe.title, recipe.ingredients);
}
```

JSON-LD parsing also handles nested recipes, local references and mixed instruction steps and sections.
Instruction section headings are kept and blank ingredient lines are removed.

`extract_recipes_with_url(&html, url)` tries the same metadata parser, then a website fallback if no complete recipe was found.
Fallbacks currently support RecipeTin Eats and Bio-Mayer and live in separate website modules. This is modularly expandable.

On 60 saved recipe pages, generic Rust extraction and the schema-only parser from Python's [recipe-scrapers 15.11.0](https://github.com/hhursev/recipe-scrapers/tree/15.11.0) (which was used as a reference) each returned 58 complete recipes.
The Rust website fallbacks brought that count to 60.
