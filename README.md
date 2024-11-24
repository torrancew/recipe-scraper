# recipe-scraper

[![GitHub](https://img.shields.io/crates/l/recipe-scraper)](https://github.com/torrancew/recipe-scraper)
[![crates.io](https://img.shields.io/crates/d/recipe-scraper)](https://crates.io/crates/recipe-scraper)
[![docs.rs](https://docs.rs/recipe-scraper/badge.svg)](https://docs.rs/recipe-scraper)

`recipe-scraper` is a Rust library for scraping structured recipe data from the web

It provides a simple set of APIs to find and parse compliant recipe formats

## Support

`recipe-scraper` is fairly pragmatic, and extracts minimal data from recipes. It currently extracts the following (meta-)data:

And supports the following structured recipe formats:

- [schema.org Recipe (in JSON-LD form)](https://schema.org/Recipe) -- Tested against a fairly wide selection of sites. Not perfect but pretty effective in the face of various real-world edge cases.

`recipe-scraper` provides methods that operate on HTML strings, as well as directly on JSON. It explicitly does **not** provide HTTP client functionality.

## Usage & Examples

Scraping a recipe from an HTML string (which can be obtained via `reqwest` or `ureq`, etc):

```rust
use recipe_scraper::{
  Extract,
  Scrape,
  SchemaOrgEntry,
  SchemaOrgRecipe,
};
// let html = ...;
let schema_entries = SchemaOrgEntry::scrape_html(&html);
if let Some(first_valid_recipe) = schema_entries.into_iter().flat_map(Extract::extract_recipes).next() {
  println!("Found recipe!: {first_valid_recipe:?}")
}
```
